# 📨 Telegram Forwarding — Stage 1 + Stage 2 (implementation record)

> **Written:** 2026-09-03 · **Last update:** 2026-09-04 ·
> **Status:** Stage 1 + Stage 2 are **done — shipped in `v1.6.0`**, **all four**
> hardware tests confirmed (§F), and the two OTP false positives in §G fixed in
> `v1.6.0` / `v1.6.1`. The roadmap entry is `05 §1.1`, the dependency trap is `03 §18`.
>
> **⚠️ This doc has turned from a plan into a record.** §B/§C are the reasons why it
> was written the way it was — the implementation landed exactly along those lines.
> §E (what must not be done) is **still live**.

## Stage 2 — the files that were written

| File | What it contains |
|---|---|
| `src-tauri/src/forwarder.rs` **(new)** | `ForwardItem` · `ForwarderHandle::deliver` (non-blocking, filter, `MAX_QUEUE`=500 oldest-drop) · `Forwarder::shutdown` (final flush) · `take_work` / `requeue` (coalescing + edit boundary, pure) · `format_one` / `format_batch` / `clip` / `origin` · sender thread (`MIN_INTERVAL`=3.5s, `MAX_BATCH`=10, 429 honour, migration heal, `catch_unwind`) · 15 tests |
| `src-tauri/src/telegram.rs` | `edit_message` added (`message is not modified` = success) |
| `src-tauri/src/commands/telegram.rs` | `ForwardingConfigDto` + `split()` |
| `src-tauri/src/commands/mod.rs` | `start_live(retention_hours, forwarding)` · forwarder start/shutdown in the supervisor thread · `deliver` from **both** `Sms` arms |
| `src/lib/types.ts` | `forwarding.{enabled, forwardOtp, forwardNonOtp}` added |
| `src/lib/services/api.ts` | `forwardingArgs()` · sent by `startLive` · `forward:failed` / `forward:migrated` listeners |
| `src/lib/pages/Settings.svelte` | the three switches (per §D) |

**Validation (as Stage 2 was written, 2026-09-03):** Rust tests **202** · frontend tests 89 · clippy `-D warnings` clean ·
`--locked` release check good · `npm run check` 0/0.
*(Past v1.6.1 it is Rust **214** / frontend **89** — the current numbers are owned by `AGENTS.md` Validation.)*

---

## A. Stage 1 — already in place (none of this gets rewritten)

| File | What it contains |
|---|---|
| `src-tauri/src/telegram.rs` | `TelegramConfig` · `build_client` (SOCKS5 + `ensure_crypto_provider`) · `send_message` (→ returns `message_id`) · `get_me` · `detect_group` / `pick_group` · `SendError::{Migrated, RateLimited, Other}` · `interpret` · `redact` · `escape_html` · `test_message_html` · 20 tests |
| `src-tauri/src/commands/telegram.rs` | three commands: `verify_telegram_token` · `detect_telegram_group` · `send_telegram_test` (migration auto-heal) · `require_token` · `host_label` · 5 tests |
| `src/lib/types.ts` | `settings.forwarding = { botToken, chatId, proxyUrl }` |
| `src/lib/stores/settings.svelte.ts` | `forwarding` getter + `setForwarding` |
| `src/lib/pages/Settings.svelte` | `forwarding` group · `type: "text"` / `"secret"` branch · `pendingAction` loading state |
| `src/lib/services/api.ts` | `verifyTelegramToken` · `detectTelegramGroup` · `sendTelegramTest` |
| `src/lib/utils/telegram-preview.ts` | browser-preview parity + 10 tests |

**The point that matters:** `send_message` **returns Telegram's `message_id`**.
Stage 1 never uses it — it was kept deliberately for Stage 2's `editMessageText` (§B.1).

---

## B. The four hard problems of Stage 2

### B.1 Hook point — `sms:new` **alone is not enough** (the most important one)

The live event handler in `commands/mod.rs` has two arms, and both of them produce messages:

| Arm | file:line | Which event it emits | Forward it? |
|---|---|---|---|
| `LiveEvent::Batch` | `mod.rs:1087` | `messages:added` (`mod.rs:1104`) | **Never** — this is the entire inbox already sitting on the SIM when live starts. Forwarding it dumps every earlier message into the group |
| `LiveEvent::Sms` → `None` arm | `mod.rs:1171` | `sms:new` | **Yes** |
| `LiveEvent::Sms` → `Some` arm | `mod.rs:1163` | `messages:updated` | **Yes — `editMessageText`** |

What the `Some` arm is: `mod.rs:1137-1146` looks for a prefix match —
same `port` + `from` + `received` plus `message.text.starts_with(&it.message.text)` means
**no new row is added, the existing one is replaced**. Which means `sms:new` **is never emitted**.

**When does it happen:** concatenated SMS. `live.rs:268` shows the fragment first via
`asm.peek_partials()`, and `live.rs:344` `handle_cmgr` emits the complete message once the remaining
parts arrive. **Myanmar text fits only 70 characters per part in UCS-2, so this is not a rare case.**

Hooking `sms:new` alone means **nothing is sent at all** in this case — and that is this feature's
main silent-failure mode.

**The fix:**
1. Deliver to the forwarder from **both branches** of the `Sms` arm. `None` → `sendMessage`,
   `Some` → **`editMessageText`** (reusing the `message_id`)
2. Keep one `SmsItem.id` → Telegram `message_id` map (`HashMap<u64, i64>`). Clear it on every
   `start_live` (session-scoped), and clear alongside the retention purge (`mod.rs:1520`) too
3. If the id is not in the map in the `Some` arm (still waiting in the queue, or forwarding was off)
   → **post a new `sendMessage`**, never raise an error
4. The hook must **not** live at the `LiveEvent` layer — `extract_otp` (`mod.rs:1095`, `:1118`) and
   id allocation happen at the command layer. `live.rs` knows nothing about OTPs

**Dedup:** `live.rs:579` `fingerprint()` = `from` + `received` millis + `text` hash.
`live.rs:565` `dedup()` catches the reconnect re-read. But **a growing partial produces a new
fingerprint** (`live.rs:576` comment) — that is deliberate, it is what lets a completion through.
So the forwarder **cannot** use the fingerprint as its dedup key; use `SmsItem.id`.

### B.2 Rate limit — 20 messages per minute into one group

Telegram FAQ: *"In a group, bots are not be able to send more than 20 messages per minute."*

"send one every 3 seconds" = 20/min = **sitting exactly on the limit, not below it**.
If 64 ports burst at once the queue grows long and **the OTP arrives after it has expired** —
in that state the whole feature might as well not work at all.

**The fix — coalescing:** as queue depth grows, **combine several OTPs into one message**.
The limit counts *messages*, so throughput multiplies immediately.
The principle is the same as `src/lib/utils/toast-queue.ts` (when 16 ports fail at once,
the cards coalesce instead of stacking 16 of them) — this is an existing pattern in this repo.

`SendError::RateLimited(secs)` already exists (`telegram.rs`) — honour its `retry_after`.

### B.3 Thread model — HTTP must **never** be called from inside a live worker thread

The live worker holds its port exclusively (`live.rs:342` monitoring loop).
The HTTP timeout is `TIMEOUT = 15s` — blocking the worker for 15 seconds loses `+CMTI`
notifications, and it drags the `SIM_SWEEP_EVERY` cadence (`live.rs:67`, 600 seconds) along with it.

**The fix:** **one dedicated** forwarder thread + a `std::sync::mpsc` channel.
The command layer only does `tx.send(...)` (non-blocking). Pacing + coalescing +
retry all happen inside the thread. Wrap it in `catch_unwind` (this repo's per-worker pattern), and
on a panic **do not** write into `live_error` — that field means "this port has stopped monitoring"
(AGENTS.md backend invariants). Use a global forwarder status + a toast.

> **What was actually written:** `Arc<Shared>` instead of `mpsc` — `Mutex<VecDeque<ForwardItem>>` +
> `Condvar wake` + `AtomicBool stop` (`forwarder.rs:169`). `mpsc` cannot bound queue depth and
> cannot do oldest-drop either — obeying §E's "no unbounded queue" means holding the `VecDeque`
> yourself. `deliver` is still non-blocking and the thread still parks (it does not spin), which is
> the same either way. On shutdown, dropping the handle is **not enough** —
> `Forwarder::shutdown` flushes the remainder as one final message and joins.

### B.4 Config lifetime

**The same road** as `retentionHours`: pass it as an argument of `start_live` (`api.ts:469` and
`mod.rs:934-940` are the precedent). **No new config file** on the Rust side —
`core/sim_directory.rs:3-5` writes the invariant down: *"User preferences live in exactly one
place: the frontend settings store."*

The cost to accept: changing token/chat_id after live is running needs Stop → Start.
That is simple, and `retentionHours` works exactly the same way. Say so in the UI.

---

## C. Implementation order (6 steps)

> Run the six validations at the end of every step (`AGENTS.md` "Validation" section).

1. **Add `edit_message` to `telegram.rs`** — the same shape as `send_message`, with
   `message_id` in the payload. Tests: it reuses `interpret`, so no error-taxonomy test is needed,
   just the one payload-shape test
2. **Message formatter** — `format_sms_html(item)` and `format_batch_html(&[item])`.
   `escape_html` has to be applied to body/sender/port alike. OTP inside `<code>` (tap-to-copy).
   Whether to mask the sender with `logging::mask_number` **has to be decided** — the group is a
   trusted audience, so leaving it unmasked is appropriate, but write the difference from the log down in the doc.
   Tests: it is a rune-free pure function, so unit testing is easy (Myanmar text + markdown metachar cases)
3. **Forwarder thread + queue** (new `src-tauri/src/forwarder.rs`) —
   `mpsc::Receiver<ForwardJob>` · pacing · coalescing · `RateLimited` honoured · `Migrated`
   auto-heal (send the new chat_id back to the frontend in an event and have it saved).
   **Bound the queue depth** — unbounded means RAM grows while the network is down.
   Tests: split the pacing/coalescing decision out as a pure function that needs no channel
4. **Wire it into `start_live`** — a `telegram: Option<TelegramConfig>` argument, spawn the forwarder
   thread, `tx.send` in both branches of the `Sms` arm. In `stop_live`, drop the channel and join the thread
5. **The three switches + UI** (§D)
6. **README + a `03` case entry (if a bug turns up) + `05 §1.1` update**

---

## D. The three switches — must land **in the same change** as Stage 2

`04 §H` (inert-control rule): a control is wired to real behaviour inside the change that adds it,
or it is not added at all. In Stage 1 these three were **deliberately left out**.

| Field | Default | Where it has to be wired |
|---|---|---|
| `forwarding.enabled` | `false` | decides whether `start_live` passes `None` |
| `forwarding.forwardOtp` | `true` | the forwarder's filter (`item.otp.is_some()`) |
| `forwarding.forwardNonOtp` | **`false`** | promotional SMS would fill the 20/min budget immediately |

`deepMerge` iterates the stored keys (`03 §T1`), so a new field takes its default on an existing
profile — no migration needed.

---

## E. What must not be done (already decided — not to be reopened)

| Never do this | Why |
|---|---|
| Hook `sms:new` **alone** | §B.1 — concatenated/Myanmar-language messages disappear silently |
| `parse_mode: Markdown` | The body is attacker-controlled. A `*`/`_`/backtick in it gives `400 can't parse entities` → the OTP silently never arrives. Use HTML + `escape_html` |
| An HTTP call inside a live worker thread | §B.3 — `+CMTI` notifications get lost |
| A Rust-side config file / token persistence | the `sim_directory.rs:3-5` invariant · use the `start_live` argument |
| Putting a forwarder error into `live_error` | that field stands for the port's monitoring state |
| Raw HTTP errors in the log | The token sits in the URL path (`03 §18`) — always put it through `redact` |
| Re-enabling `reqwest`'s default features | `03 §18` — 22 crates + a cmake/C toolchain |
| An unbounded queue | RAM grows while the network is down |
| Announcing "it works even when you are away from the PC" without a tray | Closing the window ends the app. A tray is the prerequisite for that promise (`05 §A` — `minimizeToTray` was deleted because there is no tray code) |

---

## F. Hardware test — ✅ all four confirmed (evening of 2026-09-03)

| # | What had to be tested | Status |
|---|---|---|
| 1 | concat OTP — **only one** bubble in the group | ✅ **confirmed** — 21:59:26→34 carried **four** parts (`idx 4,5,6,7 [concat]`), `NEW SMS` fired **once**, and Telegram showed **one** bubble with the complete Myanmar text |
| 2 | Messages already on the SIM when live starts **must not be forwarded** (`Batch` arm) | ✅ **confirmed** (20:48 run) |
| 3 | Burst — coalescing | ✅ **confirmed** — the two OTPs at 22:01:31 + 22:01:44 entered the queue during an outage and arrived at 22:02 in **a single** `🔐 2 new messages` bubble |
| 4 | The queue keeps sending once the network drops and comes back | ✅ **confirmed** — 21:59:02 (`retrying in 5s` → the code, found (6 digits), arrived at 21:59) and 22:01:46→22:01:55→22:02:09 (`5s → 10s → 20s` exponential backoff, and then it arrived) |

**Multi-SIM is confirmed too:** the coalesced bubble showed `***972` (ttyUSB14) —
earlier runs had been `***573` (ttyUSB47).

---

## G. The two OTP false positives found in the field test — ✅ **fixed** (`03 §21`, `03 §22`)

**1. `2026` was read as an OTP.** At 21:59 a MyID **login notification** (not an OTP message at all)
was forwarded and the OTP badge showed `2026` — which is the **year out of the date `2026/09/03`**
inside the message.

**Fix:** the `decoder::in_date_or_time()` guard was added — if there is a 1–2 digit field on the
other side of a separator (`/ : - .`), then that run is a date/time field and not an OTP.
Because `extract_otp` uses `captures_iter()`, a date sitting earlier in the message does not hide
the genuine code that comes after it. Neither the gate nor the cascade was **touched**
(`05 §B.1` hard refusal). Four tests. Detail in `03 §21`.

**2. `3211` was read as an OTP (v1.6.1).** The following night at 01:12 a KBZPay **logout
notification** was forwarded and the OTP badge showed `3211` — which is the **KBZPay Call
Center number**. What opened the gate was the message's own "employees will never ask
for your OTP, PIN or NRC" warning — **the same shape** as §21.

**Fix:** the `decoder::after_phone_label()` guard (guard #2) — if the text in front of the number
**ends in** a phone label (call center · hotline · helpline · customer service · contact · call ·
dial · tel · phone · **`KW_PHONE`**, the Burmese word for "phone"), then that run is a phone number.
That it is a **suffix** match rather than a window, and that `.`/`,` are not treated as glue, is what
guards against false negatives. The cascade was not touched this time either. Four tests. Detail in `03 §22`.

