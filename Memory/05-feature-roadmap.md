# 🚀 Planned Features & Roadmap (Future Work)

> **Project:** SIM Bank SMS Reader (`sms-tauri`) · Repo: `RyanWez/sms-deliverer`  
> **Status:** Backlog / Future Enhancements (the list of features still to be built)
>
> This file has two parts: (1) the **Feature Backlog** below — ideas not yet implemented;
> (2) **§Settings Controls — Decisions Ledger** — the keep/delete/defer/refuse calls already made.
> **Do not reopen** anything in the ledger, least of all the two §B hard refusals.

---

## 📋 Feature Backlog List

### 1. Telegram Bot & HTTP Webhook SMS Forwarding *(High Priority / High Value)*
* **Description:** A system that forwards every new SMS or OTP the SIM bank receives immediately and automatically to a **Telegram Bot**, a **Discord Channel** or a **Custom REST API Endpoint (Webhook)**, so the operator does not have to be sitting in front of the machine.
* **What it would contain:**
  * A `Forwarding` tab in Settings.
  * Telegram Bot Token, Chat ID and Thread ID fields.
  * A Custom HTTP Webhook URL (POST JSON payload) with Header Authorization.
  * A choice of `Forward OTP Only` or `Forward All Messages`.
  * A retry mechanism (up to 3 automatic re-sends while the network is down).

#### 1.1 Status — **Stage 1 + Stage 2 are done · shipped in v1.6.0 · confirmed on hardware**. All that is left is the "What is left" list at the bottom (tray · webhook/Discord · thread id · disk-backed queue)

**Field verification (2026-09-03, bank plugged in over USB, `npm run tauri dev`):** Verify →
`@simbank_otp_bot`; Detect → `"Acti Chat" (-1004417127000)` supergroup (no warning — that the
warning does appear when it should had already been confirmed against the basic group
`-5258368202`); Send Test → the message really arrives in the group (`✅ SIM Bank SMS Reader /
Forwarding is configured. / Host: localhost`). Detect worked through the **`my_chat_member`
primary path** — without needing the `/start@…` fallback (the "You added SIM Bank OTP" update was
still inside the 24-hour window). `pick_group`'s `kind` classification, the HTML escaping and
`host_label()`'s fallback chain (`HOSTNAME` unset → `/etc/hostname`) were all confirmed in the field.

Decision: the destination is **a single private group**. Group membership is itself the allowlist —
the app stores **one** chat id and keeps no per-person list. So Settings needs no
person-management field at all (adding and removing people happens inside Telegram).

**Stage 1 — what is already in:**
* `src-tauri/src/telegram.rs` — the client (`build_client`, SOCKS5), `send_message`, `get_me`,
  `detect_group`, the error taxonomy (`SendError::{Migrated, RateLimited, Other}`),
  `redact`, `escape_html`. 20 tests
* `src-tauri/src/commands/telegram.rs` — three commands: `verify_telegram_token`,
  `detect_telegram_group`, `send_telegram_test`. These **do not check** `port_busy()` (they never
  touch the serial port — setup happens while the bank is live, so answering "Busy" would force
  the operator to stop monitoring). 5 tests
* Frontend: `settings.forwarding` (`botToken`, `chatId`, `proxyUrl`), the Settings page's
  `forwarding` group, three `api.ts` methods, `utils/telegram-preview.ts` (browser parity). 10 tests

**Three decisions (not to be reopened):**
1. **`parse_mode: HTML`, not Markdown.** The message body is attacker-controlled — anyone who can
   send the bank an SMS chooses it. If it carries `*`, `_` or a backtick, Markdown mode rejects the
   whole send with `400 can't parse entities` = **the OTP silently never arrives**. HTML needs only
   three characters escaped (`escape_html`), and `<code>` is tap-to-copy on every client
2. **Supergroup migration is auto-healed, and the operator is not told.** Telegram hands back the
   new id in `parameters.migrate_to_chat_id` — `send_telegram_test` retries **once** and the
   frontend saves the new id. Nobody has to be told "press Detect again"
3. **The token is plaintext `localStorage` — the UI must not pretend otherwise.** There is no
   keyring integration, and it is written up as it is. Because the token sits in the URL path,
   `redact` filters every error that leaves the module (see case §18)

**Stage 2 — ✅ done · real OTPs confirmed arriving on hardware · detail in `08`:**

> **Field verification ✅ done (2026-09-03 evening, 34 modems):** **all four** hardware tests
> confirmed — a single OTP · the `Batch` arm not forwarding · concat (4 parts → 1 bubble, Burmese
> text) · outage retry (5s→10s→20s backoff) · burst coalescing (`🔐 2 new messages`). Both the log
> masking and the token redaction held on a real failure. Detail in `08 §F`.
>
> **Two OTP false positives found in the field test — both fixed:** `extract_otp` read `2026` out
> of the date `2026/09/03` in a MyID login notification (fixed in v1.6.0, `03 §21`), and then
> KBZPay's logout-notification Call Center number `3211` (fixed in v1.6.1, `03 §22`), as OTPs. Both
> were fixed with a new guard — neither the gate nor the cascade was touched (§B.1 hard refusal).
* Live event hook — **both** `Sms` arms call `deliver`. The forwarder keys on `item.id` and turns
  the second visit into an `editMessageText` (it does not stack a new bubble)
* Coalescing queue — `forwarder.rs`: `MIN_INTERVAL` 3.5s (≈17/min, under the ceiling of 20),
  `MAX_BATCH` 10, `MAX_QUEUE` 500 (when it is full, drop **the oldest** — the one most likely to
  have gone stale), `retry_after` honoured, migration heal + the `forward:migrated` event
* Three switches — `enabled` / `forwardOtp` (default on) / `forwardNonOtp` (default **off**)
* `ForwardingConfigDto` arrives as a `start_live` argument (the `retention_hours` precedent) — so
  changing the token or the group needs **Stop → Start**, and the UI says so

**What is left:**
* Thread ID (forum topic) · a generic webhook + Discord · a disk-backed offline retry queue
* Tray icon and close-to-tray shipped in v1.8.0. `general.minimizeToTray` is wired to real behavior: closing the window (`X`, `Alt+F4`, or taskbar close) hides to the notification tray while live workers and Telegram forwarding continue unattended. The 2026-09-05 audit found three consequences and v1.8.1 fixed all three: the tray's *Quit* now winds the live session down and waits for the forwarder flush instead of `app.exit(0)`-ing over it (`03 T6`), `tauri-plugin-single-instance` stops a second launch contending for the ports the first copy holds and doubles as the way back to a window hidden on a shell with no tray host (`03 T7`), and the `tray-icon` Cargo feature is now declared rather than inherited (`03 T8`). The Linux blank-menu case behind the v1.8.0 follow-up fix is `03 §27`.

---

### 2. Interactive AT Command Console & Signal Strength Indicator
* **Description:** A terminal console inside each port's Detail Drawer for sending AT commands straight to that GSM modem and testing it, plus a display of signal quality.
* **What it would contain:**
  * An **Interactive AT Console** in the Port Detail modal (for example: `AT+CSQ` to check the signal, `AT+CUSD=1,"*124#",15` to check the balance, `AT+CPIN?` to check the SIM PIN).
  * Per-SIM signal strength (RSSI / dBm) shown on the Port Card as small green signal bars.
  * Quick AT Command Presets (sending the most frequently used commands with one click).

---

### 3. Message Batch Actions & Date-Range Filtering
* **Description:** Selecting many of the SMS in the Inbox and acting on them all at once, plus being able to search by date.
* **What it would contain:**
  * Multi-select checkboxes in the Message Table.
  * **Batch Actions:** Select All, Batch Delete Selected, Batch Copy Selected Text, Export Selected Only.
  * **Date-Range Filter:** easy filtering by "Today", "Yesterday", "Last 7 days", "This month" and "Custom Date Range (From - To)".

---

### 4. Modern Notification Chimes & Audio Themes
* **Description:** Playing a pleasant, modern notification chime (Pop/Bell/Chime) when a new SMS or OTP arrives.
* **What it would contain:**
  * A notification-sound on/off switch and a volume slider under Settings -> Notifications.
  * Selectable sound presets (e.g. `Modern Pop`, `Subtle Bell`, `Crisp Chime`, `Muted Click`).
  * A distinct, more noticeable sound for an OTP than for an ordinary SMS.

---

### 5. Quick Stats & Analytics Dashboard Overview
* **Description:** A dashboard giving an overview of the whole SIM bank's day-to-day activity in tables and graphs.
* **What it would contain:**
  * The total SMS received in a day and the comparison against yesterday (growth rate).
  * The Top 5 Active SIMs / Ports by SMS received.
  * A summary of the ports throwing errors (dead / timeout modems).
  * OTP Detection Success Rate (%).

---

# ⚖️ Settings Controls — Decisions Ledger (2026-08-30)

> Commits: `fbd7b8b` (11 fields deleted) · `f88d6d0` (`notifications.enabled` wired) · `10e0058` (leaving the Logs page when Developer Mode is switched off)
> **What this ledger is for:** the decisions below are **not to be reopened** in a later session.
> The rule itself is doc 04 §H — *never add an inert switch*.

## A. The 11 fields that were deleted

The whole `otp` group went, along with its `setOtp` setter. Because `otpPattern` was the only
`type: "text"` field, the Settings page's shared text-input branch was deleted with it.

> **Update — the text input branch is back (§1.1 Stage 1):** the `type: "text"` and
> `type: "secret"` branches were re-added to the Settings page — for the `forwarding` group's
> `botToken`, `chatId` and `proxyUrl`. This does **not** undo the `otpPattern` decision: what §B.1
> refuses is not a *text input*, it is an **operator-editable OTP regex that replaces the whole
> cascade**. All three fields ship attached to a button that invokes real behaviour
> (Verify / Detect / Send Test).

| Field | Former label | Why it was deleted |
|---|---|---|
| `general.minimizeToTray` | Minimize to System Tray | Previously inert and deleted; re-added and wired to real system tray & close-event minimization in v1.8.0 |
| `notifications.soundEnabled` | Play Sound | No audio pipeline. Already in the backlog as feature §4 |
| `notifications.desktopNotifications` | Desktop Notifications | No native notification path — see the §E feasibility study |
| `notifications.otpOnlyNotifications` | OTP Messages Only | Toasts fire for OTPs only anyway, so the semantics were themselves wrong |
| `otp.autoCopy` | Auto-copy OTP to Clipboard | No such feature — §D (dropped as a switch) |
| `otp.showInTable` | Show OTP Column | The OTP column in `MessageTable.svelte` is unconditional — it is always shown |
| `otp.highlightNewOtp` | Highlight New OTP | The highlight depends on `item.is_new` (`new-msg-highlight`) and never read the setting |
| **`otp.otpPattern`** | OTP Detection Pattern (Regex) | **Hard refusal — §B.1** |
| `appearance.compactMode` | Compact Mode | No CSS surface — §D (dropped) |
| **`developer.logLevel`** | Capture Log Level | **Hard refusal — §B.2** |
| `developer.maxLogs` | *(no label)* | Never rendered in the UI; the ring buffer is hardcoded on the Rust side (`MAX_RING_BUFFER = 1000`) |

The two inert fields that were left (`general.portRefreshInterval`, `developer.autoScroll`) were
**left deliberately** — see §C. (`portRefreshInterval` was **wired in v1.4.0** and is no longer
inert, §C.2. `autoScroll` remains inert as it was.)

## B. Hard Refusals — these two are **never to be added back** as editable fields

These are not "deferred for lack of time", they are **refused**. The reason for deleting them is not
implementation cost — it is that the control does more harm precisely when it really works.

### B.1 `otp.otpPattern` — an operator-editable OTP regex

OTP detection is `src-tauri/src/core/decoder.rs::extract_otp`, and it is not one regex but a
**keyword gate plus a four-step ordered cascade** (all of them `LazyLock<Regex>` statics):

1. `normalize_myanmar_digits()` — Myanmar digits (`U+1040`–`U+1049`) → ASCII
2. The `KEYWORD_RE` **gate** — `otp|one.?time|code|pin|verification|verify|confirm` plus the Myanmar
   keyword constants (`KW_KODE` = "code", `KW_CONFIRM`, `KW_SECURE` = "secure"). **No match returns
   `None` immediately**
   *(`KW_CONFIRM` had a spelling error — fixed in v1.4.0, doc 03 §T3)*
3. `P1` (4–8 digits within 24 chars of a keyword) → `P2` (digits + `is|as your|` / `KW_IS`) → `P3`
   (bare 6-digit) → `P4` (bare 4–8 digit) — this **order** is itself what holds precision up
4. Every match is then checked by **two guards** (filters, not patterns): `in_date_or_time()` for a
   date/time field (`03 §21`) and `after_phone_label()` for a call-centre/hotline number
   (`03 §22`). When a guard removes one, `captures_iter()` carries on to the remaining matches

The UI's own placeholder was `\b(\d{4,8})\b` — which is `P4` and nothing else, **with no gate**. If
the operator types something plain in there the keyword gate is gone, and promotional-SMS balances,
dates and fragments of phone numbers start matching as OTPs.

**The worst part: it fails silently.** No error is raised, the UI looks healthy, but **the wrong
number** lands on the clipboard — and the operator does not know they broke it themselves.

> If transparency is what is wanted: show the active patterns **read-only** (a list that cannot be
> edited), not an input field.

### B.2 `developer.logLevel` — a capture-level switch

Masking is **not at the sink** — `src-tauri/src/logging.rs`'s `mask_number()` / `otp_summary()` are
called by hand at **each individual** Info-level line. The capture gate is a hardcoded Info:
`capture_entry` (`level > Level::Info` → drop), both `Log::enabled` impls (ring buffer + file), and
`set_max_level(LevelFilter::Info)`.

Below that gate sit **unmasked** debug lines:

| Where | What is in it |
|---|---|
| `core/at.rs` `>> {cmd}` | Every AT command |
| `core/at.rs` `<< {preview(&text, 160)}` | The 160-char preview of every reply — for `AT+CMGL=4`/`AT+CMGR` that is **raw PDU hex**: the sender MSISDN + the message body (OTP included); for `AT+CUSD` it is the subscriber's own number |
| `core/at.rs` `++ {preview(&line, 120)}` | Unsolicited notification lines |
| `core/modem.rs` USSD reply body | The body of a USSD reply that would not parse — **deliberately moved to `debug!`** (the comment itself says "debug never reaches a sink") |

Lower the gate and all of it is written into both (1) the 1000-entry ring buffer that the Logs page
shows verbatim and (2) `app.log` — **which rotates only at 5 MB and is never aged out**, so it is a
file that outlives the inbox retention window (2 hours by default). The entire privacy masking
effort comes undone with one flick of a switch.

> If a debug mode is ever genuinely needed: **redact in the sink** (a PDU/number scrubber inside
> capture\_entry), not by exposing the debug lines that already exist.

## C. Deferred — in the **agreed order** (a later session continues in this order)

> **v1.5.0 status:** C.1 (theme) and C.2 (port auto-refresh) were **finished in v1.4.0**, and C.8
> (getting backend outcomes through to the operator) and C.9 (toast cap + coalesce) were
> **finished in v1.5.0** — which leaves **C.3 as the only** setting still to be decided.
> The finished ones are kept as history (delete them and the reason those blockers were blockers
> goes with them).
> **C.4–C.7 (L1–L4) are the limitations that were still open in v1.5.0** — of those four,
> **C.6 (L3) was closed in v1.6.2** (**shipped and released**, case `03 §24`), and
> **C.4 / C.5 / C.7 are still open** — re-checked against the v1.5.0 code, but with two things to
> watch:
> (1) the file:line evidence in those entries was **recorded against the v1.4.0 code** — #17–#20
> lengthened `src-tauri/src/commands/mod.rs`, so the line numbers have moved
> (`merge_ports` → `:187`, `live_status` → `:532`, `start_live`'s per-port spawn → `:1007`,
> the `Closed` arm → `:1184`) — find an item **by name**, never by number.
> (2) the only one whose substance actually changed is **C.4**: since #19, `live_error` carries
> across a refresh when the tty name matches (`mod.rs:236`), so the "the error text disappears"
> half is closed. But `live_ready` still does not carry when the name changes (`:221`), and the
> `Reconnecting` arm still looks the row up **by name** (`:1067`, lookup `:1072`), so **the symptom
> of a permanent `CONNECTING…` is not cured**.
> **v1.5.0's remaining PRs (#12/#13/#15/#18/#19/#20) are not backlog features, they are bug
> fixes** — which is why their detail is not in this file but in **doc 03** (cases §14–§17,
> traps T4/T5).

### C.1 ✅ **DONE (v1.4.0):** Theme Dark/Light was made to actually work

The control was already on the Settings page (System / Dark / Light), and **Light was a no-op**. It
was not one thing — there were **four separate blockers** (the ledger recorded three, which was
**incomplete** — the Logs console's hardcoded hex is the fourth, below), and the theme only arrived
once all four were resolved:

| # | Blocker | Former state | v1.4.0 resolution |
|---|---|---|---|
| 1 | **An incomplete class strategy** | `applyTheme()` (`src/lib/stores/settings.svelte.ts`) only added and removed the `dark` class — there was no such thing as a `light` class, and **not one `dark:` Tailwind variant is used anywhere in `src/`** (grep → zero). The components only use CSS-variable tokens such as `bg-surface` | `setResolved()` (`settings.svelte.ts:154`) writes both `dark` and `light` explicitly with `classList.toggle`, and sets `root.style.colorScheme` as well (for the native scrollbar and `<select>` popups) |
| 2 | **The light tokens were held hostage by the OS** | `src/app.css`'s light palette sat under `@media (prefers-color-scheme: light) { :root:not(.dark) { … } }`. On a machine whose OS is dark, choosing Light removed the `dark` class but the media query did not match — so `:root`'s dark tokens stayed and **nothing changed** | `src/app.css:21` `:root, :root.dark` and `src/app.css:48` `:root.light` each define **all 20 variables in full** (no partial override), so the palette depends **on the class alone**. `prefers-color-scheme` survives in app.css only as a comment — it is read in exactly two places (the flash guard + `applyTheme`), and only to decide which class to put on |
| 3 | **The shell pinned dark** | `index.html`: `<html class="dark" style="…color-scheme: dark">` plus an inline `<style>` carrying `html, body, #app { background-color: #171717 !important }`. Because of the `!important`, the app background could not be overridden with a token | `index.html`'s flash guard is now a **synchronous inline IIFE** — before any stylesheet or bundle loads it reads the theme out of `localStorage` `sms-reader-settings`, puts the `dark`/`light` class and `colorScheme` on, and on corrupt JSON or disabled storage leaves the dark default already declared on `<html>`. The pre-stylesheet paint colour is written for both `html` and `html.light`, and **the `!important` is gone** |
| 4 | **The Logs console hardcoded hex** *(the ledger recorded this as a "Bonus trap" — it was really a blocker)* | `src/lib/pages/Logs.svelte` had `bg-[#0d1117] text-[#e6edf3]` — **the only hardcoded hex in all of `src/`**. The log lines inside it use token colours, so under the light theme it was **light on light — completely unreadable** | Three tokens dedicated to the console were added (`--console-bg` / `--console-fg` / `--console-row-hover`, present in both themes); `Logs.svelte:223` uses `bg-[rgb(var(--console-bg))] text-[rgb(var(--console-fg))]`, and the row hover at `:242` uses `--console-row-hover`. The console deliberately does not follow `--surface` (it is a terminal surface — the GitHub canvas pair) |

> ⚠️ **Never delete `index.html`'s flash guard** — before Vite injects the CSS the webview shows the
> `<html>` default for a moment, and that is seen as a flash of light. In v1.4.0 it was
> **replaced, not removed** — the guard now picks the class according to the persisted theme. It
> **deliberately duplicates** `settings.svelte.ts`'s resolution logic (storage key, JSON shape,
> class names, `color-scheme`) because it has to run before the bundle loads.
> Change one side and change the other with it (`index.html`'s own comment says so).

**Two things to remember:**
- **The OS media listener existed all along — but was never unsubscribed.** v1.3.1 added one
  `addEventListener('change', …)` at startup with no `removeEventListener`, and covered for it by
  checking `theme === 'system'` inside the callback. The listener is now **only attached while
  System is the selection** — `detachSystemListener()` (`settings.svelte.ts:136`) detaches before
  every attach, which makes `applyTheme` idempotent (listeners cannot stack), and a user who pinned
  Dark/Light is no longer dragged along when the OS switches to dark in the evening
- On an embedded webview with no `matchMedia`, **dark is the fallback** (`settings.svelte.ts:173`) —
  not an OS-less light branch, because the shipped default is dark

### C.2 ✅ **DONE (v1.4.0):** `general.portRefreshInterval` — wired, and the `live_ready` trap closed

**How it used to be (kept for context):** the field existed in both `types.ts` and the Settings page
(default `30`) and no timer ever read it — it was left that way deliberately. The trap was this:
`refresh_ports` (`src-tauri/src/commands/mod.rs`) rebuilt a fresh `PortInfo` for every port with
**`live_ready: false`** (carrying only `checked`/`alive`/`iccid` over by stable `path`). So if the
background timer ran while live mode was on, **every LIVE badge would go dark** — at a moment when
the modems were perfectly healthy (it already happened on a manual Refresh).

**Now resolved (v1.4.0)** — the ledger said "one of the two", but in fact **both** were done:

| Layer | File | What happened |
|---|---|---|
| Timer | `src/App.svelte:78` `restartPortRefresh()` + `$effect` (`:99`) | Arms/re-arms whenever `portRefreshInterval` changes (no stacked timers; `stopPortRefresh` on unmount), and does not arm at all when `isTauri()` is false (there is no hotplug in the browser preview). Every tick is **skipped while any port operation is running** — `portsBusy()` (live/scan/USSD/delete) **plus `liveStore.detectBusy` separately** (`:92`, detect is not part of `portsBusy()`) |
| Clamp | `src/lib/utils/port-refresh.ts` `portRefreshPeriodMs()` | `MIN_PORT_REFRESH_SECONDS = 5` / `MAX_PORT_REFRESH_SECONDS = 3600`; `0` and everything non-finite/negative/junk → `null` = **off**. The ceiling is what guards against a corrupt value pushing the `setInterval` delay past 2³¹−1 ms and overflowing it into near-zero (a tight loop across 64 serial devices). A plain `.ts` with no runes and no Tauri import — `npm test` can exercise it |
| Diff / UI | `port-refresh.ts` `diffPorts` / `summarizeNames` / `describePortChanges` | The diff is based on the **device name** (not the index — the backend sorts by port number, so one stick appearing lower down shifts every index; and not `path` either — that a replug changed the tty node is itself something the operator has to know). Appeared = Success, disappeared = Warning, both = Info, **silence when nothing changed**, and `… and N more` past three names |
| Merge | `src-tauri/src/commands/mod.rs:143` `merge_ports(enumerated, old, sim_dir, live_session)` | `refresh_ports` (`:194`) now calls a pure function — the rules can be unit-tested without a real `/dev/serial/by-path` (6 tests from `:1675`) |

**How the `live_ready` trap was actually closed:** the badge means "a worker is sitting on this port
right now", so a refresh only leaves it alone when **all three** of the following hold
(`mod.rs:177`):

1. **A live session must still be running** — `st.live_on || st.live_stop.is_some()` (`:203`), the
   same window `port_busy()` defines as ports-held. No session, no worker
2. **The port must still be in the enumeration** — a vanished port has no entry in this list, so
   when it comes back it starts from a fresh `false`
3. **The tty name behind the stable path must not have changed** (`p.live_ready && p.name == name`)
   — a live worker holds **the name it was spawned with for its whole life** and reopens that same
   name after an outage, so a renumbered stick keeps its path but no longer has a worker — carrying
   the badge over would make it lie

State is carried over **by stable `path` only** (never by name) — the name is the thing a replug
reshuffles, so matching on it would put one stick's liveness and card onto another stick.

**A matching ICCID is deliberately not one of the conditions:** `refresh_ports` **never opens a
port**, so the ICCID it reports is either copied over from the old entry or the SIM directory's
"last seen in this path" hint. All it can change, then, is `None` → one kind of hint, and that is
**not evidence that the card changed** (it may well be the one from the earlier session). What
catches a real swap is the tty-name check. Test:
`mod.rs` `a_slot_hint_filling_in_an_unknown_iccid_leaves_the_badge_alone`.

`live_error`, on the other hand, is `None` after every refresh (`mod.rs:187`) — that is the root of
L1; see §C.4.

### C.3 `developer.autoScroll` — still inert, **still undecided**

The Logs page has its own working session-local toggle (`logsStore.autoScroll`, default `true`,
`src/lib/stores/logs.svelte.ts`) and never reads the setting. Wiring it is not the hard part — the
store would be **seeded** from settings — the decision still to be made is whether the toggle stays
session-only or writes back into the setting (the owner has not decided).

---

**C.4–C.7: four limitations checked against the v1.4.0 code and deferred (L1–L4)** — these are not
bug-free "things not built yet", they are **holes in behaviour that already exists**. Each one
carries file:line evidence and the symptom the operator sees.

### C.4 (L1) Live mode never picks up a **stick that comes back under a different name**

- **Evidence:** a worker holds the tty name it was spawned with **for its whole life**
  (`src-tauri/src/core/live.rs:176` — the reconnect loop reopens that exact name with
  `AtChannel::open(port_name)`). When the name has changed, `merge_ports` carries neither
  `live_ready` (`src-tauri/src/commands/mod.rs:226`, gated on `p.live_ready && p.name == name`) nor
  `live_error` (`:241`, gated on the same name equality) onto the new row — so the row comes back
  looking untouched and `portStatus` **falls through to CONNECTING** for as long as live mode is on
  (`src/lib/utils/port.ts:63`)
- **Worse:** the orphaned worker's `Reconnecting` event looks its row up **by name**
  (`src-tauri/src/commands/mod.rs:1141` arm, lookup at `:1146` `find(|p| p.name == port)`) — and
  since no row carries that name any more, **it cannot even write an ERROR**
- **Symptom:** the operator plugs the stick back in and the card stays at `CONNECTING…`
  **forever** — no error, no ERROR badge, no message
- **Workaround:** **Stop → Start** live (`start_live` re-spawns fresh workers for whatever names are
  checked at that point)
- **More visible since v1.4.0:** auto-refresh re-enumerates on a timer, so a state that used to
  need a manual Refresh now arrives on its own
- **History — closed in v1.5.0 (#19), do not read it as current behaviour:** this entry used to say
  that `merge_ports` wrote `live_error: None` for every port on every refresh (`mod.rs:187` in the
  v1.4.0 code), which wiped a "Reconnecting: Port lost: EIO" or "Serial I/O failed: …" within one
  refresh interval on **every** port, renamed or not — and it never came back, because `OutageLatch`
  emits one event per outage rather than repeating. That half is fixed: the carry is now gated on
  the tty name (`:241`), and nothing here has to expire it, because `start_live` and `stop_live`
  clear every `live_error` at their boundaries and `detect_ports` overwrites it per port. What
  stays open is only the renamed-stick case above — where that same name gate is exactly what drops
  both fields

### C.5 (L2) `start_live` has no thread pool and no stagger — it spawns as many workers as there are checked ports

- **Evidence:** the `for port in ports` loop at `src-tauri/src/commands/mod.rs:838` does one
  `thread::spawn` per port (`:842`), and `ports` comes out of the `p.checked` filter (`:803`). There
  is **no** semaphore and no worker cap
- **Compare:** every other port-heavy path respects the cap — `detect_ports` (`:277`,
  `MAX_CONCURRENT_PROBES = 32`), and `start_scan` (`:475`), `get_sim_numbers` = USSD (`:615`) and
  `cleanup_sim_storage` (`:1351`), the three of them on `MAX_CONCURRENT_PORTS = 16` (the constants
  at `:84` / `:90`)
- **Symptom:** starting live on a 64-slot bank puts **64 AT conversations at once** on one USB
  bridge
- **⚠️ Not benchmarked** — what was verified is the **concurrency shape** (that there is no cap);
  the effect on real throughput or failure rate has not been measured

### C.6 (L3) ✅ **DONE (v1.6.2):** `LiveEvent::Closed` now drops the port from the ready list — a `failed` bucket was added

**Status:** implemented, validated and **shipped in v1.6.2** (`fix: report what the status lines and
logs actually count (#28)`, tag `v1.6.2`). The case entry (symptom → root cause → fix) is
**`03 §24`**, and the plan item is `07 §D.3`.

**How it used to be (kept as historical evidence — the line numbers were recorded against the
v1.4.0/v1.5.0 code):**

- **Evidence:** the `Closed` arm (`src-tauri/src/commands/mod.rs:1019`) wrote `p.live_ready = false`,
  set `p.live_error`, did `st.live_failed.push(...)` and wrote `status_text` as
  `"{port} FAILED: {e}"` — but **never did `st.live_ports_ready.retain(...)`**, and never called
  `live_status()` again either
- **Compare:** the `Offline` arm did `st.live_ports_ready.retain(|p| p != &port)` (`:886`) and then
  recomputed `live_status(&st, port_count)` (`:890`)
- **Symptom:** after a worker crashed the card showed ERROR, but the next time the status line was
  recomputed (on any later `Ready`/`Offline` event) `live_ports_ready.len()` was still counting the
  port that had dropped out — so **"Live x/y ready" read high and the "connecting…" count went wrong
  with it** (`live_status` at `:395`)

**How it was closed (v1.6.2):** rather more than adding one `retain` line — `live_failed` was turned
from **write-only into a counted bucket** (an `N failed` clause in `live_status`,
`commands/mod.rs:543`), and the bucket rules were pushed into a single `mark_port_failed` (`:579`):
per-port dedup (a panic can be reported twice) plus removal from `live_offline` ("failed" beats "no
modem"). Because `connecting…` is not a bucket but the **remainder**, the three buckets have to stay
disjoint — that is now a backend invariant in AGENTS.md. The hand-written `status_text` overrides in
three arms (`Closed`, the outer `catch_unwind`, `Reconnecting`) were deleted. **A dead worker is
deliberately not counted as `no modem`** — that was the call this entry had to make. 5 tests
(`mod.rs:2167`–`:2250`), no hardware needed.

### C.7 (L4) There is **no liveness re-probe** inside the live monitoring loop

- **Evidence:** `probe_channel` runs **once** per (re)connect (`src-tauri/src/core/live.rs:201`). The
  inner loop (`:327`–`:370`) does exactly four things: drain the `+CMTI` queue / `handle_cmgr`,
  `flush_stale` a stalled concat group, check `ch.is_dead()`, and the `SIM_SWEEP_EVERY` (600 s)
  retention sweep — **it never issues an AT to re-check the modem**
- **Symptom:** in the state where the tty node is still open but the modem has stopped answering AT,
  the badge **stays green LIVE while not one message arrives** — `is_dead()` only catches
  channel-level errors, so it does not catch going quiet
- **The 60 s `OFFLINE_RETRY` re-probe does not help** (`live.rs:53`, `:210`) — that is only for the
  **branch where the probe has already failed** (a port that has entered the Offline latch), not for
  a port that already reached Ready

### C.8 ✅ **DONE (v1.5.0):** Getting backend outcomes through to the operator — the broken half of the event contract

**Problem:** Rust emits 18 kinds of event and the frontend listened to only 16 — `export:saved` and
`sim_cleanup:done` had **no listener at all**. And there were two more cases where it listened but
**hid the answer**: `delete:done` had an empty payload, and `live:reconnecting` was only a
`console.warn`. So:

| Case | What the operator saw (before) |
|---|---|
| Export succeeded | **Nothing** — only the "Choose a location…" toast. Indistinguishable from a cancel |
| Delete: 2 of 10 slots went | **Identical to a clean success**. Only the 8 rows still there told them |
| Port 7 went quiet | `console.warn` — **invisible**, since devtools are disabled in a packaged build |
| SIM cleanup failed on 3 ports | Only in the Ports page footer. Invisible from the Inbox |

**Root cause:** `liveStore.statusText` holds every one of the backend's long-running outcomes, but it
was rendered in **exactly one place, `Ports.svelte:547`** — and the operator is on the Inbox for all
of these events.

**Fix:**
- Give `delete:done` a payload: `{ requested, freed, removed, kept, failed_ports }`. **The display
  string is not re-parsed on the frontend** — the contract is made explicit. `kept > 0 ||
  failed_ports > 0` is a `Warning` toast, otherwise `Success`
- Add the `export:saved` + `sim_cleanup:done` listeners
- Make `live:reconnecting` a `Warning` toast (the `detect:done` shape). **`live:offline` stays
  console-only** — a slot with no SIM is not an incident, which is what the code comment says too
- Add a `page-footer` to `Inbox.svelte` (reusing the `Ports.svelte` markup, no new tokens) — message
  count + `statusText`
- Preview parity: `deleteSelected`'s non-Tauri branch has to toast as well

**Send every toast through the `toast` wrapper that already exists in `api.ts`** — do not add a new
id counter. `ToastContainer.svelte:21` keys on the id with `{#each toasts as t (t.id)}`, so two
overlapping counters make the duplicate key throw (there are three counters already: `api.ts:22`
from 1, `logs.svelte.ts:6` from 1000, and `updater.ts:45`).

**The work this left behind — now fixed (see C.9):** the toast array had no cap
(`live.svelte.ts:13`) and did not coalesce. Adding the `live:reconnecting` toast only brought that
**closer**.

### C.9 ✅ **DONE (v1.5.0):** Bounding the toast column and merging identical notices

**Problem:** `addToast` was `toasts = [...toasts, t]` — no cap, no dedupe. A toast lives 4 seconds and
`.toast-container` is a fixed bottom-right column with no `max-height` (`app.css:258`). So **16 ports
failing at once stacked 16 cards up past the top of the viewport and covered the very UI that was
reporting the failure**

**Fix — `src/lib/utils/toast-queue.ts`** (rune-free and `$lib`-free, so the Node runner can import it
directly — the `csv.ts`/`port-refresh.ts` precedent):
- `MAX_TOASTS = 5`, and `pushToast` keeps the newest with `slice(-MAX_TOASTS)`
- A matching `kind` + `title` merges onto one card and increments `count`, so the title becomes
  `Port lost (16)` (`countSuffix`)
- **A toast carrying an `otp` is never merged** — each code is a distinct thing the operator came to
  read, and merging would silently lose one
- The body takes the **newest** and is not merged — a body concatenating 16 port names cannot be read
  inside a 4-second card, and the count shows the scale already
- A coalesced card is moved to the end of the array — a repeat is better re-announced at the bottom
  than written as an update to a card the eye has already left

**The 4-second timer:** `setTimeout` keys only on the id it was scheduled for. A coalesced card gets
a new id, so the timer already in flight finds nothing to remove and the merged card gets its own
full 4 seconds — **the notice for a port that is still flapping stays up**, which is the intent

**13 tests** (`toast-queue.test.ts`) — the cap, coalescing, OTPs not merging, a differing title/kind
not merging, immutability, and that coalescing below the cap does not lose another card

**Confirmed in preview:** 10 Refreshes back to back → **1** card, `Refreshed (10)`, with the
container height stopping at 127px (it would have queued 10 cards before). The cap itself cannot be
reached in preview — the synthetic app only produces two distinct titles — which is what the unit
tests catch

### C.10 Merging the four supervisors into a `run_port_pool` — **deliberately deferred** (not a bug)

The **one code refactor** that was considered for v1.5.0 and agreed to be left alone. It used to be
only a passing note under doc 03 §T5 Rule 3; it is raised to an entry here.

**The shape repeats (= duplication debt):** the four port-heavy commands have the same structure — an
`Arc<Mutex<Vec<String>>>` work queue + `take_port` + a worker cap + a per-port `catch_unwind` + a
supervisor that joins, clears the busy flag, builds the status line and emits `*:done`:

| Command | supervisor | worker | per-port `catch_unwind` | cap |
|---|---|---|---|---|
| `detect_ports` | `src-tauri/src/commands/mod.rs:381` | `:396` | `:398` | `MAX_CONCURRENT_PROBES` = 32 (`:139`) |
| `start_scan` | `:675` | `:684` | `:691` | `MAX_CONCURRENT_PORTS` = 16 (`:133`) |
| `get_sim_numbers` (USSD) | `:814` | `:828` | `:830` | `MAX_CONCURRENT_PORTS` |
| `cleanup_sim_storage` | `:1698` | `:1714` | `:1716` | `MAX_CONCURRENT_PORTS` |

The live supervisor (`:1062`, per-port spawn `:1081`, `run_live`'s `catch_unwind` `:1334`) has **no**
cap (§C.5), so a merge would pull it into this pool as well — which is the pull towards resolving
C.5 at the same time.

<!--CHUNK-->










