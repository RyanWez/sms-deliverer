# 03 — Troubleshooting Casebook (27 cases that actually happened, + 8 latent traps)

> Format: Symptom → Root Cause → Fix → Preventive Rule. Search in here first, before you debug.

---

## 1️⃣ In-app "Check Now" → the spinner finishes and then nothing appears

- **Symptom:** the UI button spinner turns, result popup/notification zero
- **Root Cause:** the result rendering used the browser `alert()` / `confirm()` — in a Tauri v2 webview (esp. Linux WebKitGTK) those are
  **silently swallowed** (not even an error is raised)
- **Fix:** in `src/lib/services/updater.ts` → replaced with the toast system (`liveStore.addToast`) + the `@tauri-apps/plugin-dialog` native confirm
- **Rule:** do not use browser dialog APIs in a Tauri frontend — only plugin-dialog / the dialog wrapper.
- **v1.3.0 update:** the native confirm has now been removed as well. An available update is shown with the
  in-app card under Settings → Updates (release notes + Update Now + Restart Now) — long release notes
  cannot be read inside a dialog, and the shape is completely different on every platform. The state machine is in
  `src/lib/stores/updater.svelte.ts`, and the rate limit + byte/percent formatting in
  `src/lib/utils/update-policy.ts` (pure, tested).

## 2️⃣ Plugin calls do not work (updater/process/dialog)

- **Symptom:** invoke fails / denied / silent
- **Root Cause:** Tauri v2 capabilities ACL — having the plugin registered is not the same as having the permission
- **Fix:** in `src-tauri/capabilities/default.json` → add `"updater:default"`, `"process:default"`, `"dialog:default"`
- **Rule:** every time a new plugin goes into `Cargo.toml` + `lib.rs`, add its capabilities permissions too, then rebuild.

## 3️⃣ latest.json 404 + no .sig assets

- **Symptom:** the release holds only the installers, `latest.json`/`*.sig` zero → updater endpoint 404
- **Root Cause chain:** the sign key/password secret (or) the `bundle.createUpdaterArtifacts` flag is missing → no .sig is produced →
  tauri-action log: *"Signature not found for the updater JSON. Skipping upload..."*
- **Fix:** add `"createUpdaterArtifacts": true` + verify the secrets (pubkey ↔ private pair match!)
- **Health probe:** `releases/latest/download/latest.json` → HTTP 200 expected

## 4️⃣ Ubuntu CI build: `rust-lld: error: unable to find library -lxdo`

- **Symptom:** the windows job succeeds, the ubuntu job gives `linking with cc failed`
- **Root Cause:** the Tauri `linux-libxdo` feature (a global-shortcut dependent crate) needs a system lib that the runner does not carry by default
- **Fix:** add `libxdo-dev` to the workflow's apt step
- **Rule:** keep the Linux runner's new requirements (gtk/webkit/appindicator/rsvg/patchelf/**libudev**/**libxdo**) in mind as a known set.

## 5️⃣ The update is not seen (an update exists but the app says "already latest")

- **Symptom:** even after a new release is published the app does not find the update
- **Root Cause:** version mismatch — the updater compares the app runtime version against the latest.json payload.
  History: while npm=1.x / tauri.conf=2.0.0 were mixed — release-please bumped only the npm field, so the tauri version stayed at 2.0.0
- **Fix:** the extra-files config (doc 02 §2) auto-bumps the chain across all 4 files; baseline reset to 1.0.1
- **Rule:** merge only once you can see all 4 version lines changing in the release PR diff.

## 6️⃣ The release PR is wrong / tag scheme confusion (sms-tauri-v… prefix)

- **Symptom:** in the RP PR #4/#5 diff the compare link reads `sms-tauri-v1.0.1...sms-tauri-v1.1.0`; PR open → close → reopen cycle
- **Root Cause:** the upstream default `includeComponentInTag=true`; it does not match the manual bootstrap tag `v1.0.1`
- **Fix:** `"include-component-in-tag": false` in the config → RP auto-closes the stale PR + recomputes (verified live)
- **Rule:** do not cut the first release until the tag naming config has been aligned with the baseline.

## 7️⃣ Ghost publish run "queued" 14+ hours / a tag that was deleted

- **Symptom:** an old-tag run in the run list looks pending forever
- **Diagnose:** `gh run list` + `gh run cancel <id>` (if it has already completed, cancel returns an error — harm none);
  which releases/tags are left: `gh api repos/<o>/<r>/releases` , `git ls-remote --tags origin`
- **Rule:** after a full purge (delete releases → delete tags w/ branch cleanup) the run history can go stale — do not panic, verify the API/state machine first.

## 8️⃣ Release PR tests fail: `cargo test --locked` — Cargo.lock out of sync (now auto-fixed)

- **Symptom:** in PR #7 (release 1.2.0) both the ubuntu and windows cargo-test jobs fail —
  `error: cannot update the lock file … because --locked was passed`
- **Root Cause:** release-please bumps the `version` in `Cargo.toml` but does not touch the
  `sms-tauri` package version inside `Cargo.lock` (in doc 02 §2 the extra-files do not include the lock).
  Only cargo itself can write the lockfile version, and CI runs with `--locked`, so it reports the mismatch as an error.
- **What was done by hand before (kept here for context):** checkout the release branch → `cargo check`
  (regenerate the lock) → `chore: sync Cargo.lock with version X.Y.Z` commit → push to the PR branch →
  CI auto re-run → green → merge (commits `2d32e01` / `49a489a`)
- **What happens automatically now:** the `sync-cargo-lock` job has been added to
  `.github/workflows/release-please.yml`. It looks at release-please's
  `prs_created` / `pr` output, checks out the `fromJSON(...).headBranchName` branch with `secrets.PAT` →
  `cargo update --workspace --manifest-path src-tauri/Cargo.toml` →
  verifies with `cargo metadata --locked` → and only when `git diff --quiet -- src-tauri/Cargo.lock` shows
  a change does it commit + push `chore: sync Cargo.lock with version X.Y.Z`
  (loop guard — because release-please re-runs on every push to main). Pushing with the PAT
  re-triggers the PR's own checks (with the default `GITHUB_TOKEN` it does not).
- **Rule:** if you hit a `--locked` error on a release PR again, **do not resolve the lockfile by hand first** —
  (a) did the `sync-cargo-lock` job run at all (did the release-please job output `prs_created=true`),
  (b) is `secrets.PAT` still within its lifetime/scope — check these two first. Do not close/reopen the PR.
- **Unverified (2026-08-30):** this automation was added on this branch
  (`fix/reliability-and-privacy-hardening`) and **has not yet** been through a single real release
  PR. That is why the manual procedure above remains as the fallback.
  (The original manual fix was verified on v1.2.0, 2026-08-28)

## 9️⃣ Scan / Live / Get SIM take far too long — 64 ports exist but only 7 SIMs

- **Symptom:** with only 7 SIMs inserted in the SIM bank, Scan ~97s, Get SIM ~3 minutes;
  `modem not responding` 57 times in the log; in Live all 64 ports report `Live ready`
- **Root Cause:** the SIM bank creates a tty device for every channel — **regardless of whether a SIM is present**.
  `available_ports()` finding 64 devices is not wrong, but because there was no liveness-checking step,
  all three of `read_port` / `get_sim_number` / the live worker sent the full AT sequence to a silent port and
  paid every one of the timeouts to the end — Scan 24s, Get SIM 35s, Live 22s per empty slot
  (× `MAX_CONCURRENT_PORTS = 16`, 4 batches)
- **Secondary bug:** `network_problem(None, None)` returned `None` ("no problem").
  So a modem that answered nothing at all to `AT+CREG?` went on to spend 2×9s on USSD —
  the pre-check that was added precisely to save that timeout did **nothing at all** on the ports that needed it most
- **Fix:** `modem::probe_channel()` — `AT` twice, 800ms each. If any one final result code (OK/ERROR/+CME ERROR)
  comes back, the port is alive. This gate is placed in front of all of `read_port`, `get_sim_number`, `delete_messages`
  and the live worker → a dead port costs **~1.6s** instead of 24s/35s.
  USSD is also skipped when `AT+CREG?` returns no result code.
  In the live worker, when the probe fails it is `LiveEvent::Offline` instead of `Ready` (re-probe once every 60s).
  The `detect_ports` command + `PortInfo.alive` + the Ports page "Detect Modems" button →
  dead ports are auto-unchecked (probe concurrency is separate: `MAX_CONCURRENT_PROBES = 32`)
- **Rule:** before sending a heavy AT sequence to a port, **check first whether it is alive**.
  A device node existing is **not** the same as a modem existing. Do not resolve this by lowering timeout constants —
  adding the gate is the root-cause fix. (verified in the v1.2.0 log, 2026-08-29)

## 🔟 Long (concatenated) SMS come out with their characters mangled — GSM-7 + UDH

- **Symptom:** garbage of the form `iAX§OOKIARÑi§AΘsΦÇAB...` in the export; OTP `None`;
  in the log it happens only after `live SMS read (idx 3) [concat]` + `(idx 4) [concat]` link up
- **Root Cause:** in the GSM-7 branch of `decoder.rs` the UDH is **skipped twice** —
  the byte cursor `i += 1 + udhl` steps over the header, and then `decode_gsm7(&bytes, i, septets, skip)`
  is told to skip `skip * 7` bits **again**. Result = bit alignment off by 1 bit → every character changes
  (spaces turning into `A` is this shift's signature)
- **Why it was not caught:** there was only a single concat test and it exercised UCS-2 (Myanmar).
  The GSM-7 + UDH path — the path most English OTP messages arrive on — had **zero** coverage
- **Fix:** make the bit cursor start from the UDHL byte (`ud_start`), and advance `i` only for the UCS-2 recovery probe.
  3 tests added: UDHL=5 (1 fill bit), UDHL=6 (0 fill bits), and a GSM-7 regression without UDH
- **Rule:** GSM-7 septet counts are counted from the **start** of the UDH (header + fill bits included),
  not from the payload byte. Every time a new encoding branch is added, write tests for **both** with and without UDH.
  (commit `431dcaf`, 2026-08-29)

## 1️⃣1️⃣ Pressing Get SIM gives `Found: 1/32` — registered modems refuse USSD

- **Symptom:** only 1 of the 32 ports yields a number. In the log 22 ports answer both `*88#` and `*124#` with
  `+CME ERROR: 100`, 7 give `no reply within 9s`, and only 2 show `reg stat 2` (no network)
- **Root Cause (not one but three):**
  1. `+CME ERROR: 100` comes back **within 13 ms** (14:27:36.811 → .824) — that is not a network timeout,
     the modem is refusing the command outright. If an earlier run left a USSD session unclosed, the firmware
     believes a session is still open and instantly refuses the new `AT+CUSD=1`
  2. some firmware does not accept the `,15` (dcs argument)
  3. only the two USSD codes `*88#`/`*124#` are hardcoded — they differ per carrier
- **What matters:** 29 of the 31 failures report `reg stat 1` (home network, signal 10–22/31) —
  so "the SIM line is bad" is true for only 2 ports. The rest is a modem/USSD problem
- **Fix:** change the order in `get_sim_number` — (a) clear a stale session first with `AT+CUSD=2`,
  (b) try `AT+CNUM` (EF_MSISDN — a file on the SIM, needs no network, returns within ms) **first**,
  (c) when USSD is refused, retry the same code once without the dcs. The code list is
  collected in one single place as the `OWN_NUMBER_USSD_CODES` const (Mytel: `*88#` → `*124#`)
- **Rule:** do not ask the network for what you can get straight from the SIM. When a `+CME ERROR` comes back within ms
  it is not a network problem — look at the modem state. `Found: x/y` is not data loss:
  the numbers obtained earlier remain in the cache exactly as they were (the `else` branch of `ussd_one_port` only writes a log).
  But **if the SIM in one slot is swapped**, the old number stays behind on that stable path key —
  that is the cache's one and only danger

## 1️⃣2️⃣ Two SIM numbers for one port, With SIM 34 while only 32 modems exist

- **Symptom:** ttyUSB39 (READY) and ttyUSB43 (NO MODEM) both show `***803`.
  ttyUSB27/28 share the same `***632`, ttyUSB30/31 share the same `***146` (both READY).
  The badge reads `With SIM 34` but `32 modems` / `Selected 32`
- **Root Cause:** `stable_id()` **never matched once**. `serialport` returns
  `/dev/ttyUSB7`, while the `/dev/serial/by-path/...` symlink points at `../../ttyUSB7` —
  the code compared `target.file_name() == name`, i.e. `"ttyUSB7" == "/dev/ttyUSB7"` →
  **always false**. So `p.path` was just the port name, and the cache key was the unstable tty name.
  On top of that, `number_of(stable, legacy)` falling back to the legacy **name** key made it worse still —
  after a hotplug renumbering, the previous SIM's number appeared on whatever other stick now holds that name.
  The `/dev/ttyUSB64..67` rows in the CSV (sharing numbers with ttyUSB48..51) are the evidence that renumbering happened
- **Fix (architecture change):** the number is no longer filed against the port, it is filed against the **ICCID**.
  `probe_port` tells the card apart with `AT+CCID`/`AT+ICCID`/`AT^ICCID`. CSV format v2 =
  `sim,<iccid>,<number>` (durable) + `slot,<stable_path>,<iccid>` (a hint only).
  A slot with no modem loses its hint (the number remains under the ICCID),
  and moving a SIM to another slot brings its number along. The v1 (port-keyed) file is not migrated —
  it is set aside as `sim_numbers.csv.v1-port-keyed` and abandoned (its keys can no longer be trusted).
  The frontend `hasValidSim()` no longer counts an `alive === false` port as With SIM
- **Rule:** naming something "stable id" does not make it stable — **write a test** for whether it resolves
  (`stable_id_resolves_a_by_path_symlink` checks it with a tmpdir symlink). When choosing a cache key,
  choose **the identity the hardware itself reports** (ICCID), not the name the OS hands out.
  A legacy fallback key is the road to silent data corruption — if it cannot be migrated, dropping it is better.

## 1️⃣3️⃣ Only 32/64 modems found on Windows — the bank had not finished booting (misdiagnosis case)

- **Symptom:** in the Windows session (18:00–18:08), 32 × `Modem not responding (no reply to AT)`,
  `Found: 31/64` → `34/64`. In the Linux session (18:47–18:52), `63/64` + 62 numbers.
  In the Windows log there is **not a single `Cannot open`** — every port opened and half of them did not answer `AT`.
  The set that did not answer is identical across all 8 minutes (COM3–18, COM26–29, COM39–50)
- **The conclusion I got wrong first:** looking at "an identical set + 100% open success" I diagnosed it as a
  static config fault (DTR/RTS low). The platform asymmetry of the `serialport` crate
  does genuinely exist (below), which made it look even more believable
- **The real Root Cause:** the bank **had not finished power-up/enumeration**. 4 pieces of evidence:
  1. **On the same Windows machine, the same version, in a session 50 minutes later (18:56–19:24),
     `Modems OK: 62/64`, `Found: 57/64`, `Live ready 63`** — it got better on its own with no code change
  2. Within the 18:00 session, COM10 at 18:03 and COM18 at 18:04 **came back on their own** (staggered boot)
  3. In the Linux log the nodes appearing stage by stage is directly visible: 52 ports → 32 nodes missing → 64
  4. If DTR/RTS were low, our DCB is identical in every session, so it **would have to fail deterministically** —
     it did not. That is the point that cuts the theory down
- **Why Windows looks worse than Linux:** on Linux, until the module is ready the `/dev/ttyUSB*`
  node does not exist → it honestly says `Cannot open`. On Windows the `COM` port is
  always present in the registry, so it opens and answers nothing → which misleads you into "there is no modem"
- **Fix (hardening only, not for the diagnosed fault):** in `open_port`, an explicit `.flow_control(None)`
  + `raise_modem_lines()` (DTR/RTS assert; a no-op on Linux). `at.rs::send` marks a write error
  as a dead channel and says `Serial I/O failed: …` — kept separate from `NOT_RESPONDING`, so
  none of the three of scan/USSD/live records alive=false any more (`windows/com.rs` also plugs the port timeout in
  as `WriteTotalTimeoutConstant`, so on a TX stall the AT never leaves and it lies with "the modem does not answer").
  **An `AT&K0` nudge was added and then reverted** — it rested on the wrong theory, and besides,
  turning flow control off on a module that has not finished booting can cause a FIFO overrun on large PDU dumps
- **Rule 1:** **a fault that recovers by itself is not a static config fault.**
  When resolving a root cause, check first "if this theory is right, how would it fail" — a theory that has to be
  deterministic cannot explain an intermittent symptom
- **Rule 2:** look for the newest log file **first**. I compared 18:00 (the bad one) with Linux;
  the 18:56 Windows log was the single thing that would have settled it, and I did not use it
- **Rule 3:** never surface "the device does not answer" and "I could not send" as one and the same
  error — merging them sends you hunting for a SIM problem when the bug is host-side

## 1️⃣4️⃣ Deleting a live mode message shows "Deleted 1 message(s) (1 SIM slot(s) freed)" but it stays on the SIM

- **Symptom:** while Live mode is running, deleting an arriving SMS from the Inbox makes the row disappear and
  the status line posts the **clean success** line `Deleted 1 message(s) (1 SIM slot(s) freed)`.
  But the message is still on the SIM. Because `live::seen` has recorded its fingerprint it does not come back
  during that session — the slot is **occupied without being visible**. On a card with only 20–50 SIM slots
  it gradually fills up and the modem will start silently refusing new SMS
- **Root Cause:** both `decoder::parse_cmgr` and `parse_pdu_cmgr` **hardcode** `index: 0`.
  The `+CMGR` header carries the status but not the index — the only party that knows the slot is the
  caller that sends `AT+CMGR={idx}` (`live::handle_cmgr`), and it never wrote it back into the parsed message.
  So every SMS that arrives during live mode has `index == 0`, and for a concat,
  `part_indices == [0]` (because `Reassembler` takes each fragment's slot from `msg.index`)
- **How it got past the `confirmed_removals` invariant** — this is the most important part:
  1. `message_slots` returns `[0]` → `delete_each` sends `AT+CMGD=0`
  2. SIM slots start **at 1**, so the modem refuses
  3. `slots_still_present` re-reads with `AT+CMGL="ALL"` — slot 0 is **not** in the list,
     because **it is a slot that does not exist**
  4. `confirm_delete` reads "not in the list" as **evidence** that it is gone
  → `ok: true, deleted: 1, indices: [0]` → `confirmed_removals` lets the row go.
  The entire re-read-the-SIM confirmation **worked and still gave the wrong answer** — the absence of a
  slot that does not exist is evidence of nothing
- **Blast radius (precisely):** only the one path that **manually deletes** a row obtained from live mode.
  The scan path (`parse_pdu_list`/`parse_text_mode_list`) fills the slot in correctly from the `+CMGL` header.
  The retention sweep (`live::sweep_expired`, `modem::expire_old`) **re-reads the SIM itself**
  and uses the list parser, so it is unaffected
- **Fix (v1.5.0):** `parse_cmgr(resp, port, idx)` / `parse_pdu_cmgr(resp, port, idx)` — the slot goes in as a
  parameter, with no default. `live::handle_cmgr` supplies `idx` **before** `asm.push`, so
  `finish()`'s `part_indices` collects the real slots. `decode_deliver`'s `index: 0` is
  left in place, but with a comment added saying "the raw PDU carries no slot; both callers overwrite it"
- **Second line of defence:** both `message_slots` and `models::expired_indices`
  drop non-positive indices. Since `confirmed_removals` already checks `!idxs.is_empty()`,
  a row with no slots will be **KEPT** — whatever path a 0 arrives from in future, there is no more
  silent deletion
- **Rule 1:** **"if you are going to use absence as evidence, remove the sentinels first."** Absence-based
  confirmation (`slots_still_present`) only works when the key being looked for is a key that can actually exist.
  For a key that cannot exist the answer is always "it is gone"
- **Rule 2:** **do not** give `0` the double meaning of "unknown". With `index: Option<i32>`
  this bug would have been a compile error. `i32` + a hardcoded `0` went wrong silently
- **Rule 3:** a test that checks `text`/`from` is not enough. `pdumgr_single_read` already
  existed but did not assert `index`, which is why the bug was not caught — **assert every
  field that is an input to delete/cleanup**

## 1️⃣5️⃣ The Inbox Search box erases what you type — search does not work at all

- **Symptom:** typing into the Inbox's Search messages field does not stick. Actually tried in the browser preview
  (localhost:1420) — after typing `+469602294397` (13 characters) one character at a time,
  **the field is empty**, the message count stays at `35 msgs`, and the filter never takes effect.
  This is not "a few characters got skipped" — **the entire search feature does not work**
- **Root Cause:** the sync `$effect` in `FilterBar.svelte`:

  ```js
  if (messagesStore.query === '' && localQuery !== '') {
    localQuery = '';                       // ← no debounceTimer guard
  } else if (messagesStore.query !== localQuery && debounceTimer === null) {
  ```

  Branch 2 is shielded with `debounceTimer === null` so it cannot clobber during typing.
  **Branch 1 is not shielded**. And because `debounceTimer` is not `$state`, the effect does not track it —
  the dependencies are only `messagesStore.query` and `localQuery`. So every keystroke
  changes `localQuery` → the effect re-runs → `query` is still `''`, so
  `localQuery = ''` → `value={localQuery}` writes the DOM back. This happens every time `query` is
  `''` (= when you start typing, and after a clear)
- **This effect was guarding a case that cannot happen:** the comment says "store is cleared externally",
  but the only writer of `messagesStore.query` is `FilterBar` **itself**
  (`onSearchInput` and `clearSearch` — confirmed by grepping the whole repo).
  So there is no external update to mirror. Page navigation is already resolved by the
  `let localQuery = $state(messagesStore.query)` initialiser itself
  (when the Inbox remounts, the store still holds the query)
- **Fix (v1.5.0):** the effect was deleted. The input owns `localQuery`, and the debounce
  writes the store — one-way. `src/lib/pages/Ports.svelte:28-42` already had this shape
  written correctly (`rawQuery`/`debouncedQuery`, no sync effect)
- **Verification (A/B, browser preview):** with the effect present — the field is empty, still `35 msgs`.
  After deleting it — the field holds all of `+469602294397`, the count falls to `1 msgs`,
  and the row that remains is that very sender. After the Clear button `35 msgs` comes back, and
  typing again after a clear works too (the case where the effect was at its worst)
- **Rule 1:** **in runes, a plain `let` cannot be an effect's guard.** Because `debounceTimer` is
  not reactive the effect does not track it — yet reading it **inside** the effect
  gives the impression that it is guarded. Writing the guard in one branch and
  not in the other is the exact shape of this bug
- **Rule 2:** **do not give the DOM's value two sources of truth.** `value={localQuery}` is a
  binding and the input event writes `localQuery` — any effect that steps in between and writes
  `localQuery` is racing the user's hands
- **Rule 3:** if you are going to write "an effect that syncs an external update", **grep for whether an external
  writer actually exists**. If there is none, that effect is not a protection but a bug
- **Rule 4:** if it is a layer the frontend cannot unit-test yet (a component), **A/B it in the browser
  preview** — stashing the fix and re-running the previous state proves both "it is fixed and
  works" and "this really was a bug"

## 1️⃣6️⃣ A port going busy for a moment becomes "NO MODEM" and the SIM number disappears from disk

- **Symptom in 2 kinds, one single cause:**
  1. If ModemManager or some other process holds the port for a moment while **Detect** is running
     (or if it hits EBUSY), that port becomes `NO MODEM`, gets deselected, and then
     **the slot→ICCID mapping is lost from `sim_numbers.csv` for good** — getting the SIM number
     back means re-running Get SIM Numbers
  2. After a **Refresh** (background timer, once every 30 seconds by default) the error text
     "Reconnecting: Port lost: EIO" / "Serial I/O failed: …" **disappears**,
     and then **never comes back** — the port shows as an ordinary idle row for the whole remainder of the outage
- **Root Cause (1):** `detect_ports` **collapsed** the probe's 4 possible outcomes **into two**:
  ```rust
  let (alive, iccid) = match probed {
      Ok(Ok(r)) => (r.alive, r.iccid),
      _ => (false, None),          // ← both Err and panic became "no SIM"
  };
  ```
  And `ProbeResult` **had no** error field, so `probe_failure_reason`'s
  "silence vs transport failure" distinction (the one all three of scan/USSD/live obey) was completely
  absent on this one path. As soon as `alive == false`, `sim_dir.clear_slot(&path)`
- **Root Cause (2):** `merge_ports` set `live_error: None` **unconditionally**.
  Since `OutageLatch` sends only one event per outage (it does not resend again and again), what was deleted
  **never comes back**. `alive` was already being carried over, so **only the silence** remained, and
  the two things the operator needs most are the two that vanish
- **Fix (1) — v1.5.0:** add `failure: Option<String>` + `proved_empty()` to `ProbeResult`, and then
  a 3-variant `ProbeVerdict` enum:
  | Verdict | `alive` | `checked` | `iccid` | `sim_dir` | `live_error` |
  |---|---|---|---|---|---|
  | `Alive(iccid)` | `Some(true)` | `true` | set (when obtained) | `set_slot` | `None` |
  | `Empty` (silence only) | `Some(false)` | `false` | `None` | `clear_slot` | `None` |
  | `Inconclusive(why)` | **untouched** | **untouched** | **untouched** | **untouched** | `Some(why)` |

  `Empty` is the **one and only** verdict allowed to set `alive = Some(false)`
- **Fix (2) — v1.5.0:** `live_error` is carried across a refresh — but, just like `live_ready`, **only when the tty
  name is unchanged**. After a renumber, that message is about the worker of a name that no longer exists,
  so it misleads. **There is nothing to expire**: both `start_live`/`stop_live` clear
  every `live_error` at their boundaries, and `detect_ports` overwrites it per port itself
- **Trade-off (deliberately chosen):** an Inconclusive port **stays selected as it was**, so
  the next scan has to pay its timeout. Compared with losing the ICCID mapping this is far cheaper —
  and besides, the status line says it plainly: `N port(s) could not be probed — left as they were`
- **The part that is still outstanding:** on a port that was previously `Empty` (`alive == Some(false)`),
  posting an Inconclusive verdict still shows only `NO MODEM` and not the reason text, because
  `portStatus` checks the `alive === false` branch first. **Deliberately left that way** — reordering the branches
  would turn every empty slot in the bank red for one transient failure (the comment in `utils/port.ts`
  warns about exactly that itself). The existing evidence says "empty", so showing that is correct
- **Rule 1:** **do not treat "unknown" as one and the same as "absent".** A `bool`/`(bool, Option<_>)`
  with only 2 states invites this collapse — if there are 3 verdicts, use an enum, and
  the compiler will make you handle every arm
- **Rule 2:** in `match probed { Ok(Ok(r)) => …, _ => … }`, **the catch-all arm silently eats
  the distinction**. Handle the `Err` variants one by one
- **Rule 3:** when writing code that says it "clears" state, check **whether it will come back**.
  Deleting state that came from a latch/one-shot event is **deleting it permanently**
## 1️⃣7️⃣ The live SIM sweep logs "deleted N" yet the SIM fills up — two copies drifted apart

- **Symptom:** On a bank running continuously in Live mode the log reports `SIM cleanup deleted N
  expired message(s)` once every ten minutes (`SIM_SWEEP_EVERY`), but the SIM fills up little by
  little and the modem starts silently refusing new SMS. **This is exactly the failure the sweep
  exists for**
- **Root Cause — one operation written as two implementations, one of which drifted:**
  | | `modem.rs` (scan path) | `live.rs::delete_indices` (drifted copy) |
  |---|---|---|
  | Checking the `AT+CMGD` result | `l.trim() == "OK"` (the whole line) | `resp.contains("OK")` |
  | Re-reading the SIM to confirm | `slots_still_present` + `confirm_delete` | **nothing at all** |

  `contains("OK")` also accepts text such as `+CMS ERROR: 321 ... NOT OK`, a command echo, or an
  `OK` that merely appears inside an unsolicited line. And because there is no confirmation, what
  gets counted is the number of deletions the modem was **asked** for, not the number that actually
  **went**
- **Fix (v1.5.0) — the helper was merged (structural)**: `modem::delete_confirmed(ch, port, indices,
  list_cmd)` was exposed as `pub(crate)` and `live::delete_indices` was **deleted**.
  `delete_messages` (which opens the port itself) and the live sweep (which cannot open it) now both
  go through a single entry point — **there is nothing left that can drift again**
- **Why the `list_cmd` parameter is needed:** `confirm_delete` used to hardcode `AT+CMGL="ALL"`
  (the text-mode form). The live worker runs in **PDU mode** whenever it can — there the quoted form
  returns `ERROR` → `slots_still_present` gives `None` → it silently falls back to the per-command
  count (= the very bug being fixed comes back). `list_all_cmd(pdu_mode)` picks `AT+CMGL=4` /
  `AT+CMGL="ALL"`
- **Test:** `the_sweep_deletes_high_slots_first_and_confirms_against_the_sim` (highest-first
  order + **the re-read is actually sent** — the old live loop never sent it),
  `the_sweep_confirms_with_the_list_form_for_the_mode_it_is_in`
- **Rule 1:** **Never keep two implementations of one operation.** In this repo `setup_sms_mode`
  had been written twice, once for scan and once for live, drifted, and had to be merged (noted in
  the `modem.rs` doc comment) — this is **the same lesson a second time**. Expose it as a
  channel-taking helper and call it; do not copy it
- **Rule 2:** **Never** check an AT reply with `contains("OK")`. Use only `lines().any(|l| l.trim()
  == "OK")` — `OK` must be a result code, not a substring of text
- **Rule 3:** When merging a helper, check **whether a mode-dependent constant is hiding in it**.
  `AT+CMGL="ALL"` was hardcoded, so merging alone would silently degrade the PDU-mode caller —
  lifting it out as a parameter is what resolves that

## 1️⃣8️⃣ Adding `reqwest` as a new dependency brought in 22 crates + a cmake/C toolchain, and then building the client panicked

- **Symptom (two in a row):** For the Telegram forwarder I wrote `reqwest = { version = "0.13.4",
  features = ["blocking", "json", "socks"] }`. It compiled, every test passed — but
  1. `Cargo.lock` went from **501 → 523** packages (`aws-lc-rs`, `aws-lc-sys`, `cmake`,
  `fs_extra`, `h2`, `encoding_rs`, `chacha20`, `core-foundation` …). `aws-lc-sys` needs
  **cmake + a C compiler** — a new failure mode for the Windows CI leg
  2. Putting it back with `default-features = false` + `rustls-no-provider` stopped the crates
  coming in, but `build_client()` then **panics**:
  `No rustls crypto provider is configured. When using the rustls-no-provider feature you must
  install a crypto provider before building a Client`
- **Root Cause:** `reqwest`'s default features include `default-tls` → `rustls` →
  `__rustls-aws-lc-rs` (the aws-lc-rs provider), plus `charset` (encoding_rs), `http2` (h2) and
  `system-proxy`. But `tauri-plugin-updater` uses reqwest with **`default-features = false`,
  features `["json", "stream"]` + `rustls-no-provider`** and pulls `rustls` with
  `features = ["ring"]` — so this binary **already contains a ring-backed rustls**, and I was
  pulling in an entire second provider.
  `rustls-no-provider` hands the process-level provider over to the caller — the updater installs
  one at `updater.rs:446` if `CryptoProvider::get_default().is_none()`, but **only when the updater
  runs**. If the operator presses Verify before the updater has run there is no provider → panic
- **Fix:** the feature set was made **exactly the same** as the updater's, with only the two
  additions that are needed (`blocking` — this crate uses OS threads and blocking I/O, not async
  tasks · `socks` — the feature list has `socks = []`, so it adds no crate at all). Then `rustls`
  was added as a direct dep (ring, default-features off — already in the lock) and
  `telegram::ensure_crypto_provider()` (`std::sync::Once`) does the install inside `build_client` —
  no dependence on the updater. `install_default()` returns `Err` when one already exists, and
  since that is expected, `let _ =`
- **Result:** the `Cargo.lock` diff is **3 lines only** (two `futures-sink`, `futures-channel` edges
  + `sms-tauri`'s `reqwest` entry). **Zero** new crates
- **Test:** `build_client_succeeds_with_no_proxy` (it catches the provider — because it panics,
  this trap is found in CI and not when the operator presses Verify),
  `build_client_accepts_a_socks5h_proxy` (fails if the `socks` feature is dropped — the one setting
  that makes this feature useful on a locked-down network),
  `build_client_treats_a_blank_proxy_as_direct`, `build_client_rejects_a_malformed_proxy`
- **Rule 1:** As soon as a new dependency is added, **check `Cargo.lock`'s package count**
  (`grep -c '^\[\[package\]\]'`). "It is already in Cargo.lock" **does not mean the feature set is
  the same** — the same crate with different features has a different transitive tree
- **Rule 2:** If a plugin already pulls a crate, **read that plugin's Cargo.toml and copy its
  feature set**. Guess, and you pull in a whole second TLS stack
- **Rule 3:** Features like `*-no-provider` / `*-no-default` **hand runtime setup over to the
  caller**. Compiling is not the same as working — write a test that builds the client/handle
  (no network needed, no hardware needed)

## 1️⃣9️⃣ A brief network outage and the OTP never reaches Telegram again — every error had been classified as one and the same thing

- **Symptom:** In a field run (2026-09-03 20:58) a live OTP came in and appeared in the Inbox, but
  it **never reached Telegram**. The log had one WARN only:
  `Telegram forward failed: Could not reach api.telegram.org: error sending request for url
  (https://api.telegram.org/bot<token redacted>/sendMessage)`.
  Ten minutes earlier (20:48) the same path had worked
- **Confirmed first — the token redaction holds on a real failure:**
  `<token redacted>` is the protection from `03 §18`. Without it the bot token would be written
  **verbatim** into `app.log` (which rotates at 5 MB) and onto the Logs page
- **Root Cause — `SendError::Other` was standing for two different things:**

  | Category | Example | On a retry |
  |---|---|---|
  | Telegram **rejects** it | `401 Unauthorized`, `chat not found` | never succeeds |
  | The **route does not reach** | DNS, broken route, timeout, ISP block page | **may succeed** shortly |

  `forwarder::run` **let both go** with `Err(e) => { report(); }` — the comment saying
  "keeping it would block every code behind it" is only true for a rejection; for a transport
  failure it **deletes the very reason the queue exists**
- **Why it dropped out briefly (verified):** `api.telegram.org` has both A and AAAA records.
  On this machine **the IPv6 route is broken** (`curl -6` → "Could not connect" at 0 ms,
  `curl -4` → `302`, 0.3 s), and the system resolver prefers IPv6
  (`getent hosts` returns the AAAA first).
  **But the connector must not be forced to IPv4:** that 20:48 worked is proof that hyper's
  happy-eyeballs fallback does hold. On an IPv6-only network `local_address(0.0.0.0)` would shut
  down forwarding entirely — the fix is retry, not address family
- **Fix:** `SendError::Other` was split into **`Unreachable` and `Rejected`** +
  `is_transient()`. A `.send()` / `.text()` failure in `post()`, a non-JSON body
  (a block page — something other than Telegram answering) and Telegram's own **5xx** are
  `Unreachable`; a 4xx `ok:false` is `Rejected`. On `Unreachable` the item goes back into the queue
  and the backoff climbs from `RETRY_BASE` 5 s to `MAX_BACKOFF` 60 s (reset on success)
- **Test:** `only_network_and_rate_limit_failures_are_retryable` (the classification itself),
  `interpret_treats_a_telegram_5xx_as_retryable`,
  `interpret_reports_a_non_json_body_with_a_bounded_preview` (block page = `Unreachable`),
  `interpret_surfaces_a_rejection_description` (401 = `Rejected`)
- **Rule 1:** An error enum's variant must stand for **the caller's decision**, not for a layer of
  message. `Other` hides "what should be done with this error" — whether a retry is possible or not
  has to be readable from the type
- **Rule 2:** When writing a queue, check **whether a transport failure ends up on the drop path**.
  The queue exists for exactly that failure
- **Rule 3:** Do not guess at "it dropped out briefly" — **actually check** (`curl -4` / `curl -6` /
  `getent ahostsv4|v6`). In this case the broken IPv6 route was found — but only after finding it,
  and proving that the fallback does hold, was it possible to decide **not to touch the connector**

## 2️⃣0️⃣ Forwarding stops with `CHAT_WRITE_FORBIDDEN` — even Send Test fails, but the code is not what is wrong

- **Symptom:** On a setup that had been working (it got through at 20:48), at 21:39 every forward
  stopped, and **even the Send Test button** failed:
  `Telegram forward failed: Telegram rejected the request: Bad Request: CHAT_WRITE_FORBIDDEN`.
  The token is right (Verify succeeds), the chat id is right, the network is fine
- **Root Cause — not the code, a group setting:** the bot is in the group but **no longer has
  permission to post**. Two possibilities:
  1. The group's **Permissions → Send Messages was turned off**. That restricts ordinary members —
     and **the bot is an ordinary member**
  2. The bot was removed from the group
- **How this case came about:** the trade-off list in `05 §1.1` says "protect the group invite like
  a password · allow Add Members to admins only". When the owner went in to change Permissions,
  **Send Messages got turned off along with it** — this is a failure mode created by our own
  security advice
- **Fix (Telegram side):** **promote the bot to admin** — an admin is not subject to member
  restrictions, and since the bot only uses `sendMessage` it needs no further rights.
  Or turn Send Messages back on
- **Fix (code side) — make the error message into the work to be done:** `rejection_hint()`
  was added. Telegram's string is **a complete diagnosis** but says nothing about what to do —
  an operator reading `CHAT_WRITE_FORBIDDEN` has no reason to know to go and look at the group
  Permissions. The toast now carries **both** Telegram's original text
  (to search for) **and** the fix. It covers five kinds: `CHAT_WRITE_FORBIDDEN` /
  `NOT ENOUGH RIGHTS` · `KICKED` · `CHAT NOT FOUND` · `UNAUTHORIZED` ·
  `CAN'T PARSE ENTITIES` (case-insensitive)
- **The classification was right:** treating this as `§19`'s `Rejected` is **correct** —
  no amount of retrying reopens the group permission. Had it retried, the queue would be
  blocked and every remaining OTP would be queued up behind it
- **Rule 1:** A third-party API's error string must be shown **verbatim** (so it can be searched),
  but **must never be shown on its own**. What the operator needs is not "what happened" but
  "what to do"
- **Rule 2:** When writing security advice, write **which setting must not be touched** as well.
  Saying "restrict Add Members" is what got Send Messages turned off too

## 2️⃣1️⃣ `extract_otp` reads the year `2026` out of a date as an OTP — a login notification got forwarded

- **Symptom (field, 2026-09-03 21:59):** MyID's **login notification** (not an OTP message)
  was forwarded into the Telegram group and the OTP badge showed **`2026`**.
  The message contains the date `2026/09/03 21:59:21` — there is no code in it
- **Root Cause — the last rung of the cascade, `P4`, caught the date:**
  1. The `KEYWORD_RE` gate **opened** — the message contains "OTP", but that is the
     **warning line** *"do not share your OTP with anyone"*. Notifications carry
     the keyword themselves — getting past the gate does not mean a code has to be there
  2. `P1` (digits within 24 characters after the keyword) found nothing · `P2`/`P3` found nothing
  3. **`P4` (`\b[0-9]{4,8}\b`)** caught `2026` — because `\b` is a boundary next to `/`,
     a date field **cannot be told apart** from a bare digit run
- **`Memory/05 §B.1` had foreseen this:** "P3/P4 match bare digits and are only safe because the
  keyword gate ran first · promotional SMS balances, **dates** and number
  fragments will start matching" — that date case is the one that really turned up in the field
- **Forwarding makes it worse (but it is not a regression):** the Telegram changes
  **never touched** `extract_otp` — it has been like this since v1.5.0. Before, one person
  saw it on a screen; now it reaches a whole team's phones, and it eats into the 20/min ceiling too
- **Fix — the `in_date_or_time()` guard, without touching the cascade:**
  * `extract_otp` went from `captures()` to **`captures_iter()`** — a date earlier in the message
    **no longer hides a real code** that comes after it
  * `FIELD_SEPARATORS` = `/ : - .`. A separator **on its own** is not grounds for rejection —
    what rejects is a **1–2 digit field** on the other side of the separator.
    That keeps `G-483920` (Google), `code 483920.` and `1234-5678` (a code sent split in two)
    **working**
  * The byte index can be checked directly: no byte of a multi-byte UTF-8 sequence is ever equal
    to an ASCII separator or digit — so Myanmar script is never misread as a date
- **The gate and the cascade were left untouched** (`05 §B.1` hard refusal) — the guard is
  **only a filter**, not a new pattern
- **Four tests:** `a_login_notification_with_a_date_is_not_an_otp` (a real Myanmar-script
  message), `a_date_does_not_hide_a_real_code_later_in_the_message`,
  `a_year_is_rejected_at_either_end_of_a_date` (both `2026/09/03` and `09/03/2026`),
  `a_dash_or_dot_next_to_a_code_is_still_a_code`
- **Rule 1:** The keyword gate **does not mean** "this message contains a code" —
  it only means "this message talks about a code". Notifications, warnings and advertisements
  carry the keyword themselves
- **Rule 2:** `\b` **does not separate a digit run from its context**. If you write a bare-digit
  pattern, you have to inspect the surroundings yourself
- **Rule 3:** `captures()` (the first match) **creates loss** whenever a pattern can match more
  than once — once the date is rejected, it has to carry on to the remaining matches

## 2️⃣2️⃣ `extract_otp` reads the Call Center phone number `3211` as an OTP

- **Symptom (field, 2026-09-04 01:12):** KBZPay's **logout notification** was forwarded into the
  Telegram group and the OTP badge showed **`3211`** — that is KBZPay's **Call Center
  number**. The same body appeared as 2 bubbles (whether the message itself arrived twice
  has not been checked — the forwarder's `Amend` path only **edits** when the id is the same, so it
  creates no new bubble, which makes two ids the likely explanation)
- **What is in the message:** `... please change your PIN immediately or contact KBZPay Call
  Center 3211.` and `... employees will never ask for personal information such as
  OTP, PIN, or NRC ...` — **there is no code**
- **Root Cause — the same shape as §21, but a new guard is needed:**
  1. The `KEYWORD_RE` gate opened — "PIN" and "OTP" are present, but as the **warning line**
     *"will never ask"* (§21 Rule 1 confirmed again)
  2. From `PIN` to `3211` is **43 characters** — outside `P1`'s 24-character window ·
     `P2`/`P3` found nothing
  3. **`P4` (`\b[0-9]{4,8}\b`)** caught `3211` — a hotline has **exactly the same shape** as a
     code; only the words in front of it tell them apart
- **`in_date_or_time` cannot cover this** — there is no separator next to `3211`, only a space and
  a full stop. So what is needed is a **lexical guard, not a structural one**
- **`Memory/05 §B.1` had foreseen this one too:** "promotional SMS balances, dates and
  **phone-number fragments** start matching as OTPs" — the date (§21) and now the phone number
  (this one), both have really turned up in the field. What is left is **the balance**
- **Fix — `after_phone_label()` (guard #2), without touching the cascade:**
  * `PHONE_LABELS`: call center/centre · customer service/care · service center/centre ·
    hotline · hot line · helpline · help line · contact · call · dial · tel · telephone ·
    phone · **`KW_PHONE`**
  * `LABEL_FILLER`: `at` · `on` · `no.` · `no` · `number` · `is` · `us` · `our` · **`KW_NUMBER`** —
    for "Call Center **at** 3211", "hotline **number is** 3211", "contact **us** 3211"
  * **Not a window — a suffix match:** the text in front of the number **must end** in a label.
    With a window, a "call" anywhere in the message could veto an unrelated code
  * **`.` and `,` are not put into `LABEL_GLUE`** — they close a clause. "We blocked a
    call. 123456 is your code" **must still yield** the code
  * **Word boundary:** `hotel 3211` is not `tel`, `recall` is not `call`. But when a label's first
    char is **not** ASCII no boundary is required — since Myanmar script does not separate words
    with spaces, this is the only way to split off "phone number"
- **A full MSISDN does not need this guard:** `09…` is 11 digits, `P4` stops at 8, and `\b` does
  not match **inside** a longer digit run — that was already safe before. What is left is
  the **short code** (4–8 digits) only
- **`to_ascii_lowercase()` does not move byte indexes** — an ASCII case fold keeps the byte
  count the same and does not touch multi-byte sequences, so a match offset obtained from
  `normalized` refers to **the same text** in `lower`
- **Four tests:** `a_call_centre_number_is_not_an_otp` (the real KBZPay logout body),
  `a_call_centre_number_does_not_hide_the_real_code` (hotline first, code after),
  `a_labelled_number_is_rejected_however_it_is_written` (five kinds of label + the Myanmar
  "phone number 3211"), `a_label_in_another_clause_does_not_veto_a_code` (`.` · `,` · `hotel`)
- **Rule 1:** An OTP false positive is **not a pattern-fixing problem** — it is an
  add-another-filter problem. §21 and this one both only add a guard, and keep hands off
  `P1`–`P4` and the gate (`05 §B.1` hard refusal)
- **Rule 2:** In a lexical guard, **which punctuation goes into the glue** is the crux. Make `.`
  glue and a label in the previous sentence deletes the next sentence's code — and in this app
  **a false negative is worse than a false positive**: `forwardNonOtp` defaults to `false`,
  so when no OTP is found that message reaches Telegram **not at all**
- **Rule 3:** Bank/telco notifications put the hotline in **every message**. When `P4`
  catches bare digits, running into a hotline is **the normal case**, not a coincidence
## 2️⃣3️⃣ `SIM cleanup done. Deleted 14 | FAILED: 30/64` — counting 30 empty slots with no modem as failures

- **Symptom (field, v1.5.0 64-port run):** SIM cleanup ran across **all 64 ports** before Detect
  had been run. The status line read `SIM cleanup done. Deleted 14  |  FAILED: 30/64`. **The real
  failure count was zero** — those 30 were slots with no modem in them at all (Detect afterwards
  reported `34/64`, and there was a `Modem not responding` log for every one of them)
- **What this case costs the operator:** while standing in front of the bank debugging an
  incident, a line saying "30 failed" is **a working bank declaring itself broken** — the
  operator has to go hunting for 30 failures that do not exist, and the real problem is buried
  underneath them
- **Root Cause:** the cleanup worker's counter folded **every non-ok result** from `expire_old`
  and the panic arm into the **single** `failed` bucket. `modem::expire_old` returns `read_port`'s
  error **verbatim**, so the code **already knows** that probe silence means `NOT_RESPONDING` —
  it is only the counter that does not separate them. Both the status line and the
  `sim_cleanup:done` payload were built out of that single number (only `deleted`/`failed`)
- **The right shape already existed in the project:** `detect_done_status`
  (`src-tauri/src/commands/mod.rs:519`) has separated `Empty` from `Inconclusive` since #19 and
  says precisely `30 port(s) with no modem deselected` — **cleanup simply did not follow that
  shape**
- **Fix (shipped in v1.6.2):**
  * A third counter, `empty` (`commands/mod.rs:1704`), whose arm matches
    `r.error.as_deref() == Some(crate::core::modem::NOT_RESPONDING)` **exactly** (`:1734`) — that
    one string and nothing else. An `AtChannel::open` error or a `Serial I/O failed: …` is a
    **host-side fault**, so it stays in `failed`, and so does the panic arm (`:1745`)
  * That arm **writes no log line** (deliberately) — `modem::probe_failure` has already recorded
    the silence once for that port, so logging it again would double the log. The count reaches
    the operator through the status line and the event payload
  * A new builder, `cleanup_done_status(deleted, empty, failed, total)` (`:627`), sitting beside
    `detect_done_status`/`scan_done_status`; being a pure function, it is testable. **Every
    clause is omitted at zero**, so a clean run is one sentence and `FAILED` **never appears**
    when nothing failed. The field case now reads:
    `SIM cleanup done. Deleted 14 expired SIM slot(s)  |  30 port(s) with no modem`
  * **The word "deselected" is not included** (the point where it differs from detect) — cleanup
    does not change the port selection
  * `empty` in the `sim_cleanup:done` payload (`:1772`), and the frontend listener
    (`src/lib/services/api.ts:414`–`:432`) shows it as its own clause, **raises no Warning
    severity for empty** (Warning only when `failed > 0`), and stays **silent** when
    `deleted == 0 && failed == 0` (`:424`) — it fires unattended every 10 minutes, and empty
    slots are a steady state. `?? 0` (`:419`) for a backend whose payload carries no `empty`
- **The first field sighting of `05 §C.10`:** C.10 had already recorded in a table that the four
  supervisors have four different failure-accounting policies (cleanup = "increments the failure
  counter"), but **this is the first time it actually reached an operator**. So this fix is not a
  detour — it is a **down payment** on C.10
- **Four tests** (`commands/mod.rs:1902`–`:1938`):
  `cleanup_status_stays_one_sentence_when_there_is_nothing_to_report`,
  `cleanup_status_counts_empty_slots_without_calling_them_failures` (the real field case, which
  also asserts `!contains("FAILED")`),
  `cleanup_status_reports_real_failures_against_the_port_total`,
  `cleanup_status_keeps_empty_slots_and_failures_apart`
- **Rule 1:** **Never put "empty" and "broken" into one counter.** This is the **counter
  version** of §16 Rule 1 ("don't know" must not be folded into "not there") — in this repo half
  a bank being empty is normal
- **Rule 2:** **Do not show a zero clause.** `FAILED: 0` all by itself sends the operator
  searching — only add the clause when something really failed
- **Rule 3:** Before writing status wording, **look for an honest pattern that already exists**.
  Detect had already made this distinction, so cleanup had something to copy, not something to
  invent

## 2️⃣4️⃣ The status line still reads `Live 34/34 ready` after a worker has died — two numbers on one screen that do not agree

- **Symptom:** when a live worker exits altogether (the transport dies, or it panics) the port row
  turns red, but the status line **keeps counting that port inside `Live N/N ready`**
- **Two counters on one screen that do not agree:** the `Closed` arm emits `ports:updated`, so the
  frontend **recomputes `readyPorts` itself** from `ports.filter(p => p.live_ready)`
  (`src/lib/services/api.ts:242` listener, derive `:248`–`:250`) — so the badge
  (`Toolbar.svelte:39`) said `Live 33/34` (correct) while the footer status line coming out of Rust
  said `Live 34/34 ready` (wrong). The operator is looking at both numbers at the same time
- **v1.5.0 made it worse in two ways:**
  1. **`statusText` was put onto the Inbox footer** (`05 §C.8`) — a wrong number that used to be
     visible only on the Ports page is now **on the main page the operator works on**
  2. **worker panics were made to report through `Closed`** (`live::WORKER_PANIC`) — so the panic
     path that became newly reachable in v1.5.0 leads straight into **the one arm that does not
     decrement the ready count**
- **Root Cause:** the `Closed` arm wrote `p.live_ready = false`, set `p.live_error` and did
  `st.live_failed.push(...)` — but **never touched `live_ports_ready`**, and instead of calling
  `live_status` again it **replaced `status_text` outright** with `"{port} FAILED: {e}"`. So by the
  time some later `Ready`/`Offline` event recomputed the line, the port that had dropped out was
  still sitting in the ready list
- **And `live_failed` was write-only state** — pushed in two places, cleared in `start_live`, with
  **no reader at all**. So the app recorded the fact that a worker had died without that fact
  entering any number
- **Fix (shipped in v1.6.2) — recognising the bucket as state:**
  * An `N failed` clause added to `live_status` (`src-tauri/src/commands/mod.rs:543`), in the order
    ready → `no modem` → `failed` → `connecting…`
  * **`connecting…` is not a bucket — it is the remainder** `total - (ready + offline + failed)`
    (`:554`), so it can only be computed **last**. That is exactly why the three buckets **must be
    disjoint** — count a port twice and the thing that starts lying is that remainder
  * `mark_port_failed(st, port, reason)` (`:579`) — **the single place where the bucket rules
    live**. It removes the port from the ready list and from `live_offline` first and only then
    pushes onto `live_failed`, and it pushes only if the port is not already there
  * **Rule 1 — dedup per port:** `run_live` catches its own panic and reports it as `Closed`, and a
    panic that escapes that is reported **again for the same port** by the worker's **outer
    `catch_unwind`** (`:1334`) — two entries and the line announces a failed count higher than the
    bank has ports. The first reason is kept (that one is specific, the second is a generic
    backstop), and the row's `live_error` already shows the latest text
  * **Rule 2 — "failed" beats "no modem":** when the worker of a silent slot dies, **nothing is
    retrying that port any more**, and leaving it in both buckets counts one port twice. The
    reverse (offline after failed) cannot happen — a worker emits no more events after `Closed`
- **Three arms were fixed:**
  | Arm | What it does | Which bucket |
  |---|---|---|
  | `LiveEvent::Closed` (`:1297`) | `mark_port_failed` (`:1314`) + rebuild `live_status` (`:1315`) | `live_failed` |
  | Outer `catch_unwind` (`:1334`) | `mark_port_failed` (`:1351`) + `live_status` (`:1352`) | `live_failed` (the dedup keeps it from incrementing twice) |
  | `LiveEvent::Reconnecting` (`:1141`) | drops from the ready list only (`:1155`) + `live_status` (`:1156`) | **no bucket at all** → the `connecting…` remainder |

  Reconnecting is transient, so landing in `connecting…` is **the true state**, and when the
  `Ready` arm comes back round it puts the port back itself
- **The hand-written `status_text` override was deleted in all three.** The reason is not lost — it
  arrives as the row's `live_error` inside the same arm's `ports:updated` payload, and
  `Ports.svelte` and `PortDetail.svelte` render it. The status line is a **bank-wide count**, so it
  must not carry one single port name
- **`p.alive` is untouched in all three** — only probe silence is allowed to style a slot as
  "empty" (§16, AGENTS.md invariant)
- **Deliberate decision — a dead worker is not counted as `no modem`:** a worker dying says
  **nothing at all** about whether there is a SIM in the slot. Putting it into `live_offline` would
  create a new falsehood (claiming 30 empty slots) — which is why a new bucket was needed
- **This entry supersedes the `05 §C.6` (L3) latent trap** — that entry is now
  `✅ DONE (v1.6.2)`, and the evidence is kept there for the history
- **Five tests** (`commands/mod.rs:2167`–`:2250`):
  `live_status_moves_a_reconnecting_port_into_the_remainder` (which also asserts that both offline
  and failed are empty), `live_status_drops_a_port_whose_worker_closed`,
  `a_port_reported_failed_twice_is_counted_once` (including that the first reason is kept),
  `a_silent_port_whose_worker_died_is_counted_as_failed_only`,
  `live_status_reports_every_bucket_it_has` (`Live 1/5 ready | 1 no modem | 1 failed | 2 connecting…`)
- **Rule 1:** **Where there is a number computed as a remainder, the buckets have to be disjoint** —
  and do not hold that disjointness together by hand in each individual arm, **push it into one
  helper**. Copy two rules across three arms and the fourth arm will forget them
- **Rule 2:** **Write-only state is the handwriting of a bug** — pushed to `live_failed`, cleared
  again, with no reader, tells you "this fact is not part of any number". It is greppable: if a
  field has a site that writes it and no site that reads it, take notice
- **Rule 3:** When the UI and the backend **compute the same number separately**, the disagreement
  on screen only shows up later. The frontend deriving from `p.live_ready` happened to be right
  here — but "one side is right" is not a fix, it is only **evidence that the other side is
  wrong**

## 2️⃣5️⃣ `read -> 2 msg(s)` followed by `deleted 5 msg(s)` — a log that is correct but looks wrong

- **Symptom (field, v1.5.0 run):** two log lines whose numbers look like they do not add up:

  ```
  COM39: pdu-mode read -> 2 msg(s)
  COM39: deleted 5 msg(s)
  ```

  **Both of them are correct** — what goes wrong is the reading
- **That is exactly where this case lies in wait:** a correct log that looks wrong. The operator
  goes hunting for a data bug that does not exist, and once "is this log even true?" has been
  asked, **the instrument of field debugging is itself broken** — that is the loss
- **Root Cause — one unit, `msg(s)`, standing for two different things:**
  | | What it counts | Evidence |
  |---|---|---|
  | Read side | **the reassembled row** (`msgs.len()`) | `src-tauri/src/core/modem.rs:379` (pdu), `:400` (text) |
  | Delete side | **the SIM slot** (`gone.len()`) | `modem.rs:549`, partial `:558`, the form that cannot confirm `:531` |

  A concatenated SMS eats one slot per fragment, so **2 rows = 5 slots** is perfectly normal
- **The status line had separated them all along:** `Deleted 1 message(s) (1 SIM slot(s) freed)`
  (`commands/mod.rs:1566`) writes the two units two different ways — **it was only the per-port log
  line that was left behind**
- **Fix (shipped in v1.6.2) — the convention:** the noun is always **`slot(s)`**, qualified as
  **`SIM slot(s)`** only in the operator-facing status line. Eight sites:
  * three lines in `modem.rs::confirm_delete` (`:531`, `:549`, `:558`)
  * three lines in `commands/mod.rs` — the cleanup status line (`:628`), the per-port cleanup log
    (`:1722`), and `Deleted N slot(s) from PORT` (`:1538`)
  * two lines in `core/live.rs` — the live worker's retention sweep (`:284` initial, `:381`
    periodic)
- **Deliberately left alone (because they really are rows):** `Auto-purged N expired message(s)`
  (`commands/mod.rs:1652` — inbox rows), `Deleting N message(s)...` (`:1507` — the selected rows),
  and the two status lines that already write both units (`:1566`, `:1572`)
- **The plan (`07 §D.1`) had recorded `modem.rs` only** — the remaining five sites (three in
  `commands/mod.rs`, two in `live.rs`) were found during review and added. Fix one file only and the
  unit is still split between files, and the live sweep's line is `SIM cleanup deleted N …`, which
  is something you have to read side by side with the cleanup status line
- **`confirm_delete`'s doc comment (`modem.rs:517`–`:521`) records this gap** — whoever writes the
  next count log has to be able to read out of the code itself which one is a row and which one is a
  slot
- **Rule 1:** When you log a count, **pair it with a unit that says what is being counted**. Two
  different kinds of number under the same unit name are not a wording problem — **they look like a
  data bug**
- **Rule 2:** **A log that looks wrong is itself a bug.** Field debugging rests on trusting the log,
  so "the code is correct" does not answer this case
- **Rule 3:** When you change a unit, **grep for every site** (`msg(s)` · `message(s)` ·
  `slot(s)`) — fixing one file does not remove the gap, it **moves** it

## 2️⃣6️⃣ Two `USSD *88# rejected (+CME ERROR: 100)` lines, 15 ms apart — not a duplicate log

- **Symptom (field, v1.5.0 run):**

  ```
  18:39:31.846  COM38: USSD *88# rejected (+CME ERROR: 100)
  18:39:31.861  COM38: USSD *88# rejected (+CME ERROR: 100)
  18:39:36.585  COM38: SIM number ***573      ← the *124# fallback succeeded
  ```

  Two lines identical to the character — **it reads as a duplicate-logging bug, and it is not one**
- **Root Cause — the two attempts really are different, it is only the log that does not separate
  them:** `ussd_query` (`src-tauri/src/core/modem.rs:805`) sends `AT+CUSD=1,"*88#",15` first
  (`:810`), and on `Rejected` it cancels the session with `AT+CUSD=2` (`:814`) and retries the
  **bare form `AT+CUSD=1,"*88#"`** (`:815`). `ussd_attempt` (`:844`) accepts both `command` and
  `code` in its signature, but its three warn lines wrote **only `code`**
- **Why it matters:** hunting for the firmware that rejects `,15` (the DCS argument) is **the sole
  purpose of this retry** (comment `:806`–`:809`). So a log that cannot show which form was rejected
  **destroys the purpose of the retry itself** — the retry works, but the firmware pattern cannot be
  learned from the field log
- **Fix (shipped in v1.6.2):**
  * A helper, `ussd_form(command)` (`modem.rs:837`) — it pulls out **the argument after the closing
    quote** of the dial string (`rsplit_once('"')`), and yields `"bare"` when it is empty or absent
  * Added to the three warn lines: rejected (`:851`), `no reply within {}s` (`:866`) and
    `replied without a number` (`:887`). It now reads like this:

    ```
    COM38: USSD *88# (,15) rejected (+CME ERROR: 100)
    COM38: USSD *88# (bare) rejected (+CME ERROR: 100)
    ```

  * **`,15` is not hardcoded** — it is read out of the command that was actually sent, so even if
    `ussd_query`'s DCS changes, the log will never name an argument the modem was **never asked
    for**
  * **No change to behaviour, to the AT sequence or to timeouts** — wording only
- **No change to the privacy surface:** what newly reaches the log is only the **argument** of
  `AT+CUSD` (`,15` / `bare`), and the dial string is the carrier code the operator typed in
  themselves. `AT+CUSD`'s **reply body (which carries the subscriber's own number) stays at debug**
  (`:893`) — the logging refusal in AGENTS.md
- **Two tests** (`modem.rs:1199`, `:1209`): `ussd_form_distinguishes_the_two_attempts` (`,15` vs
  `bare`, plus an `assert_ne!` that the two must not be equal), and
  `ussd_form_reads_the_argument_as_sent` (`,0`, `, 15`, nothing at all after the dial string,
  `AT+CUSD=2` — each of them has to show up as itself)
- **Rule 1:** **When a retry is testing a hypothesis, the log has to say which hypothesis was
  tested.** Without it the retry still works but **teaches nothing** — and on a hardware fleet that
  is half the value of the retry
- **Rule 2:** When you see two identical lines, **do not assume "duplicate logging" first** — the
  gap between the timestamps (15 ms here) is the evidence that there were two attempts. Which means
  a log line must **write distinct work distinctly**

## 2️⃣7️⃣ The Linux tray menu opens as a blank rectangle with no labels — **fixed in v1.8.0**

- **Symptom (Ubuntu GNOME, on the pre-release build between `v1.7.0` and `v1.8.0`):**
  Clicking the tray icon opened the popup menu as a blank dark/blue box with no visible
  text where `Open SMS Reader` and `Quit` should be, while other apps on the same desktop
  (Antigravity IDE, Cloudflare WARP) drew crisp labels. Shipped fixed in `v1.8.0` as
  `fix(tray): persist tray menu state and guard left-click menu suppression for windows`
  (`80a83bb`, PR #37)
- **Root cause — two of them, in the same few lines:**
  1. **The `Menu` was a local in `.setup()`.** Under GTK3 + `libayatana-appindicator` the
     Rust `Menu` owns the GTK widget tree, so dropping it at the end of `setup` let the
     widgets be finalised while the AppIndicator stayed registered. GNOME Shell
     (`ubuntu-appindicators`) then asked over DBusMenu (`com.canonical.dbusmenu`) for the
     layout and properties and got empty, un-synchronised labels back. **The menu drew,
     because the indicator was still there — it just had nothing to say**
  2. **`.show_menu_on_left_click(false)` was called unconditionally.** Upstream `tray-icon`
     documents it as `Linux: Unsupported`; on Linux it fought the AppIndicator DBus
     service's own event handling
- **Fix:**
  1. `app.manage(menu)` — the menu and its GTK widget tree stay alive for the whole process
     lifetime. Nothing ever reads that state back; keeping it alive **is** the reason it is
     there, which is worth a comment to anyone who later tries to tidy it away
  2. `.show_menu_on_left_click(false)` moved behind `#[cfg(target_os = "windows")]`, so
     Windows keeps restore-on-left-click and Linux keeps its native menu-on-left-click
  3. Labels shortened to ordinary desktop actions (`Open SMS Reader`, `Quit`)
- **Rule 1:** **A Linux tray or context menu has to be owned by something that outlives
  `setup`.** "It compiles and the icon appears" is not evidence the menu survived — the
  indicator and its contents have separate lifetimes, and only the contents went missing
- **Rule 2:** A platform-specific tray builder flag is `#[cfg]`-guarded to the platforms
  that document support for it. Check the upstream doc line before calling one unguarded;
  `tray-icon` states the unsupported platform for each
- **Rule 3:** This class of bug **cannot be caught by any gate in this repo** — it needs a
  desktop shell, an AppIndicator host and a human looking at a menu. It is a
  `04 §G` hardware-playbook check, not a CI check

## ⚠️ Latent Traps (not happened yet, but lying in wait)

Not cases yet — three things found in passing while doing the 2026-08-30 settings cleanup
(`fbd7b8b`) (T1–T2 in the settings layer, T3 in the decoder), plus T4 (retention layer) and T5
(busy-flag layer) out of the 2026-08-31 audit, plus T6–T8 out of the 2026-09-05 review of the
shipped v1.8.0 tray.
There was no symptom, so they were not fixed, but each of them could become a bug that is hard to
explain later.
**T3 was fixed in code in v1.4.0, and T4/T5 in v1.5.0** — T1/T2 and T6/T7/T8 are still unfixed.
T6–T8 were reported to the operator and **deliberately deferred**: the v1.8.0 audit was a check,
not a change, and the fixes are `fix:`-class work waiting on a v1.8.1 decision.

### T1. `deepMerge` iterates the **stored** keys — a deleted setting never disappears from a profile

The loop in `deepMerge(target, source)` in `src/lib/stores/settings.svelte.ts` is
`Object.keys(source)` — `source` is the **stored profile** coming out of `localStorage`
(`sms-reader-settings`), `target` is the defaults. So the merge is not restricted to "keys that exist
in DEFAULT" — **every** stored key lands in the result.

**Consequence:** deleting a field from `SettingsState` does **not clean it out of** an existing
user's profile. `otp.otpPattern`, `developer.logLevel` and the rest stay in localStorage, and since
`saveSettings` stringifies the whole object they get **written back on every save**.

- **Harmless so far** — because nobody reads them
- **The trap ahead:** a new field carrying the same name (for example reintroducing
  `developer.logLevel` in a different shape) would **inherit the stale value** — not the default.
  This happens only on that user's machine, so it is a "it doesn't happen for me" class of bug
- **Rule:** never recycle a field name with changed semantics. If it is genuinely needed, `delete`
  the key explicitly inside `migrate()` (the same shape as the retention migration). `deepMerge`
  could also be changed to iterate by DEFAULT key — but that means every migration for the legacy
  shapes has to run first

### T2. The Settings page is data-driven, and the binding path has **no type check**

`src/lib/pages/Settings.svelte` renders out of an array of field descriptors
(`{ key, label, type, bind, … }`), and value access goes through
`getNestedValue(obj: any, path: string): any` / `setNestedValue(obj: any, …)` — where the path is
the string concatenation `` `${field.bind}.${field.key}` ``.

Which means **a `bind`/`key` pair that does not exist raises no compile error**, and `svelte-check`
does not catch it either — at runtime it is simply `undefined`. For a checkbox that means "it always
looks off", and if `setterFor(field.bind)` returns `undefined` nothing persists.

- The eleven fields deleted in `fbd7b8b` turning out type-safe was **coincidence, nothing more** —
  delete from `SettingsState` and leave the descriptor behind and the build succeeds while the
  switch quietly does nothing
- **Rule:** when you add or delete a control, **click through the Settings page yourself** — does
  the value survive a reload, does the consumer actually respond? The type checker does not protect
  this layer (doc 04 §H)

### T3. The `KW_CONFIRM` keyword constant was misspelled (OTP gate — **fixed in v1.4.0**)

**How it used to be (kept for context):** the keyword constant in
`src-tauri/src/core/decoder.rs` was

```rust
const KW_CONFIRM: &str = "\u{1021}\u{1010}\u{1014}\u{103A}\u{1015}\u{103C}\u{102F}"; // = a-ta-na-pyu
```

`\u{1014}` is **na**. Myanmar "confirm" is **a-ta-nya-pyu** — it has to be `\u{100A}` (**nya**). So
this alternative inside the `KEYWORD_RE` gate **never matched a real SMS** (a spelling that does not
exist).

- **Impact at the time:** a Myanmar OTP SMS using **"a-ta-nya-pyu" and nothing else**, with none of
  the other keywords (`otp`, `code`, `pin`, the Myanmar `KW_KODE`/`KW_SECURE` words, `verify`…), did
  not get through the gate → no OTP found (a silent miss — the decoder returns `None`, so no error
  is raised)
- Not one unit test touched this constant, which is why a green test suite did not catch it
- **Now fixed (v1.4.0):** `src-tauri/src/core/decoder.rs:7` uses `\u{100A}`, so the constant now
  spells **a-ta-nya-pyu**. A regression test came with it:
  `src-tauri/src/core/decoder.rs:943` `otp_myanmar_confirm_keyword` — `extract_otp` must return the
  OTP for a body containing "a-ta-nya-pyu", and must return `None` for **the same body with the
  keyword removed** (a negative control — the evidence that it really is this keyword that opens the
  gate)
- **Kept here as a trap all the same** — the bug class remains. **A Unicode escape sequence inside a
  keyword constant cannot be checked by eye**: `\u{1014}` ↔ `\u{100A}` differ by a single code
  point, nobody sees it in a diff or a review, and both are perfectly valid to the compiler and to
  the regex. Only **a test that asserts on the rendered characters** can catch it — not rereading
  the escape
- **Rule:** every time you add a new Myanmar (or other non-ASCII) keyword constant, add a test with
  **one body that must match it and one that must not**. Checking whether the constant on its own is
  correct is not enough — make it run the whole gate

### T4. `retentionHours` reaching Rust unclamped and panicking (retention layer — **fixed in v1.5.0**)

**Symptom (never seen in the field — reproduced with a test):** when `retentionHours` is around
`1e13`, `purge_expired_messages` / `cleanup_sim_storage` panic. Worse still is `start_live` — at
`live.rs:265`/`:361` every single port panics and `catch_unwind` converts it into
`Closed { error: "Worker crashed" }`, so **live mode stops on every port with no explanation**. The
process does not die (`panic = "abort"` is not set), only the feature does

- **Root Cause in two stages:**
  1. `retention_from_hours` guards `!h.is_finite()` and `h <= 0.0` but **does not guard the upper
     end**. `Duration::from_secs_f64` panics on overflow (`h` ≳ 5.1e15)
  2. If that gets through, `chrono::Duration::seconds(i64::MAX)` in `models::retention_cutoff_ms`
     panics with "out of bounds" (once past `i64::MAX / 1000` seconds)
- **How it gets there:** the Settings UI is a fixed-option `select`, so it cannot arrive from the UI.
  But the store rehydrates out of `localStorage` — it arrives from an old profile, a hand edit or a
  corrupt entry. `api.ts` only checks `<= 0`. And the app's purge timer calls back in **every 60
  seconds**, so once it happens it keeps happening
- **Fix (v1.5.0):** `MAX_RETENTION_HOURS = 87_600.0` (10 years) — past that it returns `None`. A
  retention window over 10 years is **semantically the same thing** as "keep everything", so it lines
  up exactly with the existing "0 = off" precedent — better than returning an error, because a wrong
  value here is **ordinary** input, not an exception. `Duration::try_from_secs_f64` is used.
  `retention_cutoff_ms` as a backstop saturates at `i64::MIN` using `try_seconds` +
  `checked_sub_signed` — the safe answer being "nothing has aged out yet"
- **Test:** `an_absurd_retention_window_is_off_not_a_panic` (commands),
  `an_absurd_retention_window_saturates_instead_of_panicking` (models)
- **Rule 1:** When you guard a numeric setting, **do not guard only the bottom end**. Having checked
  `<= 0` it is easy to think it is "guarded" — but the panic is at the top end
- **Rule 2:** Clamp or reject **in Rust** every value that comes out of `localStorage`. The frontend
  is not a validator: the Settings number input puts `min`/`max` on the DOM element only, and
  `onchange` stores whatever `parseInt` returned, unclamped
- **Rule 3:** Catching a panic from a worker thread with `catch_unwind` **hides the root cause**. A
  `Worker crashed` message resolves nothing for the operator — keep the panic-capable input out of
  the worker in the first place

### T5. A busy flag not cleared on the panic path — "Busy" until a restart (busy-flag layer — **fixed in v1.5.0**)

**Symptom (never seen in the field):** if an operation panics, `port_busy()` stays `true` forever →
Scan / Live / Get SIM / Delete / Cleanup / Detect **all** return `Busy`, and nothing recovers until
the app is restarted. The log holds one panic, and the UI shows nothing at all about why everything
is shut

- **Root Cause:** all six busy flags were cleared with an **ordinary statement** (`:369, :506,
  :642, :1051, :1222, :1395`) — not on the unwind. So a panic outside the per-port `catch_unwind`
  leaves the flag set. The two clearest instances:
  1. **`delete_selected`** — the `catch_unwind` wraps the modem loop only. A panic in the
     bookkeeping after it (`confirmed_removals`, computing `kept`) leaves `delete_busy` set
  2. **`start_live`** — the supervisor thread had **no `catch_unwind` whatsoever**, and it owns
     `live_on` **and** `live_stop` both. A panic between the join and the clear leaves both set —
     and since `live_stop.is_some()` is part of `port_busy()`, the gate is shut twice over
  3. The live per-port worker had no `catch_unwind` either — on a panic `join()` swallows it
     silently, the port is not marked failed, and **the LIVE badge stays green while catching not a
     single message**
- **Fix (v1.5.0) — `BusyGuard` (a Drop guard):**
  `{ state: SharedState, clear: fn(&mut AppStateInner) }`. Because `lock_state` is
  poison-recovering, the lock is still obtainable after a panic (that is what makes the guard work
  at all). It is the **repo's first `impl Drop`**
- **Important — the happy path is untouched:** the existing inline clears were left as they are.
  Some commands do extra work inside that same lock (`sim_dir.save()`, building the status line) —
  changing that ordering is not this guard's job. Which means `clear` has to be **idempotent**, and
  the guard only really does work **on the panic path**
- **Construct the guard in the command and `move` it into the closure** — that way, even if
  `thread::spawn` itself panics, the closure (and the guard) is dropped during that unwind and the
  flag is released. Construct it **inside** the closure and this case slips through
- **The `AppStateInner` shape does not change** (the guard holds an `Arc`) — which is why the three
  struct literals (`new_shared_state`, `idle_state`, `live_state`) and
  `port_busy_covers_every_operation_that_owns_a_port`'s `[fn(&mut AppStateInner); 6]` array compile
  unchanged. **That is by design**
- **It makes a live worker panic visible:** `live::WORKER_PANIC` ("Live worker crashed — see the
  log") is put into `live_error`, the port is pushed into `live_failed`, and `ports:updated` is
  emitted. **Do not confuse it with `modem::NOT_RESPONDING`** — only that one is allowed to set
  `alive = Some(false)`, and a worker crash says nothing about whether there is a SIM in the slot
- **Four tests:** `busy_guard_releases_the_gate_on_an_unwind`, `..._on_a_normal_return`,
  `busy_guard_is_a_no_op_once_the_flag_is_already_clear` (the guard must not touch another
  operation's flag), `busy_guard_releases_both_live_flags`
- **Rule 1:** **"it is cleared on every exit path" has to be structural, not something you remember
  in five or six places.** Give the flag an owning type and "forgetting" stops being a state the
  code can compile into
- **Rule 2:** Check the **extent** of a `catch_unwind`. Wrapping only the loop does not cover the
  bookkeeping outside the loop — `delete_selected` is that very case
- **Rule 3:** **Write down what is not in scope as not in scope.** Merging the four supervisors into
  a `run_port_pool` is not part of this change: the panic policies are **four different ones**
  (scan: push `failed_notes` + increment `done` · ussd: increment `done` only, so a port that
  panicked reads as "not found" · cleanup: increment the failure counter · detect: **since v1.5.0
  (#19) `ProbeVerdict::Inconclusive` — `alive`/`checked`/`iccid`/`sim_dir` untouched**; the old
  behaviour of marking it dead and clearing the `sim_dir` slot is this very case, §16). Merging them
  means either picking one policy or adding a per-port on-panic callback — and since that is a
  behaviour-normalising refactor, it is a separate change. **Deliberately deferred; the entry is doc
  05 §C.10**

### T6. The tray's `Quit` is `app.exit(0)` — the forwarder flush never runs

`lib.rs`'s `"quit"` menu arm calls `app.exit(0)`. The Telegram queue's flush lives somewhere
else entirely: in `start_live`'s supervisor thread, after `for h in handles { h.join() }`, at
`commands/mod.rs`'s `if let Some(f) = forwarder { f.shutdown(); }` — whose own comment says why
it is there, *"Flushing here rather than dropping the queue means a code that arrived a second
before Stop still lands"*. `app.exit(0)` ends the process, so the supervisor never joins, never
reaches that line, and the queue goes with it.

**Consequence:** press *Quit* while Live is running and every item the forwarder had not yet
paced out is lost silently — no error, no log line, no Telegram bubble. The window is
`MIN_INTERVAL` 3.5 s per send with `MAX_BATCH` 10 items each, so a burst that just arrived is
exactly what is still sitting there. `AGENTS.md` already states the invariant this crosses:
*"`stop_live` must call `Forwarder::shutdown` — it flushes the remainder as one last message and
joins; dropping the handle does not."* `stop_live` still honours it. The tray does not go
through `stop_live`.

- **Not a regression** — before v1.8.0 the window's close button ended the process the same way,
  with the same loss. What v1.8.0 changed is that there is now a labelled *Quit* button, on a
  tray icon, on an app that is *designed* to be left running unattended. The operator will press
  it mid-session, which is the one case that loses codes
- **The fix, when it is taken:** have the `"quit"` arm run the orderly stop (`stop_live`, or at
  minimum the forwarder shutdown) and only then `app.exit(0)`. Note that `stop_live` can take up
  to ~15 s if a worker is parked in `AT+CMGL=4`, so Quit cannot be synchronous-and-instant *and*
  lossless — that trade-off is the actual design question, not the plumbing
- **Rule:** **every path that ends the process is a path that has to flush.** Count them before
  adding one. A queue whose flush lives in one supervisor is only as safe as the number of ways
  the process can die without reaching it

### T7. Close-to-tray with no single-instance guard — two processes, one bank

`minimizeToTray` defaults to `true`, so `✕` hides the window and the process keeps every serial
port open. There is no `tauri-plugin-single-instance` anywhere in the dependency graph
(`cargo tree` and a source grep both come back empty), so nothing stops a second launch.

**Consequence:** an operator who believes they closed the app double-clicks the icon and now has
two processes. The second one cannot open a single port — the first still holds them — so it
probes every slot into `ProbeVerdict::Inconclusive` (the v1.5.0 `#19` behaviour, §16) and
presents a bank that looks entirely dead, while the first copy is invisibly still reading SMS
and forwarding it. The operator is now debugging the wrong process. Before v1.8.0 this could not
happen: `✕` ended the process, so there was never a first copy to collide with.

- **Worse on Linux**, where the tray icon needs an AppIndicator host in the shell. On a desktop
  without one the window vanishes with no icon to bring it back, so relaunching is the *obvious*
  thing to do — see README's Linux requirements note
- **The fix, when it is taken:** add `tauri-plugin-single-instance` and have the second launch
  show and focus the existing window instead of starting a run
- **Rule:** **the moment a process can outlive its last window, "what happens on the second
  launch?" becomes a question the design has to answer.** For an app that owns exclusive
  hardware, the answer cannot be "two of them"

### T8. `tray-icon` is enabled by `linux-libxdo`, not by this crate

`src-tauri/Cargo.toml` declares `tauri = { version = "2", features = ["linux-libxdo"] }` and
never asks for `tray-icon`. `tray-icon` is not one of tauri's defaults either — those are
`wry, compression, common-controls-v6, dynamic-acl, x11, dbus`. The tray compiles because
tauri's `linux-libxdo = ["tray-icon/libxdo", "muda/libxdo"]` is written **without** the `?`
sigil, so naming that optional dependency force-enables it. `cargo tree -e features` shows the
chain outright: `tauri feature "linux-libxdo"` → `tauri feature "tray-icon"`.

**Consequence:** the feature this app's tray depends on is switched on by a side effect of an
unrelated feature, in a crate we track with a caret range. If upstream ever rewrites that entry
in the idiomatic form (`tray-icon?/libxdo`) — exactly the kind of thing a maintainer tidies —
`tauri::tray` disappears and `lib.rs` stops compiling.

- **Loud, not silent** — it is a compile error, which is the good version of this problem. But
  because CI runs `--locked`, it will not appear on the branch that caused it: it appears on
  whoever next regenerates `Cargo.lock`, or on a release PR
- **The fix costs nothing:** add `"tray-icon"` to the feature list. It is already enabled, so
  `Cargo.lock` does not change and no new crate is pulled
- **Rule:** **depend on what you use, by name.** A feature that arrives through someone else's
  feature list is a dependency you did not declare and cannot see in your own manifest

## Bonus UX Notes

- **User-reported 404 on releases page** but server-side probes said HTTP 200 (public repo) →
  browser cache/transient CDN — cross-check with a hard refresh, incognito or the direct tag URL.
- **`gh` CLI:** without `export GH_PAGER=cat PAGER=cat` the interactive pager can block
  (non-interactive shell trap); for long polls use the background process + `/tmp/*.txt` capture
  pattern.
