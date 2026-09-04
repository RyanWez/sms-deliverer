# 🗺️ Next Release Plan — the programme drawn out of the v1.5.0 field test

> **Basis:** the log of a **session an operator ran themselves on a real 64-port SIM bank**
> with v1.5.0 (tag `v1.5.0`, signed installer verified, 7 PRs merged).
> **Purpose:** this doc is not a backlog — it fixes **the contents of the next two releases
> (v1.6.2 / v1.7.0)**, every item paired with field evidence. The feature backlog and the
> deferred order live in `05-feature-roadmap.md`, the bug casebook in `03-troubleshooting.md`.
> **Every file:line proof was checked against the v1.5.0 code** — look an item up by name,
> not by number (`commands/mod.rs` grows longer with every change).
>
> **⚠️ 2026-09-04 — the version numbers have moved** (the doc was written saying `v1.5.1` / `v1.6.0`)
>
> This doc was written just after v1.5.0 and named the next two releases `v1.5.1` and
> `v1.6.0`. Those two numbers have since been taken by **other work**:
>
> | Already shipped | What it contained |
> |---|---|
> | **v1.6.0** (2026-09-03) | Telegram group forwarding — Stage 1 + Stage 2 (`08` doc) |
> | **v1.6.1** (2026-09-03) | a call centre number must not be read as an OTP (`03 §22`) |
>
> So the items in this doc are now **`v1.6.2` (four fixes) and `v1.7.0` (the live command
> mailbox)**. The numbers below have been corrected. The next time a release shifts, fix this
> box with it.
>
> **📌 2026-09-04 (latest status) — the four v1.6.2 fixes have shipped:**
>
> | Item | Status | Case entry |
> |---|---|---|
> | §C cleanup status line (`empty` counter) | shipped in v1.6.2 ✅ | `03 §23` |
> | §D.1 `msg(s)` → `slot(s)` unit | same ✅ | `03 §25` |
> | §D.2 the form marker on the USSD rejection line | same ✅ | `03 §26` |
> | §D.3 `Closed` ready count + `failed` bucket | same ✅ | `03 §24` |
> | §B live worker command mailbox (v1.7.0) | **not started** | — |
>
> `main` carries `fix: report what the status lines and logs actually count (#28)` and
> `chore(main): release 1.6.2 (#29)`, and the tag `v1.6.2` exists — release-please bumped the
> version once the PR was merged (`02` doc). Every "v1.6.2" in this doc therefore means the
> **released** version. It has still **not** been re-checked on the physical bank; unit tests were
> judged sufficient for these four (see the 🎯 table).
>
> **Three things the review added beyond the plan** (things the plan did not foresee):
> 1. **the `failed` bucket** — §D.3's suggestion on its own (`retain` + rebuilding
>    `live_status`) would drop a dead worker into the `connecting…` remainder, which is
>    **swapping one wrong number for another wrong number** — the line would claim "still
>    connecting" when in truth nothing is retrying any more. So `live_failed`, which had been
>    write-only, was promoted to a counted bucket and the rules (dedup + eviction from the
>    bucket) were pushed into the `mark_port_failed` helper

> 2. **five `slot(s)` unit sites live outside `modem.rs`** — the plan noted only `modem.rs`,
>    whereas there are in fact three in `commands/mod.rs` (cleanup status, the per-port cleanup
>    log, `Deleted N slot(s) from PORT`) plus two in `core/live.rs` (the retention sweep)
>    (`03 §25`)
> 3. **the `Reconnecting` arm had to be fixed for the same reason** — §D.3 wrote
>    `Offline`/`Reconnecting` up as one pair, but rebuilding the status line from `live_status`
>    was **`Offline` alone**; `Reconnecting` wrote `status_text` by hand and so did not reflect
>    the buckets. It now drops the port from the ready list and rebuilds the line, and **enters
>    no bucket** (= the `connecting…` remainder, which is the true state)

---

## A. v1.5.0 Field Verification — what is proven and what is not

### A.1 ✅ PR #12 (`fix: give live-mode messages their real SIM slot index`) — **confirmed on hardware**

This is the bug in doc 03 §14. In v1.4.0 `parse_cmgr` / `parse_pdu_cmgr` hardcoded `index: 0`,
so every message arriving over the live path (`+CMTI` → `AT+CMGR`) carried slot 0 and a delete
sent `AT+CMGD=0` — and because SIM slots start **at 1**, and because the confirmation is
absence-based, that read back as a **false success**: "it is gone".

**Evidence 1 — the slot number really does come through** (in v1.4.0 it was always `idx 0`):

```
18:40:04.539  COM38: live SMS read (idx 4) [concat]
              COM38: live SMS read (idx 5)
              COM38: live SMS read (idx 6)
              COM38: live SMS read (idx 7)
              COM38: live SMS read (idx 8) [concat]
              COM38: live SMS read (idx 9) [concat]
```

Log site: `src-tauri/src/core/live.rs:517` (concat suffix `:520`) and `:535`.

**Evidence 2 — a single-slot message that arrived over live really did leave the SIM:**

```
18:46:47.485  COM38: initial batch 6 msg(s)
18:47:07.303  Live stop requested
18:47:15.819  COM38: deleted 1 msg(s)
18:47:15.825  Deleted 1 message(s) (1 SIM slot(s) freed)
18:47:20.214  COM38: initial batch 5 msg(s)
```

**What matters here is that `initial batch` is not the UI's own count** — it is **the card's
own answer** to the `AT+CMGL=4` the live worker sends after opening the port (`live.rs:250`
list → `:292` log). So `6 → 5` is not "a row disappeared", it is the proof that **the SMS
really did leave the SIM**. On v1.4.0 this line would have stayed at `6` — which is exactly
§14's real symptom.

**Evidence 3 — a concat that arrived over live (two parts, slots 8 + 9, KBZPay) was
reassembled and yielded a 6-digit OTP:**

```
18:46:14.954  NEW SMS on COM38: from=***Pay otp=found (6 digits)
```

Log site `src-tauri/src/commands/mod.rs:1123` — the sender passes through `mask_number` and the
OTP through `otp_summary` (AGENTS.md Logging rule). **No unmasked number or OTP may ever be
written into this doc** — the log lines above are already masked, which is why they are copied
verbatim.

**Evidence 4 (from early in the session) — deleting a concat row that came from the scan path
confirmed both slots and the card was empty on the re-scan:**

```
COM17: pdu-mode read -> 0 msg(s)
Deleted 1 message(s) (2 SIM slot(s) freed)
```

### A.2 ⏳ Not yet exercised — **deleting a concat row that arrived over live**

Removing two slots at once **from the live path** has **not been done on hardware**. Each half
has its own evidence (A.1 evidence 2 = live plus a single slot, evidence 4 = a two-slot concat
but over the scan path) — but **never both within one operation**. Whether `part_indices`
really collects the actual slots is only unambiguously visible in that combination. Once §B's
mailbox work is done, this case has to be added to the `04 §G` playbook and exercised there.

### A.3 ⚠️ The parts the field run did **not** put under pressure (shipped, but not field-proven)

| What shipped | What happened in the field |
|---|---|
| `BusyGuard` (#18, `commands/mod.rs:103`, `Drop` `:117`) | not a single panic occurred — so the guard's `Drop` path **never fired once**. Whether "Busy" never got stuck because the guard works or because nothing panicked is **impossible to tell apart** |
| the live worker's `catch_unwind` (`core/live.rs:137`, `WORKER_PANIC` `:56`) | not one `Worker crashed` line — equally unproven |
| `ProbeVerdict::Inconclusive` (#19, enum `commands/mod.rs:306`, `of` `:319`) | **never appeared once**. All 30 dead ports went `NOT_RESPONDING` → `Empty`: `Detect done. Modems found: 34/64 \| 30 port(s) with no modem deselected` (status builder `:514`; an `Inconclusive` would have added the further clause at `:520`–`:525` — it did not appear). So **only the `Empty` route is field-proven**, and there is still no evidence that doc 03 §16 will not come back |

**What that means:** none of these three may be recorded as "tested". A step that deliberately
manufactures EBUSY / ModemManager contention needs to go into the `04 §G` playbook.

---

## B. ISSUE 1 — with live mode on, Delete / Clear All / Get SIM Numbers cannot be run

**→ the headline item of the next release (v1.7.0).**

- **Symptom:** with live running the operator selects a row — `Delete Selected` is
  **disabled**. `Clear All` and `Get SIM Numbers` likewise. They only come back once live is
  stopped
- **Not a v1.5.0 regression** — v1.4.0 behaved the same way. But **#12 is what made it
  matter**: deleting a live message used to look as though it worked while in truth nothing
  happened at all, so **nobody ever asked "when am I allowed to delete?"**. Now that it
  genuinely works, that question has become a real one

### B.1 Mechanism — three layers

| Layer | Evidence | What it does |
|---|---|---|
| Frontend | `src/lib/components/Toolbar.svelte:10`–`:16` — the `busy` derivation contains **`liveStore.on` (`:11`)** | `Delete Selected` (`:109`), `Get SIM Numbers` (`:92`), `Clear All` (`:135`) and `Scan & Read All` (`:66`) are all `disabled={busy}`. Only the `Live Mode` button has a condition of its own (`:80` — no `liveStore.on` in it) |
| Backend gate | `AppStateInner::port_busy()` (`src-tauri/src/commands/mod.rs:53`, `live_on` `:55`) | even with the button re-enabled, the first check in `delete_selected` (`:1331`) at `:1338` returns `Err("Busy")` |
| **the real root cause — physical** | `modem::delete_messages` (`src-tauri/src/core/modem.rs:566`) **opens the port itself** (`at::AtChannel::open`, `:567`) | the live worker is **already holding** that port. On Windows a COM port cannot be opened twice — so lifting the UI gate would only earn an "Access denied", not a delete |

So this is not a UI bug — it is **the hole left in the architecture**. `Get SIM Numbers`
(`modem::get_sim_number` `:642`, open `:643`) and `Clear All` (the same `delete_messages`) are
shut out by that one single cause as well.

### B.2 What the current workaround (Stop Live → Delete → Start Live) costs — three things

1. **Restarting live re-reads every port with `AT+CMGL`** (`core/live.rs:250`, text fallback
   `:256`, 15 s timeout) — visible in the log as the `initial batch` lines, **34 ports' worth**
   on this bank. One delete backfills the entire bank
2. **An SMS that arrives between Stop and Start does not come through `+CMTI`** — it lands in
   the initial batch, and because that is the first connect it enters with `is_new: false`
   through `LiveEvent::Batch` (`live.rs:310`, handler `commands/mod.rs:1087`) — so **no live
   badge and no OTP toast**. The operator sitting there waiting for the OTP has already
   received it, silently
3. **`stop_live` does not finish immediately** — `stop_live` (`commands/mod.rs:1261`) clears
   `live_on` at once (`:1266`), but the workers hold their ports until the supervisor joins
   them, and a worker parked in `AT+CMGL=4` (15000 ms timeout) can take **~15 s more**.
   `port_busy()`'s `live_stop.is_some()` (`:56`) exists for exactly that window. In the field
   log too, `18:47:07.303 Live stop requested` → `18:47:15.819` delete took **~8.5 s**

### B.3 Proposed solution — give every live worker a **command mailbox**

**The idea:** rather than serving a delete request by opening the port, **queue it to the worker
that owns that port**. In between its `+CMTI` polls the worker runs `AT+CMGD` plus the existing
`modem::delete_confirmed` **on the `AtChannel` it is already holding** and sends an `OpResult`
back.

**Where it goes:** the monitoring loop `core/live.rs:342`–`:385` — today it pulls a `+CMTI`
index off `queue` (`:340`) at `:343`, and when there is none it waits 500 ms in
`ch.wait_notification(500)` (`:347`). The mailbox check belongs between those two.

**Three merits of this design:**
- **no new lock is needed** — the worker already owns its channel and the mailbox is just one
  `mpsc::Receiver`. Two threads never touch the channel
- **the confirmation path needs no change** — `modem::delete_confirmed`
  (`src-tauri/src/core/modem.rs:477`) accepts an already-open channel, and `live::sweep_expired`
  (`core/live.rs:443`, called at `:463`) is **a precedent already demonstrating this pattern** —
  a live worker doing a confirmed delete on its own channel has existed since v1.5.0. What is
  left is only "getting the operator's request to that place"
- **`Clear All` and `Get SIM Numbers` are solved at the same stroke** — the mailbox only has to
  carry three message types

**Risk:** it **changes the timing of the live loop** — `AT+CMGD` plus the confirming `AT+CMGL`
(15 s timeout) can hold up the `+CMTI` poll, so notifications can back up in the queue while a
mailbox item is being serviced. Therefore:
- this is **a change that earns its own `Memory/03` case entry** (symptom → root cause → fix)
- the **`04 §G` Hardware Live-Check Playbook has to be run** — do not merge without the bank
  attached. The combination A.2 has not exercised (live + a two-slot concat) is one step of that
  playbook run
- when the backend gate is opened up, `port_busy()`'s check of `live_on` **cannot be deleted** —
  it may only be bypassed for the commands that have a mailbox route (scan opens the port
  itself, so it stays blocked)

**Version framing:** a new capability → `feat:` commit → **v1.7.0**.

---

## C. ISSUE 2 — `SIM cleanup done. Deleted 14 | FAILED: 30/64` is misleading

- **Field evidence:** cleanup was run **across all 64 ports** before Detect. Of those,
  **30 had no modem at all** (Detect reported `34/64` later on) and each one logged
  `Modem not responding`. **The real failure count is zero** — and yet the status line said
  `FAILED: 30/64`. For an operator standing in front of the bank debugging an incident that is
  **an extremely alarming line** — it says 30 things failed when nothing failed at all
- **Root Cause:** the cleanup worker's failure counter (`src-tauri/src/commands/mod.rs:1591`–
  `:1598`) increments `failed` for **every non-ok result out of `expire_old`** (`:1592`),
  **exactly like** the panic arm (`:1599`–`:1602`) — so an empty slot with no modem cannot be
  told apart from a genuine failure. `modem::expire_old` (`src-tauri/src/core/modem.rs:874`)
  returns the error as it is when `read_port` is not ok (`:876`–`:883`), and since probe silence
  is `NOT_RESPONDING` the code **already knows** this is "no modem" — it is only the counter
  that does not distinguish. On top of that the status line is built out of that single number
  (`commands/mod.rs:1618`–`:1622`; the event payload `:1627`–`:1630` likewise carries only
  `deleted`/`failed`)
- **For comparison:** since #19 `detect_ports` separates `Empty` from `Inconclusive` and says
  **precisely** `30 port(s) with no modem deselected` (`detect_done_status`
  `commands/mod.rs:514`, verdict enum `:306`). So **the correct shape already exists in this
  project** — cleanup simply does not follow it
- **This is the first symptom of `05 §C.10` (four supervisors, four different panic/failure
  accounting policies) to show up in the field** — that entry's table already recorded cleanup
  as "increments the failure counter", but this is the first time it reached an operator. Fixing
  it is therefore **not a detour, it is a down payment on C.10** — it hardens the point that the
  policy has to be settled per command
- **Suggestion (shape):** separate "no modem" from "failed" the way detect does — for example
  `SIM cleanup done. Deleted 14  |  30 empty  |  FAILED: 0`. One more counter (`empty`) plus a
  check of `probe_failure`/`NOT_RESPONDING` plus the status-line wording — **this is not a
  behaviour change**, so `fix:` commit → **v1.6.2**. Adding `empty` to the `sim_cleanup:done`
  payload as well lets the UI toast match

---

## D. Three small items — **all inside v1.6.2 alone**

### D.1 The single unit `msg(s)` stands for two different things

This is how it looks in the field log — as though the numbers do not add up:

```
COM39: pdu-mode read -> 2 msg(s)
COM39: deleted 5 msg(s)
```

- **Read side (`src-tauri/src/core/modem.rs:379`, text-mode `:400`):** `msgs.len()` is
  **the number of rows after reassembly** — the `Reassembler` merges the concat fragments into
  one before anything is counted (`:366`–`:378`)
- **Delete side (`src-tauri/src/core/modem.rs:543`, partial-failure form `:551`–`:557`, the
  form for an unconfirmed delete `:524`–`:528`):** `gone.len()` is **the number of SIM slots** —
  two rows can be five fragments
- Because both print `msg(s)`, going from `2` to `5` reads like an error — when in fact it is
  correct. Note that the status line already keeps them apart:
  `Deleted 1 message(s) (1 SIM slot(s) freed)` (`commands/mod.rs:1436`) writes the two units as
  two different things — **it is only the per-port log line that was left behind**
- **Suggestion:** change the delete lines to `slot(s)` (`deleted {} slot(s)`). It is wording
  only, so `fix:`. **README sync:** these log formats (`… read -> N msg(s)`,
  `deleted N msg(s)`, `SIM cleanup done…`) are **not in** README.md (grepped — README holds only
  the probe timeout, the timeout-chain table and the AT flow), so neither §C nor §D needs a
  README change — but check again at the time of the change (AGENTS.md Documentation duty)
- **⚠️ Correction (2026-09-04, only found after actually doing the work):** the "no README change
  needed" above **is true for §C / §D.1 / §D.2 only** — the per-port log formats really are
  absent from README (re-grepped). But **for §D.3 there is one**: README.md's Live SMS
  Monitoring bullet (line 62) said `NO MODEM` was the **only** thing excluded from the ready
  total, so adding the `failed` bucket made it incomplete — it has now been synced to include
  `failed` and the `connecting…` remainder as well. **Lesson:** README does not hold log
  **formats**, but it does hold **the arithmetic of the behaviour (which ports count in the ready
  total)** — adding one new counter can falsify that sentence

### D.2 The USSD rejection warning shows only the **code** — not the **command**

```
18:39:31.846  COM38: USSD *88# rejected (+CME ERROR: 100)
18:39:31.861  COM38: USSD *88# rejected (+CME ERROR: 100)
18:39:36.585  COM38: SIM number ***573      ← *124# fallback succeeded
```

- **Two lines identical to the letter**, 15 ms apart — it reads like a duplicated log. In truth
  they are **two genuinely different attempts**: `AT+CUSD=1,"*88#",15` first, then the session is
  cancelled with `AT+CUSD=2` and the **bare form `AT+CUSD=1,"*88#"`** is retried
  (`src-tauri/src/core/modem.rs:804` first attempt, `Rejected` arm `:807`, `AT+CUSD=2` `:808`,
  bare retry `:809`)
- **Root cause:** `ussd_attempt` (`:817`) takes both `command` and `code` in its signature but
  logs **only `code`** (`:824`) and never `command`. The `no reply within {}s` line
  (`:833`–`:838`) and `replied without a number` (`:853`) do the same
- **Why it matters:** hunting down firmware that refuses the `,15` (DCS argument) is the sole
  purpose of this retry (comment `:800`–`:803`) — but **the log cannot tell which form was
  refused**, so the firmware pattern cannot be recorded in the field
- **Suggestion:** make it carry whether `,15` was present — for example
  `USSD *88# (with ,15) rejected …` / `USSD *88# (bare) rejected …`, or write `command` out
  directly. It is **a string with no OTP or subscriber number in it**, so it is harmless at Info
  level (`AT+CUSD`'s actual reply body must stay at debug — AGENTS.md Logging refusal).
  `fix:` → **v1.6.2**

### D.3 `LiveEvent::Closed` does not decrement the ready count — the status line and the badge disagree

This is `05 §C.6` (L3) itself. But it has to be recorded here that **v1.5.0 made it far more
visible and far easier to reach** — which is why pulling it into v1.6.2 is worth more than
leaving it parked in C.6.

- **Symptom:** when a worker exits altogether (a dead transport, or a panic) the status line
  **keeps counting** that port inside `Live N/N ready`
- **Evidence:** the `Offline` arm (`src-tauri/src/commands/mod.rs:1051`) does
  `st.live_ports_ready.retain(|p| p != &port)`. The `Closed` arm (`:1184`) sets
  `p.live_ready = false` (`:1191`) and does `st.live_failed.push` (`:1193`), but **never touches
  `live_ports_ready`**, and rather than calling `live_status` again (`:532` — it takes the count
  from `live_ports_ready.len()`) it overwrites `status_text` directly with
  `"{port} FAILED: {e}"` (`:1194`). That `retain` belongs to the `Offline` arm **alone**:
  `Offline` is the only arm that rebuilds the status line from `live_status`, while
  `LiveEvent::Reconnecting` writes `status_text` by hand and leaves the ready list stale, so it
  needed the same fix (review note 3 in the status box above)
- **The frontend corrects itself, Rust does not:** because the `Closed` arm emits
  `ports:updated`, the `ports:updated` listener at `src/lib/services/api.ts:242` re-derives
  `readyPorts` from the port list itself with `filter((p) => p.live_ready)` at `:248`–`:250`
  (the `live:reconnecting` and `live:offline` listeners derive it the same way at `:255` and
  `:271`), and the badge string is built from that count in
  `src/lib/components/Toolbar.svelte:39` — so **the badge is right** (`Live 33/34`) while
  **the status line coming from Rust is wrong** (`Live 34/34 ready`). Two counters disagreeing
  with each other on one screen
- **Why v1.5.0 made it worse — two things:**
  1. **#17 put `statusText` onto the Inbox footer** — a wrong number that used to be visible
     only on the Ports page now sits **on the main page the operator works on**
  2. **#18 made panics report through `Closed`** (`live::WORKER_PANIC`) — so the panic path
     newly reachable in v1.5.0 leads straight into **that one arm which does not decrement the
     ready count**. On a worker panic the row goes red, but the status line keeps saying
     `34/34 ready`
- **Suggestion:** add `st.live_ports_ready.retain(|p| p != &port);` to the `Closed` arm and
  rebuild `status_text` from `live_status(&st, port_count)` (`{port} FAILED: {e}` does not get
  lost, because `live_failed` already shows it). **Whether it goes into `live_offline` has to be
  decided deliberately** — `Closed` means "the worker died", not "there is no modem", so
  counting it as `no modem` would create a new falsehood. Test: after a `Closed`, `live_status`
  decrements ready. `fix:` → **v1.6.2**

---

## E. What this plan **excludes** (deferrals already decided — not restated here)

| Item | Where it lives |
|---|---|
| `developer.autoScroll` — the one setting still undecided | `05 §C.3` |
| limitations L1, L2, L4 (renamed stick, no live thread pool, no liveness re-probe) | `05 §C.4`, `§C.5`, `§C.7` |
| ~~L3 `Closed` over-count~~ — **pulled into this plan and shipped in v1.6.2** (`03 §24`), see §D.3 | `05 §C.6` → `§D.3` |
| merging the four supervisors into `run_port_pool` — **last in the order** (§C is this entry's down payment) | `05 §C.10` |
| the missing `main` ruleset + the updater release-draft flip | `02 §6` |

---

## 🎯 Release Shape — summary

| Release | Contents | Commit type | Risk | Hardware needed | Status (2026-09-04) |
|---|---|---|---|---|---|
| **v1.6.2** | §C (the cleanup status line counting "no modem" as a failure) + §D.1 (the `msg(s)` unit meaning two things) + §D.2 (the USSD rejection line) + §D.3 (`Closed` not decrementing the ready count — review brought the **`failed` bucket** and the **`Reconnecting` arm** in on top) | all `fix:` | **low** — counters and wording, plus the arithmetic of the status line. AT sequence / timeouts / delete confirmation **untouched**, and the event payload change is additive (`empty` added to `sim_cleanup:done`) | not needed (unit tests are enough — 11 more Rust tests added) | **shipped and released as v1.6.2** — `#28` merged into `main`, `chore(main): release 1.6.2 (#29)`, tag `v1.6.2` (cases `03 §23`–`§26`) |
| **v1.7.0** | §B (live worker command mailbox — Delete / Clear All / Get SIM Numbers usable while live is on) | `feat:` | **high** — it changes live loop timing | **needed — the `04 §G` playbook**, plus a new `Memory/03` case entry | **not started** |

> **Order:** v1.6.2 shipped first, as planned. It was low risk and it makes the operator's status
> line immediately trustworthy — and when §B's much larger change is being debugged, **a status
> line you can trust becomes a tool**. What is left is §B itself, which **has not been started**:
> its PR title must be a valid conventional commit (`06` doc) → squash merge → the
> release-please PR.
