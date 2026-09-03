//! The live-mode Telegram forwarder: one queue, one thread, one group.
//!
//! # Why a thread of its own
//!
//! Every live worker holds its serial port open and sits in a 500 ms
//! `wait_notification` loop watching for `+CMTI`. An HTTP call from that loop
//! would block it for up to [`crate::telegram`]'s 15-second timeout, losing
//! notifications and dragging the 10-minute SIM sweep out of cadence. So the
//! workers only ever `deliver()` into a queue, which never blocks, and this
//! thread does all the talking.
//!
//! # Why the queue coalesces instead of only pacing
//!
//! Telegram allows ~20 messages per minute into a group. "One send every three
//! seconds" is exactly 20/min — the ceiling, not a budget — so a 64-stick bank
//! taking a burst builds a backlog that delivers codes after they have expired,
//! which is indistinguishable from the feature not working. The limit counts
//! *messages*, not codes, so once more than one item is waiting they are packed
//! into a single message ([`MAX_BATCH`] at a time) and throughput stops being
//! the constraint. Quiet periods still get one bubble per message.
//!
//! This is the same trade the toast column makes in `utils/toast-queue.ts`.
//!
//! # Editing rather than double-posting
//!
//! A concatenated SMS surfaces twice: the first fragment, then the completed
//! text superseding it in place (`commands/mod.rs`, the `Some` arm of the
//! partial-row lookup). Both arrive here under the same `SmsItem` id, so the
//! `sent` map turns the second one into an `editMessageText` on the bubble the
//! first one produced. Myanmar text fits 70 characters per part in UCS-2, so
//! this is the ordinary case for a Burmese OTP, not an edge one.

use crate::telegram::{self, escape_html, SendError, TelegramConfig};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use tauri::Emitter;

/// Minimum gap between sends: ~17 messages/minute, comfortably under Telegram's
/// 20/min group ceiling. Deliberately not 3.0 s — that lands exactly on the
/// limit, where a single clock skew or retry produces a 429.
const MIN_INTERVAL: Duration = Duration::from_millis(3_500);

/// Most items packed into one coalesced message. Ten `<code>` lines sit far
/// inside Telegram's 4096-character message limit while giving the queue a 10×
/// throughput multiplier under load.
const MAX_BATCH: usize = 10;

/// Queue ceiling. Reached only when the network has been unreachable for a long
/// stretch; past it the **oldest** items are dropped, because a code that has
/// been waiting the longest is the one most likely to have expired already.
const MAX_QUEUE: usize = 500;

/// Cap on a retry pause, whether it came from Telegram's `retry_after` or from
/// the network backoff. A pathological value must not park this thread past the
/// point where `stop_live` is waiting to join it.
const MAX_BACKOFF: Duration = Duration::from_secs(60);

/// First pause after the network fails to carry a request. Doubles up to
/// [`MAX_BACKOFF`] while the failures continue, and resets on the first success.
const RETRY_BASE: Duration = Duration::from_secs(5);

/// How much of a message body one bubble carries. Long promotional SMS are
/// common and the operator is here for the code, not the marketing.
const BODY_CHARS: usize = 400;

/// One message on its way to Telegram.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardItem {
    /// `SmsItem.id`. The edit key: a completion arriving for a fragment already
    /// forwarded carries the same id.
    pub id: u64,
    pub port: String,
    /// The SIM's own number when known, so the bubble names the line rather than
    /// a tty that renumbers on every hotplug.
    pub sim: Option<String>,
    pub from: String,
    pub body: String,
    pub otp: Option<String>,
}

/// What the queue hands the sender on each tick.
#[derive(Debug, PartialEq, Eq)]
enum Work {
    Post(Vec<ForwardItem>),
    Amend { item: ForwardItem, message_id: i64 },
}

/// Truncate on a character boundary, marking that something was cut.
///
/// `chars()` rather than byte slicing: a UCS-2 SMS is Burmese more often than
/// not, and `&s[..n]` on a multi-byte boundary panics.
fn clip(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let head: String = text.chars().take(limit).collect();
    format!("{head}…")
}

/// The line naming which SIM a message landed on.
///
/// Falls back to the tty name only when the number is not known yet; a number is
/// what the operator recognises.
fn origin(item: &ForwardItem) -> String {
    match item.sim.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(sim) => escape_html(sim),
        None => escape_html(&item.port),
    }
}

/// One message, one bubble.
///
/// The code goes in `<code>`, which every Telegram client makes tap-to-copy —
/// the single most common thing the operator does with it.
pub fn format_one(item: &ForwardItem) -> String {
    let body = escape_html(&clip(&item.body, BODY_CHARS));
    let from = escape_html(&item.from);
    match &item.otp {
        Some(otp) => format!(
            "🔐 <code>{}</code>\n\n<b>{}</b> · from {}\n{}",
            escape_html(otp),
            origin(item),
            from,
            body
        ),
        None => format!(
            "📨 <b>{}</b> · from {}\n{}",
            origin(item),
            from,
            body
        ),
    }
}

/// Several messages, one bubble.
///
/// Only the code, the line and the sender per row: the point of coalescing is to
/// beat the rate limit, and a wall of message bodies would defeat the operator's
/// reason for looking. Non-OTP items still get a line so nothing vanishes
/// silently.
pub fn format_batch(items: &[ForwardItem]) -> String {
    if let [only] = items {
        return format_one(only);
    }
    let mut out = format!("🔐 <b>{} new messages</b>\n", items.len());
    for item in items {
        let line = match &item.otp {
            Some(otp) => format!(
                "\n<code>{}</code> · {} · {}",
                escape_html(otp),
                origin(item),
                escape_html(&item.from)
            ),
            None => format!(
                "\n📨 {} · {} — {}",
                origin(item),
                escape_html(&item.from),
                escape_html(&clip(&item.body, 60))
            ),
        };
        out.push_str(&line);
    }
    out
}

/// Everything the sender thread and the callers share.
struct Shared {
    queue: Mutex<VecDeque<ForwardItem>>,
    /// Signals "queue changed" and "stop asked", so the thread parks instead of
    /// spinning while the bank is quiet.
    wake: Condvar,
    stop: AtomicBool,
}

/// The non-blocking handle live workers hold.
///
/// Cloneable and cheap: every per-port closure gets one.
#[derive(Clone)]
pub struct ForwarderHandle {
    shared: Arc<Shared>,
    forward_otp: bool,
    forward_non_otp: bool,
}

impl ForwarderHandle {
    /// Queue one message. Never blocks, never fails, never touches the network.
    ///
    /// Filtering happens here rather than on the sender thread so a bank
    /// configured for OTPs only does not pay queue traffic for every
    /// promotional SMS it receives.
    pub fn deliver(&self, item: ForwardItem) {
        let wanted = if item.otp.is_some() {
            self.forward_otp
        } else {
            self.forward_non_otp
        };
        if !wanted {
            return;
        }
        let mut q = self.shared.queue.lock().unwrap_or_else(|e| e.into_inner());
        while q.len() >= MAX_QUEUE {
            q.pop_front();
        }
        q.push_back(item);
        drop(q);
        self.shared.wake.notify_one();
    }

    /// Ask the sender thread to flush what it has and exit.
    fn stop(&self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        self.shared.wake.notify_all();
    }
}

/// Owns the sender thread; dropping it is not enough, call [`Forwarder::shutdown`].
pub struct Forwarder {
    handle: ForwarderHandle,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Forwarder {
    pub fn handle(&self) -> ForwarderHandle {
        self.handle.clone()
    }

    /// Stop accepting work, flush the remainder as one final message, and join.
    ///
    /// Called from the live supervisor after every port worker has been joined,
    /// so a code that arrived in the last second still lands rather than being
    /// thrown away with the queue.
    pub fn shutdown(mut self) {
        self.handle.stop();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Take the next unit of work off the queue.
///
/// Pure and separate from the thread so the coalescing rule is testable without
/// a channel, a clock or a network: an item that already has a bubble goes alone
/// as an edit, otherwise up to [`MAX_BATCH`] consecutive fresh items are packed
/// together. Stopping at the first already-sent item is what keeps arrival order
/// intact.
fn take_work(queue: &mut VecDeque<ForwardItem>, sent: &HashMap<u64, i64>) -> Option<Work> {
    let first = queue.pop_front()?;
    if let Some(&message_id) = sent.get(&first.id) {
        return Some(Work::Amend {
            item: first,
            message_id,
        });
    }
    let mut batch = vec![first];
    while batch.len() < MAX_BATCH {
        let Some(next) = queue.front() else { break };
        if sent.contains_key(&next.id) {
            break;
        }
        batch.push(queue.pop_front().expect("front was just observed"));
    }
    Some(Work::Post(batch))
}

/// Put a unit of work back at the head after a recoverable failure, preserving
/// order so a burst is not reshuffled by one 429.
fn requeue(queue: &mut VecDeque<ForwardItem>, work: Work) {
    match work {
        Work::Post(items) => {
            for item in items.into_iter().rev() {
                queue.push_front(item);
            }
        }
        Work::Amend { item, .. } => queue.push_front(item),
    }
}

/// Event names. `forward:failed` is operational feedback and reaches a toast;
/// `forward:migrated` asks the frontend to save a chat id Telegram changed
/// underneath it.
const EVENT_FAILED: &str = "forward:failed";
const EVENT_MIGRATED: &str = "forward:migrated";

/// Start the sender thread.
///
/// Returns `None` when forwarding is off or unconfigured, so the caller can hold
/// an `Option` and skip `deliver` entirely rather than constructing a forwarder
/// that would drop everything.
pub fn start(app: tauri::AppHandle, config: TelegramConfig, filters: (bool, bool)) -> Option<Forwarder> {
    let (forward_otp, forward_non_otp) = filters;
    if config.bot_token.trim().is_empty() || config.chat_id.trim().is_empty() {
        log::warn!("Telegram forwarding is enabled but the token or group id is empty — skipping");
        return None;
    }
    if !forward_otp && !forward_non_otp {
        log::warn!("Telegram forwarding is enabled but both message filters are off — skipping");
        return None;
    }
    let client = match telegram::build_client(config.proxy_url.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Telegram forwarding disabled: {e}");
            let _ = app.emit(EVENT_FAILED, &serde_json::json!({ "error": e }));
            return None;
        }
    };

    let shared = Arc::new(Shared {
        queue: Mutex::new(VecDeque::new()),
        wake: Condvar::new(),
        stop: AtomicBool::new(false),
    });
    let handle = ForwarderHandle {
        shared: Arc::clone(&shared),
        forward_otp,
        forward_non_otp,
    };

    let thread = std::thread::spawn(move || {
        // Same policy as every other worker in this crate: a panic here is
        // reported, not swallowed. It must not reach `live_error`, which means
        // "this port stopped monitoring" — forwarding is bank-wide.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run(&app, &client, config, &shared);
        }));
        if outcome.is_err() {
            log::error!("Telegram forwarder thread panicked — forwarding has stopped");
            let _ = app.emit(
                EVENT_FAILED,
                &serde_json::json!({ "error": "Forwarder crashed — see the log" }),
            );
        }
    });

    log::info!(
        "Telegram forwarding on (otp={forward_otp}, non_otp={forward_non_otp})"
    );
    Some(Forwarder {
        handle,
        thread: Some(thread),
    })
}

/// The sender loop.
///
/// Waits on the condvar while idle, so a quiet bank costs nothing; wakes on a
/// delivery or on shutdown. `last_send` paces; `sent` maps `SmsItem` ids to the
/// Telegram messages they produced.
fn run(
    app: &tauri::AppHandle,
    client: &reqwest::blocking::Client,
    mut config: TelegramConfig,
    shared: &Shared,
) {
    let mut sent: HashMap<u64, i64> = HashMap::new();
    let mut last_send = Instant::now() - MIN_INTERVAL;
    // Grows while the network keeps refusing to carry requests, resets on the
    // first success. Without this a dead uplink is retried every 3.5 s, which
    // spends the whole outage hammering a socket that cannot open.
    let mut retry_in = RETRY_BASE;

    loop {
        let stopping = shared.stop.load(Ordering::Relaxed);
        let work = {
            let mut q = shared.queue.lock().unwrap_or_else(|e| e.into_inner());
            if q.is_empty() {
                if stopping {
                    return;
                }
                // No timeout: nothing else drives this thread, so there is
                // nothing to poll for.
                let (guard, _) = shared
                    .wake
                    .wait_timeout(q, Duration::from_secs(30))
                    .unwrap_or_else(|e| e.into_inner());
                drop(guard);
                continue;
            }
            // Shutdown flushes everything left as one message rather than
            // pacing it out: the operator is waiting for live mode to stop, and
            // a code that arrived a second ago should still land.
            if stopping {
                let items: Vec<ForwardItem> = q.drain(..).collect();
                drop(q);
                let text = format_batch(&items);
                if let Err(e) = telegram::send_message(client, &config, &text) {
                    report(app, &e);
                }
                return;
            }
            match take_work(&mut q, &sent) {
                Some(w) => w,
                None => continue,
            }
        };

        let wait = MIN_INTERVAL.saturating_sub(last_send.elapsed());
        if !wait.is_zero() && !sleep_unless_stopped(shared, wait) {
            // Stop arrived mid-pause: hand the work back so the shutdown flush
            // picks it up instead of dropping it.
            let mut q = shared.queue.lock().unwrap_or_else(|e| e.into_inner());
            requeue(&mut q, work);
            continue;
        }

        match deliver_now(client, &config, &work) {
            Ok(assignments) => {
                last_send = Instant::now();
                retry_in = RETRY_BASE;
                for (id, message_id) in assignments {
                    sent.insert(id, message_id);
                }
            }
            Err(SendError::Migrated(new_id)) => {
                // Telegram hands back the replacement id, so this heals itself.
                // The operator is told what happened, not asked to fix it.
                log::info!("Telegram group migrated to {new_id}; forwarding there from now on");
                config.chat_id = new_id.to_string();
                let _ = app.emit(
                    EVENT_MIGRATED,
                    &serde_json::json!({ "chatId": config.chat_id }),
                );
                let mut q = shared.queue.lock().unwrap_or_else(|e| e.into_inner());
                requeue(&mut q, work);
            }
            Err(SendError::RateLimited(secs)) => {
                let pause = Duration::from_secs(secs).min(MAX_BACKOFF);
                log::warn!("Telegram rate limited; holding {}s", pause.as_secs());
                let mut q = shared.queue.lock().unwrap_or_else(|e| e.into_inner());
                requeue(&mut q, work);
                drop(q);
                sleep_unless_stopped(shared, pause);
                last_send = Instant::now();
            }
            // The request never reached Telegram — a dead route, DNS, a timeout,
            // a block page. Dropping it here is what made a momentary uplink
            // blip lose an OTP the operator was waiting for, so it goes back on
            // the queue and the pause grows until the network returns.
            Err(SendError::Unreachable(msg)) => {
                log::warn!(
                    "Telegram unreachable, retrying in {}s: {msg}",
                    retry_in.as_secs()
                );
                let _ = app.emit(EVENT_FAILED, &serde_json::json!({ "error": msg }));
                let mut q = shared.queue.lock().unwrap_or_else(|e| e.into_inner());
                requeue(&mut q, work);
                drop(q);
                sleep_unless_stopped(shared, retry_in);
                retry_in = (retry_in * 2).min(MAX_BACKOFF);
                last_send = Instant::now();
            }
            // Telegram received the request and refused it. Retrying cannot
            // change the answer, and keeping the item would block every code
            // behind it forever.
            Err(e @ SendError::Rejected(_)) => {
                report(app, &e);
                last_send = Instant::now();
            }
        }
    }
}

/// Perform one unit of work, returning the id→message_id pairs it established.
///
/// A coalesced post maps **every** id in the batch to the one bubble it produced,
/// so a fragment that completes later edits the message its batch is part of
/// rather than posting again.
fn deliver_now(
    client: &reqwest::blocking::Client,
    config: &TelegramConfig,
    work: &Work,
) -> Result<Vec<(u64, i64)>, SendError> {
    match work {
        Work::Post(items) => {
            let text = format_batch(items);
            let message_id = telegram::send_message(client, config, &text)?;
            Ok(items.iter().map(|i| (i.id, message_id)).collect())
        }
        Work::Amend { item, message_id } => {
            // A batch bubble cannot be rewritten to hold one item's full text
            // without discarding its neighbours, so an amendment inside a batch
            // is skipped rather than allowed to erase codes already delivered.
            telegram::edit_message(client, config, *message_id, &format_one(item))?;
            Ok(Vec::new())
        }
    }
}

/// Surface a send failure to the operator.
///
/// The error text has already been through `telegram::redact`, so the token
/// cannot reach the log or the toast. Repeats collapse in the toast queue, which
/// matters when a whole bank's worth of codes fail against one dead network.
fn report(app: &tauri::AppHandle, error: &SendError) {
    let text = error.to_string();
    log::warn!("Telegram forward failed: {text}");
    let _ = app.emit(EVENT_FAILED, &serde_json::json!({ "error": text }));
}

/// Sleep, but wake early if shutdown is asked. Returns `false` when interrupted.
fn sleep_unless_stopped(shared: &Shared, dur: Duration) -> bool {
    let guard = shared.queue.lock().unwrap_or_else(|e| e.into_inner());
    let (_guard, timeout) = shared
        .wake
        .wait_timeout(guard, dur)
        .unwrap_or_else(|e| e.into_inner());
    // Timing out means the pause completed uninterrupted. Being woken early is
    // either a new delivery (harmless, the pace still applies) or a stop.
    timeout.timed_out() || !shared.stop.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: u64, otp: Option<&str>) -> ForwardItem {
        ForwardItem {
            id,
            port: format!("ttyUSB{id}"),
            sim: Some(format!("+95912345{id:04}")),
            from: "MPT".into(),
            body: "Your code is 123456".into(),
            otp: otp.map(str::to_string),
        }
    }

    #[test]
    fn clip_cuts_on_a_character_boundary_not_a_byte_one() {
        // Byte slicing Burmese at an arbitrary index panics; this is why `clip`
        // counts chars.
        let burmese = "ကုဒ်နံပါတ်မှာ ၁၂၃၄၅၆ ဖြစ်ပါသည်";
        let out = clip(burmese, 5);
        assert_eq!(out.chars().count(), 6, "5 chars plus the ellipsis");
        assert!(out.ends_with('…'));
    }

    #[test]
    fn clip_leaves_short_text_untouched() {
        assert_eq!(clip("123456", 400), "123456");
    }

    #[test]
    fn origin_prefers_the_sim_number_over_the_tty_name() {
        let mut i = item(1, Some("123456"));
        assert_eq!(origin(&i), "+959123450001");
        i.sim = None;
        assert_eq!(origin(&i), "ttyUSB1");
        i.sim = Some("   ".into());
        assert_eq!(origin(&i), "ttyUSB1", "a blank number is not a number");
    }

    #[test]
    fn format_one_puts_the_code_in_a_tap_to_copy_block() {
        let text = format_one(&item(1, Some("483920")));
        assert!(text.contains("<code>483920</code>"), "{text}");
    }

    /// The body is chosen by whoever can text the bank. Under HTML parse mode an
    /// unescaped `<` makes Telegram reject the whole send, which loses the code
    /// silently.
    #[test]
    fn format_one_escapes_a_hostile_body_and_sender() {
        let mut i = item(1, Some("483920"));
        i.body = "<b>win</b> & <script>".into();
        i.from = "A<B>".into();
        let text = format_one(&i);
        assert!(text.contains("&lt;b&gt;win&lt;/b&gt; &amp; &lt;script&gt;"), "{text}");
        assert!(text.contains("A&lt;B&gt;"), "{text}");
    }

    #[test]
    fn format_batch_of_one_is_the_single_layout() {
        let one = [item(1, Some("483920"))];
        assert_eq!(format_batch(&one), format_one(&one[0]));
    }

    #[test]
    fn format_batch_lists_every_code() {
        let items = vec![item(1, Some("111111")), item(2, Some("222222"))];
        let text = format_batch(&items);
        assert!(text.contains("2 new messages"), "{text}");
        assert!(text.contains("<code>111111</code>"), "{text}");
        assert!(text.contains("<code>222222</code>"), "{text}");
    }

    /// A non-OTP message must still appear. Dropping it from a coalesced bubble
    /// would be silent message loss.
    #[test]
    fn format_batch_keeps_non_otp_rows() {
        let items = vec![item(1, Some("111111")), item(2, None)];
        let text = format_batch(&items);
        assert!(text.contains("<code>111111</code>"), "{text}");
        assert!(text.contains("📨"), "{text}");
    }

    #[test]
    fn take_work_packs_consecutive_fresh_items() {
        let mut q: VecDeque<ForwardItem> = (1..=3).map(|i| item(i, Some("111111"))).collect();
        let sent = HashMap::new();
        match take_work(&mut q, &sent) {
            Some(Work::Post(batch)) => assert_eq!(batch.len(), 3),
            other => panic!("expected Post, got {other:?}"),
        }
        assert!(q.is_empty());
    }

    #[test]
    fn take_work_never_packs_more_than_the_batch_cap() {
        let mut q: VecDeque<ForwardItem> =
            (1..=25).map(|i| item(i, Some("111111"))).collect();
        match take_work(&mut q, &HashMap::new()) {
            Some(Work::Post(batch)) => assert_eq!(batch.len(), MAX_BATCH),
            other => panic!("expected Post, got {other:?}"),
        }
        assert_eq!(q.len(), 25 - MAX_BATCH, "the rest stays queued in order");
        assert_eq!(q.front().map(|i| i.id), Some(MAX_BATCH as u64 + 1));
    }

    /// The concatenated-SMS path: the completion carries the id of the fragment
    /// already posted, so it edits that bubble instead of adding a second one.
    #[test]
    fn take_work_turns_an_already_sent_id_into_an_edit() {
        let mut q: VecDeque<ForwardItem> = VecDeque::from([item(7, Some("483920"))]);
        let sent = HashMap::from([(7u64, 99i64)]);
        assert_eq!(
            take_work(&mut q, &sent),
            Some(Work::Amend {
                item: item(7, Some("483920")),
                message_id: 99
            })
        );
    }

    /// An edit must not be swept into a batch — it belongs to one existing
    /// message. Stopping the pack at that boundary is also what keeps the queue
    /// in arrival order.
    #[test]
    fn take_work_stops_packing_at_an_already_sent_item() {
        let mut q: VecDeque<ForwardItem> =
            VecDeque::from([item(1, Some("111111")), item(2, Some("222222"))]);
        let sent = HashMap::from([(2u64, 55i64)]);
        match take_work(&mut q, &sent) {
            Some(Work::Post(batch)) => assert_eq!(batch.len(), 1),
            other => panic!("expected a single-item Post, got {other:?}"),
        }
        assert_eq!(q.front().map(|i| i.id), Some(2));
    }

    #[test]
    fn take_work_on_an_empty_queue_is_none() {
        assert_eq!(take_work(&mut VecDeque::new(), &HashMap::new()), None);
    }

    /// A 429 must not reshuffle a burst: the codes go back in the order they
    /// arrived.
    #[test]
    fn requeue_restores_arrival_order() {
        let mut q: VecDeque<ForwardItem> = VecDeque::from([item(9, Some("999999"))]);
        let batch = vec![item(1, Some("111111")), item(2, Some("222222"))];
        requeue(&mut q, Work::Post(batch));
        assert_eq!(q.iter().map(|i| i.id).collect::<Vec<_>>(), vec![1, 2, 9]);
    }

    #[test]
    fn requeue_puts_an_amendment_back_at_the_head() {
        let mut q: VecDeque<ForwardItem> = VecDeque::from([item(9, Some("999999"))]);
        requeue(
            &mut q,
            Work::Amend {
                item: item(3, None),
                message_id: 42,
            },
        );
        assert_eq!(q.front().map(|i| i.id), Some(3));
    }
}
