use crate::core::at::AtChannel;
use crate::core::decoder;
use crate::core::models::{self, SmsMessage};
use crate::core::reassemble::Reassembler;
use std::collections::{HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub enum LiveEvent {
    Ready {
        port: String,
    },
    /// The port opened but no modem answered `AT` on it — an empty SIM slot or
    /// an unpowered stick. Distinct from `Reconnecting`, which means a port we
    /// had been talking to went away.
    Offline {
        port: String,
        error: String,
    },
    Reconnecting {
        port: String,
        error: String,
    },
    Batch {
        port: String,
        items: Vec<SmsMessage>,
        is_new: bool,
    },
    Sms {
        port: String,
        message: SmsMessage,
        is_new: bool,
    },
    Closed {
        port: String,
        error: Option<String>,
    },
}

/// Backoff between reconnect attempts. Starts at 2 s and doubles, capped at
/// 30 s — live mode is expected to ride out USB-hub resets and the kind of
/// mass `serial read failed` storm the SIM bank hits under load, so we keep
/// retrying for as long as the operator leaves Live on.
const RECONNECT_MIN: Duration = Duration::from_secs(2);
const RECONNECT_MAX: Duration = Duration::from_secs(30);

/// Re-probe cadence for a port where nothing answers. An empty slot is a stable
/// condition rather than a transient glitch, so this is deliberately slower than
/// the reconnect backoff — but a stick inserted mid-session is still picked up
/// without restarting live mode.
const OFFLINE_RETRY: Duration = Duration::from_secs(60);

/// How often a live worker deletes expired messages from its SIM. SIM storage
/// holds only ~20–50 messages; once full the modem silently rejects new SMS,
/// so an always-on live session has to prune as it goes.
const SIM_SWEEP_EVERY: Duration = Duration::from_secs(600);

/// Which "this port is not working" transitions have already been announced.
///
/// A live worker retries for as long as the operator leaves Live on, so every
/// failure branch would otherwise emit one event per attempt — a 2 s backoff
/// across a 64-stick bank is a flood the UI cannot absorb, which is what these
/// latches exist to prevent.
///
/// The half that matters is the re-arming. A latch that is only ever set makes
/// the *second* outage on a port silent, and a silently offline stick in a SIM
/// bank looks exactly like a healthy one. So the only thing that clears
/// `offline_reported` is a modem actually answering `AT` (or the port
/// disappearing, which invalidates the verdict) — never a mere reconnect
/// attempt, and never `open()` succeeding, because an empty slot opens cleanly.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct OutageLatch {
    /// The port could not be opened, or was lost while we held it.
    down_reported: bool,
    /// The port opened but nothing answered `AT` on it.
    offline_reported: bool,
}

impl OutageLatch {
    /// Record that the port could not be opened. `true` when the caller should
    /// emit `Reconnecting` — a new outage rather than another retry of one
    /// already reported.
    fn report_down(&mut self) -> bool {
        let first = !self.down_reported;
        if first {
            self.mark_down();
        }
        first
    }

    /// Record a port loss that is always worth an event by itself: we were
    /// talking to this modem a moment ago, so the transition is real.
    fn mark_down(&mut self) {
        self.down_reported = true;
        // A vanished port invalidates any earlier "no modem here" verdict. The
        // Reconnecting event overwrites `live_error` in the command layer, so if
        // the port comes back and is *still* silent the silence has to be
        // announced again, or the UI is left saying "Reconnecting…" about a slot
        // we already know is empty.
        self.offline_reported = false;
    }

    /// Record that the liveness probe found silence. `true` when the caller
    /// should emit `Offline`.
    fn report_offline(&mut self) -> bool {
        !std::mem::replace(&mut self.offline_reported, true)
    }

    /// A modem answered `AT` on this port: it is genuinely back, so both latches
    /// re-arm. `true` when the port had been reported silent, which is worth a
    /// log line.
    fn modem_answered(&mut self) -> bool {
        self.down_reported = false;
        std::mem::replace(&mut self.offline_reported, false)
    }
}

pub fn run_live<F>(
    port_name: String,
    stop: Arc<AtomicBool>,
    retention: Option<Duration>,
    on_event: F,
) where
    F: Fn(LiveEvent) + Send + 'static,
{
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_live_inner(&port_name, &stop, retention, &on_event);
    }));
    if result.is_err() {
        log::error!("Live worker crashed: {}", port_name);
        on_event(LiveEvent::Closed {
            port: port_name,
            error: Some("Worker crashed".into()),
        });
    }
}

fn run_live_inner<F>(
    port_name: &str,
    stop: &Arc<AtomicBool>,
    retention: Option<Duration>,
    on_event: &F,
) where
    F: Fn(LiveEvent) + Send + 'static,
{
    let mut asm = Reassembler::new();
    // Fingerprints of every message already surfaced for this port. After a
    // reconnect we re-read the whole SIM via AT+CMGL — without dedup, messages
    // that never left the SIM would be emitted again (with fresh ids) and show
    // up as duplicates. The fingerprint keys on sender + SCTS + full text, so
    // genuinely new messages are never suppressed.
    let mut seen: HashSet<u64> = HashSet::new();
    let mut ever_connected = false;
    let mut backoff = RECONNECT_MIN;
    // One event per outage, and one per *recovery* too: see `OutageLatch`.
    let mut latch = OutageLatch::default();
    // Set on every (re)connect, just before the monitoring loop reads it.
    let mut last_sweep;

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }

        let mut ch = match AtChannel::open(port_name) {
            Ok(ch) => ch,
            Err(e) => {
                // Port vanished (USB reset, unplug, permissions). Surface the
                // transition once so the UI can show "Reconnecting", then back
                // off silently — no point flooding the log with one warn per
                // port per retry across a 64-stick bank.
                if latch.report_down() {
                    on_event(LiveEvent::Reconnecting {
                        port: port_name.to_string(),
                        error: e,
                    });
                }
                if !sleep_stop_aware(stop, backoff) {
                    break;
                }
                backoff = (backoff * 2).min(RECONNECT_MAX);
                continue;
            }
        };

        // Reconnected: reset backoff. The port opening again is *not* proof the
        // modem is back — an empty slot opens cleanly — so the latches stay as
        // they are until the probe below answers.
        backoff = RECONNECT_MIN;
        if latch.down_reported {
            log::info!("{}: port reopened after outage", port_name);
        }

        // Liveness gate. An empty SIM slot still exposes a tty node that opens
        // cleanly, so without this the worker ran the whole SMS setup against
        // silence (~22 s of timeouts) and then announced `Ready` for a port that
        // can never deliver a message — which is how a bank with 7 SIMs came up
        // showing 64 green "LIVE" badges.
        if !crate::core::modem::probe_channel(&mut ch) {
            if latch.report_offline() {
                let why = crate::core::modem::probe_failure_reason(&ch);
                log::warn!("{}: {} — live monitoring idle for this port", port_name, why);
                on_event(LiveEvent::Offline {
                    port: port_name.to_string(),
                    error: why,
                });
            }
            if !sleep_stop_aware(stop, OFFLINE_RETRY) {
                break;
            }
            continue;
        }
        // The probe answered: this is the only signal that proves a modem is
        // there, so it is where both latches re-arm.
        if latch.modem_answered() {
            log::info!("{}: modem started answering", port_name);
        }

        // PDU mode lets us read the UDH of concatenated (long) SMS so fragments
        // can be joined into one complete message instead of truncated pieces.
        // Shared with the scan path (`modem::read_port`) so the text-mode
        // fallback can't drift between the two again.
        let pdu_ok = crate::core::modem::setup_sms_mode(&mut ch);
        ch.send("AT+CNMI=2,1,0,0,0", 3000);
        let stale = ch.take_notifications();
        if !stale.is_empty() {
            log::debug!(
                "{}: dropped {} stale notification(s)",
                port_name,
                stale.len()
            );
        }

        // Re-read everything on the SIM. On the first connect this is the
        // initial backfill; after an outage it catches the messages that landed
        // while nobody was listening (the exact gap that used to silently drop
        // traffic until the user manually stopped + scanned).
        let mut initial: Vec<SmsMessage> = if pdu_ok {
            let r = ch.send("AT+CMGL=4", 15000);
            collect_parts(&r, port_name, &mut asm)
        } else {
            // Text-mode fallback: no UDH available; fragments appear as-is.
            // `setup_sms_mode` has already put the modem *into* text mode — this
            // branch used to assume it was there already.
            let r = ch.send("AT+CMGL=\"ALL\"", 15000);
            if r.contains("+CMGL:") {
                decoder::parse_text_mode_list(&r, port_name)
            } else {
                Vec::new()
            }
        };
        // Groups still missing parts are surfaced immediately as partials —
        // nothing already stored on the SIM is ever hidden while we wait for
        // the missing parts. The command layer matches later completions (same
        // sender + receive time) against these partial rows and swaps the text
        // in place, so the user first sees the fragment, then the full message.
        initial.extend(asm.peek_partials());

        // Retention applies to SIM storage, not just the inbox list. This
        // worker holds the port exclusively open, so it is the only thing that
        // *can* prune the SIM while live mode runs — and an unpruned SIM fills
        // up and starts silently rejecting new SMS.
        if let Some(cutoff) = retention.map(models::retention_cutoff_ms) {
            let doomed = models::expired_indices(&initial, cutoff);
            if !doomed.is_empty() {
                let n = delete_indices(&mut ch, &doomed);
                log::info!("{}: SIM cleanup deleted {} expired message(s)", port_name, n);
            }
            initial.retain(|m| !models::is_expired(m, cutoff));
        }
        last_sweep = Instant::now();

        let fresh = dedup(&mut seen, initial);
        if !fresh.is_empty() {
            log::info!(
                "{}: {} batch {} msg(s) (pdu={})",
                port_name,
                if ever_connected { "reconnect" } else { "initial" },
                fresh.len(),
                pdu_ok
            );
            if ever_connected {
                // Arrived while we were blind → flag as new so the inbox
                // highlights them and OTPs toast.
                for m in fresh {
                    on_event(LiveEvent::Sms {
                        port: port_name.to_string(),
                        message: m,
                        is_new: true,
                    });
                }
            } else {
                on_event(LiveEvent::Batch {
                    port: port_name.to_string(),
                    items: fresh,
                    is_new: false,
                });
            }
        }
        ever_connected = true;

        if ch.is_dead() {
            log::warn!("{}: port lost during startup", port_name);
            on_event(LiveEvent::Reconnecting {
                port: port_name.to_string(),
                error: ch
                    .death_reason()
                    .map(|r| format!("Port lost: {r}"))
                    .unwrap_or_else(|| "Port lost".into()),
            });
            latch.mark_down();
            if !sleep_stop_aware(stop, backoff) {
                break;
            }
            backoff = (backoff * 2).min(RECONNECT_MAX);
            continue;
        }

        on_event(LiveEvent::Ready {
            port: port_name.to_string(),
        });

        let mut queue: VecDeque<i32> = VecDeque::new();
        let mut died = false;
        while !stop.load(Ordering::Relaxed) {
            if let Some(idx) = queue.pop_front() {
                for more in handle_cmgr(&mut ch, idx, port_name, pdu_ok, &mut asm, &mut seen, on_event) {
                    queue.push_back(more);
                }
            } else if let Some(note) = ch.wait_notification(500) {
                if let Some(idx) = decoder::parse_cmti_index(note.trim()) {
                    log::debug!("{}: +CMTI idx {}", port_name, idx);
                    queue.push_back(idx);
                }
            }

            // Release incomplete concat groups after a grace period — the
            // partial was already shown via peek_partials, and a completion
            // arriving later still swaps it because matching happens on
            // sender + receive time, not on text equality.
            for msg in asm.flush_stale(crate::core::reassemble::STALE_AFTER) {
                if seen.insert(fingerprint(&msg)) {
                    on_event(LiveEvent::Sms {
                        port: port_name.to_string(),
                        message: msg,
                        is_new: true,
                    });
                }
            }

            if ch.is_dead() {
                died = true;
                break;
            }

            // Periodic SIM pruning so a long-running live session can't fill
            // the SIM. Cheap: one CMGL every SIM_SWEEP_EVERY, and the delete
            // only runs when something actually aged out.
            if let Some(cutoff) = retention.map(models::retention_cutoff_ms) {
                if last_sweep.elapsed() >= SIM_SWEEP_EVERY {
                    last_sweep = Instant::now();
                    let n = sweep_expired(&mut ch, port_name, pdu_ok, cutoff);
                    if n > 0 {
                        log::info!("{}: SIM cleanup deleted {} expired message(s)", port_name, n);
                    }
                }
            }
        }

        // Cleanup path: stop requested → reset the modem and tell the UI we are
        // gone for good (no reconnect). The Closed(None) handler is a no-op on
        // the frontend side (stop_live already reset the UI), but emitting it
        // keeps the join supervisor from waiting forever.
        ch.send("AT+CNMI=1,0,0,1,0", 1500);
        ch.send("AT+CSCS=\"GSM\"", 1000);
        // Unconditional: text mode is the app's resting state, and the text-mode
        // fallback now switches the character set to UCS2 as well, so both
        // branches leave something to restore.
        ch.send("AT+CMGF=1", 1500);

        if !died {
            // Clean stop — exit entirely.
            on_event(LiveEvent::Closed {
                port: port_name.to_string(),
                error: None,
            });
            return;
        }

        // Port died mid-monitoring → reconnect cycle. Surface the OS error so
        // the operator can tell a real device loss from a transient timeout.
        let reason = ch
            .death_reason()
            .map(|r| format!("Port lost: {r}"))
            .unwrap_or_else(|| "Port lost".into());
        on_event(LiveEvent::Reconnecting {
            port: port_name.to_string(),
            error: reason,
        });
        latch.mark_down();
        if !sleep_stop_aware(stop, backoff) {
            on_event(LiveEvent::Closed {
                port: port_name.to_string(),
                error: None,
            });
            return;
        }
        backoff = (backoff * 2).min(RECONNECT_MAX);
    }

    // Loop fell through because stop was requested during a backoff sleep.
    on_event(LiveEvent::Closed {
        port: port_name.to_string(),
        error: None,
    });
}

/// Delete SIM slots over an already-open channel. Highest index first so a
/// modem that renumbers on delete can't shift slots we have not visited yet.
fn delete_indices(ch: &mut AtChannel, indices: &[i32]) -> usize {
    let mut deleted = 0usize;
    for idx in indices.iter().rev() {
        if ch.is_dead() {
            break;
        }
        if ch.send(&format!("AT+CMGD={idx}"), 3000).contains("OK") {
            deleted += 1;
        }
    }
    deleted
}

/// Re-read the SIM and delete everything past retention. Used on the live
/// worker's own channel, where reopening the port is not an option.
fn sweep_expired(ch: &mut AtChannel, port_name: &str, pdu_mode: bool, cutoff: i64) -> usize {
    let resp = if pdu_mode {
        ch.send("AT+CMGL=4", 15000)
    } else {
        ch.send("AT+CMGL=\"ALL\"", 15000)
    };
    if !resp.contains("+CMGL:") {
        return 0;
    }
    // Per-fragment rows here: PDU list entries carry their own SIM index, so
    // expired fragments are removed individually without reassembly.
    let msgs: Vec<SmsMessage> = if pdu_mode {
        decoder::parse_pdu_list(&resp, port_name)
            .into_iter()
            .map(|d| d.message)
            .collect()
    } else {
        decoder::parse_text_mode_list(&resp, port_name)
    };
    let doomed = models::expired_indices(&msgs, cutoff);
    if doomed.is_empty() {
        return 0;
    }
    delete_indices(ch, &doomed)
}

/// Sleep in small slices so a stop request wakes us immediately instead of
/// blocking up to `total` before the operator's "Stop Live" is honoured.
fn sleep_stop_aware(stop: &AtomicBool, total: Duration) -> bool {
    let deadline = Instant::now() + total;
    while Instant::now() < deadline {
        if stop.load(Ordering::Relaxed) {
            return false;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        std::thread::sleep(remaining.min(Duration::from_millis(200)));
    }
    !stop.load(Ordering::Relaxed)
}

/// Parse a CMGL response and feed every fragment through the reassembler,
/// returning assembled standalone messages. Incomplete groups are left pending
/// in the reassembler so later parts can complete them — never force-flushed.
fn collect_parts(resp: &str, port_name: &str, asm: &mut Reassembler) -> Vec<SmsMessage> {
    let parts = decoder::parse_pdu_list(resp, port_name);
    let mut msgs: Vec<SmsMessage> = Vec::new();
    for d in parts {
        match d.concat {
            Some(c) => {
                if let Some(done) = asm.push(&d.message, c) {
                    msgs.push(done);
                }
            }
            None => msgs.push(d.message),
        }
    }
    msgs
}

fn handle_cmgr<F>(
    ch: &mut AtChannel,
    idx: i32,
    port_name: &str,
    pdu_mode: bool,
    asm: &mut Reassembler,
    seen: &mut HashSet<u64>,
    on_event: &F,
) -> Vec<i32>
where
    F: Fn(LiveEvent) + Send + 'static,
{
    let resp = ch.send(&format!("AT+CMGR={idx}"), 6000);

    let completed: Option<SmsMessage> = if pdu_mode {
        match decoder::parse_pdu_cmgr(&resp, port_name, idx) {
            Some(info) => {
                log::info!(
                    "{}: live SMS read (idx {}){}",
                    port_name,
                    idx,
                    if info.concat.is_some() { " [concat]" } else { "" }
                );
                match info.concat {
                    Some(c) => asm.push(&info.message, c),
                    None => Some(info.message),
                }
            }
            None => {
                log::debug!("{}: CMGR {} -> no message parsed", port_name, idx);
                None
            }
        }
    } else {
        match decoder::parse_cmgr(&resp, port_name, idx) {
            Some(msg) => {
                log::info!("{}: live SMS read (idx {})", port_name, idx);
                Some(msg)
            }
            None => {
                log::debug!("{}: CMGR {} -> no message parsed", port_name, idx);
                None
            }
        }
    };

    if let Some(message) = completed {
        if seen.insert(fingerprint(&message)) {
            on_event(LiveEvent::Sms {
                port: port_name.to_string(),
                message,
                is_new: true,
            });
        }
    }

    let mut more = Vec::new();
    for note in ch.take_notifications() {
        if let Some(extra_idx) = decoder::parse_cmti_index(note.trim()) {
            more.push(extra_idx);
        }
    }
    more
}

/// Drop messages whose fingerprint is already in `seen`, recording the rest.
fn dedup(seen: &mut HashSet<u64>, messages: Vec<SmsMessage>) -> Vec<SmsMessage> {
    messages
        .into_iter()
        .filter(|m| seen.insert(fingerprint(m)))
        .collect()
}

/// Stable per-message signature so a reconnect re-read of the SIM can't
/// re-emit something already shown. Sender + SCTS (millis) + full text: two
/// distinct messages colliding here would need identical sender, identical
/// receive second, and identical body — astronomically unlikely in practice.
/// Note: a partial concat message that later grows gets a NEW fingerprint
/// (text changed), which is exactly what lets the completion through dedup so
/// the command layer can swap the partial row for the full text.
fn fingerprint(m: &SmsMessage) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    m.from.hash(&mut h);
    m.received.timestamp_millis().hash(&mut h);
    m.text.hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::at::Transport;
    use std::io;
    use std::sync::Mutex;

    /// Replies "OK" to everything and records the commands it was sent.
    struct OkTransport {
        pending: Vec<u8>,
        sent: Arc<Mutex<Vec<String>>>,
    }

    impl Transport for OkTransport {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.pending.is_empty() {
                return Err(io::ErrorKind::TimedOut.into());
            }
            let n = self.pending.len().min(buf.len());
            buf[..n].copy_from_slice(&self.pending[..n]);
            self.pending.drain(..n);
            Ok(n)
        }

        fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
            let cmd = String::from_utf8_lossy(data).trim().to_string();
            self.sent.lock().unwrap().push(cmd);
            self.pending.extend_from_slice(b"\r\nOK\r\n");
            Ok(())
        }
    }

    #[test]
    fn delete_indices_removes_high_slots_first_and_counts_them() {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let mut ch = AtChannel::with_transport(
            "ttyUSB0",
            Box::new(OkTransport {
                pending: Vec::new(),
                sent: Arc::clone(&sent),
            }),
        );
        assert_eq!(delete_indices(&mut ch, &[1, 4, 7]), 3);
        assert_eq!(
            *sent.lock().unwrap(),
            vec!["AT+CMGD=7", "AT+CMGD=4", "AT+CMGD=1"]
        );
    }

    #[test]
    fn sweep_is_a_noop_when_the_sim_reports_no_messages() {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let mut ch = AtChannel::with_transport(
            "ttyUSB0",
            Box::new(OkTransport {
                pending: Vec::new(),
                sent: Arc::clone(&sent),
            }),
        );
        let cutoff = models::retention_cutoff_ms(Duration::from_secs(3600));
        assert_eq!(sweep_expired(&mut ch, "ttyUSB0", true, cutoff), 0);
        // Only the list command — nothing was deleted.
        assert_eq!(*sent.lock().unwrap(), vec!["AT+CMGL=4"]);
    }

    // ── Outage reporting latches ──
    //
    // `run_live_inner` itself cannot be driven from a unit test: it calls
    // `AtChannel::open(port_name)` on a real device node (there is no transport
    // injection on that path, unlike `AtChannel::with_transport`) and each
    // iteration sleeps for a 2–60 s backoff. The decision the bug lives in is
    // therefore tested directly.

    #[test]
    fn silence_is_announced_once_per_outage_not_once_per_probe() {
        let mut latch = OutageLatch::default();
        assert!(latch.report_offline(), "first silence must reach the UI");
        assert!(!latch.report_offline(), "re-probes must stay quiet");
        assert!(!latch.report_offline());
    }

    #[test]
    fn a_modem_answering_re_arms_the_offline_latch() {
        let mut latch = OutageLatch::default();
        assert!(latch.report_offline());
        assert!(
            latch.modem_answered(),
            "recovery from silence is worth a log"
        );
        assert!(
            latch.report_offline(),
            "a second outage must be reported again — a silently offline stick \
             is indistinguishable from a healthy one"
        );
    }

    #[test]
    fn a_port_that_opens_but_stays_silent_does_not_count_as_back() {
        let mut latch = OutageLatch::default();
        assert!(latch.report_down());
        // The port opens again but nothing answers `AT` — news of its own, since
        // an empty slot opens cleanly...
        assert!(latch.report_offline());
        // ...but not proof it is back, or a stick flapping between absent and
        // silent would emit a Reconnecting event on every cycle.
        assert!(!latch.report_down());
    }

    #[test]
    fn an_outage_after_a_confirmed_recovery_is_announced_again() {
        let mut latch = OutageLatch::default();
        assert!(latch.report_down(), "first failure to open");
        assert!(
            !latch.report_down(),
            "retries of the same outage stay quiet"
        );
        latch.modem_answered();
        assert!(latch.report_down(), "the next real outage is announced");
    }

    #[test]
    fn a_vanished_port_re_arms_the_silence_verdict_but_stays_bounded() {
        let mut latch = OutageLatch::default();
        // Empty slot → Offline. Then the node disappears → Reconnecting, which
        // overwrites the port's error text in the command layer.
        assert!(latch.report_offline());
        assert!(latch.report_down());
        // Back, still silent: the "Modem not responding" label has to be
        // restored, so this reports again.
        assert!(latch.report_offline());
        // ...but a port flapping between absent and silent forever cannot keep
        // emitting: the down latch is still set, so nothing further escapes.
        assert!(!latch.report_down());
        assert!(!latch.report_offline());
    }

    #[test]
    fn a_port_lost_mid_session_marks_down_without_consuming_a_report() {
        // `mark_down` is used where the event is emitted unconditionally (the
        // port died while we held it — always a real transition).
        let mut latch = OutageLatch::default();
        latch.mark_down();
        assert!(!latch.report_down(), "the outage is already reported");
        assert!(
            latch.report_offline(),
            "if it comes back silent, that is news"
        );
    }
}
