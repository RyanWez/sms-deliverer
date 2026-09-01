# AGENTS.md — SIM Bank SMS Reader (sms-tauri)

Rules for every agent and subagent in this repo. Tauri v2 + Rust (`src-tauri/`) + Svelte 5/Tailwind/TypeScript (`src/`). Desktop app reading SMS/OTP from multi-port GSM SIM banks over serial AT commands.

`Memory/*.md` is the long-form knowledge base and is written in Burmese. This file stays English: it is what an agent reads first and it has to be unambiguous to any model. Cross-reference Memory instead of restating it.

## Golden rules
1. Conventional Commits always: `fix:` patch · `feat:` minor · `feat!:`/`fix!:` major · `chore:`/`docs:`/`ci:`/`build:`/`refactor:`/`perf:`/`style:` no release. Lowercase imperative subject; the body becomes release notes.
2. The version is written in FOUR files, by release-please and nothing else: `package.json` (source of truth, `release-type: node`), `.release-please-manifest.json`, and the two `extra-files` in `release-please-config.json` — `src-tauri/tauri.conf.json` `$.version` and `src-tauri/Cargo.toml` `$.package.version`. A fifth place, the `sms-tauri` entry in `src-tauri/Cargo.lock`, cannot be written by release-please; the `sync-cargo-lock` job pushes it onto the release branch. Never hand-edit any of the five. Tags are plain `vX.Y.Z`.
3. Never put secrets/tokens/keys in chat, code, commits or logs. OTPs and message bodies never reach an enabled log level — see Logging.
4. Verification loop, never skipped: edit → local validation → review diff → commit → push → watch CI → verify merged effect.
5. Fix root causes, not symptoms. Real bug fixes get a `Memory/03` entry (symptom → root cause → fix).

## Git and branch workflow
Detail: `Memory/06-git-workflow.md`. Non-negotiable parts:
- Unfinished work never lands on `main`. `release-please.yml` runs on every push to `main` and turns every conventional commit there into a pending release PR — a half-done `feat:` on `main` is a queued release.
- Work on `<type>/<kebab-topic>` (`fix/serial-write-vs-silent-modem`, `ci/frontend-typecheck`): lowercase, `-` separator, one branch one concern. Never touch `release-please--branches--main--components--sms-tauri`; it is bot-owned.
- PR into `main`, squash-merge. On squash the **PR title** becomes the commit subject on `main`, so the PR title must itself be a valid conventional commit. If it is not, release-please cannot read it: no version bump, no changelog entry, however carefully the branch commits were written. Fix with `gh pr edit <n> --title` before merging — afterwards it needs a history rewrite.

## Subagent discipline
- Agents never run `git commit`, `git add`, `git push` or any state-changing git command. Read-only inspection only; the orchestrator commits.
- Agents never create markdown or documentation files unless that is the assigned task.
- Stay in your file lane. Never revert or overwrite another agent's uncommitted work — an unexpected diff outside your lane belongs to someone else.

## Validation, and what is actually enforced
```
npm run check     # svelte-check, currently 0 errors 0 warnings
npm test          # node:test + --experimental-strip-types, 67 tests
npm run build     # vite build
cargo check  --manifest-path src-tauri/Cargo.toml
cargo test   --manifest-path src-tauri/Cargo.toml   # 145 tests
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```
All six pass on a clean tree. `cargo fmt --check` does **not**: ~48 pre-existing diff hunks across `commands/mod.rs`, `core/{at,decoder,live,models,modem,reassemble,sim_directory}.rs` and `logging.rs`. No workflow runs `cargo fmt` or `cargo clippy`, and there is no `rustfmt.toml`, `.editorconfig`, prettier or eslint in the repo — formatting is unenforced debt here, not a gate. Do not run `cargo fmt` to "fix" it: a whole-tree reformat buries the real diff of your change. Write new code close to rustfmt output by hand and leave the surrounding hunks alone.

CI gates are narrower than that list: `test.yml` runs `cargo test --locked`, `cargo check --release --locked`, `npm run check`, `npm test` and `npm run build`. `cargo clippy` is still ungated — it passes today, but `dtolnay/rust-toolchain@stable` moves every six weeks, so adding it with `-D warnings` would turn every open PR red on a new lint until the toolchain is pinned. The `--locked` flags are why `Cargo.lock` must never drift from `Cargo.toml`.

Nothing enforces any of it: `main` has no branch protection and no ruleset, so every check is advisory and a red run does not block a merge (`Memory/02 §6`).

## Backend invariants (src-tauri/)
- A port-touching command starts only when `AppStateInner::port_busy()` is false. It covers `scan_busy`, `live_on`, `live_stop.is_some()`, `ussd_busy`, `delete_busy`, `cleanup_busy`, `detect_busy`. `live_stop.is_some()` is the live-shutdown window: `stop_live` clears `live_on` the instant the user asks, but the workers hold their ports until the supervisor joins them, and one parked in `AT+CMGL=4` holds for up to 15 s more. The flag means "the ports are held", not "an operation is nominally running". A new busy flag joins this one method and is cleared on every exit path.
- `lock_state()` recovers from poisoning (`unwrap_or_else(|e| e.into_inner())`) so one panicking worker cannot wedge the app. Use it; never `state.lock().unwrap()`.
- Emit outside the lock: build the payload under the guard, drop it, then `app.emit`. Followed everywhere except one known violation still in the tree — the `LiveEvent::Batch` arm in `start_live` emits `messages:added` while holding the guard. Do not copy that shape; if you touch that arm, hoist the emit out.
- Identity keys: `modem::stable_id()` (the `/dev/serial/by-path` symlink name, falling back to the mutable tty name off Linux) and ICCID via `SimDirectory`. Never persist or match a SIM number by tty name.
- `merge_ports` carries session state over by **stable path**, never by name — but `live_ready` survives a refresh only when three things hold at once: a live session is still running, the port is still enumerated, and the tty name behind that path is unchanged. A live worker is bound for life to the tty name it was spawned with (`live::run_live_inner` reopens that exact name after an outage), so a replugged and renumbered stick has a worker retrying a name that no longer exists; carrying its LIVE badge forward would be a lie.
- Only probe silence means "no modem": `modem::NOT_RESPONDING` is the only reason that sets `alive = Some(false)`, and `utils/port.ts::portStatus` styles the empty slot off `alive === false`. A host-side read/write failure becomes `Serial I/O failed: …` via `probe_failure_reason` and must never clear liveness. No silent loss of messages, events or counters.
- `catch_unwind` around every per-port worker (`live::run_live` plus the detect/scan/USSD/delete/cleanup loops in `commands/mod.rs`); bounded pools `MAX_CONCURRENT_PORTS` (16) and `MAX_CONCURRENT_PROBES` (32).
- Serial: DTR and RTS asserted on open (`raise_modem_lines`) — Windows drives both low otherwise and `AT&K3`/`AT&D2` firmware then withholds every reply. `setup_sms_mode` tries `ATE0;+CMGF=0` for PDU mode (needed for the UDH) and falls back to `enter_text_mode` (`AT+CMGF=1;+CSCS="UCS2"`, retried one command at a time). `get_sim_number` probes first, then `ATE0`, `AT+CSCS="GSM"`, ICCID, `AT+CUSD=2` to cancel a stale session, then `AT+CNUM`, then the `AT+CREG?`/`AT+CSQ` pre-check before any USSD. ICCID chain: `AT+CCID` → `AT+ICCID` → `AT^ICCID`. There is **no** `AT+CMEE=1` anywhere in the code or in README.md — do not add one on the strength of an old note.
- Deletion (`delete_selected`) is confirmed, not assumed: `modem::delete_messages` re-reads the SIM and returns the indices that really went, `confirmed_removals` drops a row only when **every** slot from `message_slots` (all `part_indices`, one per fragment of a concatenated SMS) is confirmed gone, and a row with any surviving slot is **KEPT** and reported as "kept". Counts are SIM slots, not ports. Undated messages never expire. The confirmation is absence-based — a slot missing from `AT+CMGL` counts as deleted — so a slot number that *cannot* be listed reads as a false success. SIM slots start at 1: `message_slots` and `models::expired_indices` drop non-positive indices, and `decoder::parse_cmgr`/`parse_pdu_cmgr` take the slot as a parameter because a `+CMGR` header does not carry one (`Memory/03 §14`).

## Logging and privacy
`logging::init` hardcodes `set_max_level(Info)`; the ring buffer and file logger each gate at `<= Level::Info` and `capture_entry` drops anything above Info. Masking is applied by hand at individual Info call sites (`mask_number`, `otp_summary`) — **not at the sink**.

**Refusal: never add a log capture-level switch.** (`developer.logLevel` was deleted for this.) Below the Info gate sit unmasked `debug!` lines: `AtChannel::send` logs `>> {cmd}` and `<< {preview(&text, 160)}` for every command and reply, and the pump logs `++ {preview(&line, 120)}` for unsolicited lines. For `AT+CMGL`/`AT+CMGR` that preview is raw PDU hex containing the sender MSISDN and the message body including the OTP; for `AT+CUSD` it is the subscriber's own number. Lowering the gate writes all of it into the 1000-entry ring buffer shown verbatim on the Logs page and into `app.log`, which rotates at 5 MB and is never aged out — so it outlives the inbox retention window (default 2 h). If debug output is ever genuinely needed, redact **in the sink**; do not expose the existing debug lines. Detail: `Memory/05 §B.2`.

## OTP detection
`core/decoder.rs::extract_otp` is `normalize_myanmar_digits` (U+1040–U+1049 → ASCII) → `KEYWORD_RE` gate → ordered cascade `P1` (keyword, then 4–8 digits within 24 chars) → `P2` (digits then `is`/`as your`/`KW_IS`) → `P3` (bare 6 digits) → `P4` (bare 4–8 digits). The gate carries the Myanmar keyword constants `KW_KODE`, `KW_CONFIRM`, `KW_SECURE`.

**Refusal: never add an operator-editable OTP regex.** (`otp.otpPattern` was deleted for this.) `P3`/`P4` match bare digits and are only safe because the keyword gate ran first. One user-supplied pattern replaces the whole cascade and discards the gate, so promotional-SMS balances, dates and number fragments start matching. It fails silently — no error, healthy-looking UI, wrong number on the clipboard. A read-only display of the active patterns is acceptable; an input field is not. Detail: `Memory/05 §B.1`.

## The inert-control rule
A Settings switch that does nothing is worse than no switch: it teaches the operator that Settings lies, and that costs most when they are debugging a field failure with the bank in front of them. A new setting is wired to real behaviour in the same change that adds the control, or it is not added. Eleven fields were deleted under this rule — ledger in `Memory/05-feature-roadmap.md §A`, rule itself in `Memory/04 §H`. One inert field survives deliberately (`developer.autoScroll`, `Memory/05 §C.3`); `general.portRefreshInterval` was the other and is now wired via `App.svelte::restartPortRefresh`.

## Frontend rules (src/)
- State in runes stores (`src/lib/stores/*.svelte.ts`). `invoke` and `listen` appear **only** in `src/lib/services/api.ts`; every other Tauri surface (window, dialog, updater, process, app) is reached through a lazy `await import()` behind `isTauri()` from `utils/tauri.ts`. Results are reported via toasts.
- Synthetic browser-preview parity (localhost:1420) is mandatory: every `api.*` call needs a non-Tauri branch, fed by `utils/synthetic.ts`. `services/updater-preview.ts` and the real `services/updater.ts` stay separate.
- CSV export escapes formula injection: `utils/csv.ts::guardFormula` prefixes `'` to any cell starting `= + - @ TAB CR`, checked raw *and* whitespace-trimmed, because RFC-4180 quoting alone does not stop a spreadsheet evaluating the cell. Every cell is attacker-controlled — anyone who can SMS the bank controls the sender and the body. JSON export is deliberately **not** guarded (plain `JSON.stringify`, machine-read, never opened in Excel); do not "fix" it by adding apostrophes there.
- Tests: Node's built-in runner with `--experimental-strip-types` (Node 22), files `src/**/*.test.ts`. No vitest — it pulls a critical-severity advisory chain through this project's `vite@5` pin (`Memory/04 §D`). No new deps without justification. Logic worth testing goes in a rune-free, `$lib`-free module (`utils/csv.ts`, `utils/port-refresh.ts`, `utils/message-buffer.ts`) so the runner can import it directly.
- Inbox rows are buffered before being flushed into the store (`utils/message-buffer.ts`). Update and removal events can arrive for a row still sitting in that buffer, so both paths must look there as well as in the store.

## Theme and styling
- Every colour comes from a CSS variable defined in **both** theme blocks of `src/app.css` (`:root, :root.dark` and `:root.light`), in full — no partial overrides, so switching classes can never leave a stale value behind. Values are space-separated RGB triples so Tailwind's `<alpha-value>` works. There are no hardcoded hex colours anywhere in `src/`; keep it that way.
- The active theme is a class on `<html>` (`dark`/`light`), never `prefers-color-scheme`. The OS preference is read in exactly two places, both only to choose which class when the theme is `system`: the flash guard in `index.html` and `applyTheme` in `stores/settings.svelte.ts`.
- Outside `src/`, the dark background is duplicated as literal hex in `index.html` (`<meta name="theme-color">`) and `src-tauri/tauri.conf.json` (`app.windows[0].backgroundColor`). Both are `#171717` = `--background: 23 23 23`. Changing the dark background means changing all three, or the window flashes the wrong colour before the webview paints.
- **Tailwind trap:** a class naming a token that is not in `tailwind.config.ts` is silently never emitted — no error, no class, no style. `bg-popover` and `hover:bg-accent` are how the Export dropdown shipped with no background and no hover at all. The colour tokens that exist are exactly `border`, `background`, `surface`, `elevated`, `foreground`, `primary`, `success`, `danger`, `warning`, `otp`, `muted` — the last six each also with a `-foreground` variant. `--console-bg`, `--console-fg` and `--console-row-hover` are CSS variables only, with no Tailwind token, and must be used through arbitrary values (`bg-[rgb(var(--console-bg))]`) as `pages/Logs.svelte` does.

## Untrusted input
- The frontend is not a validator. Any numeric setting that reaches Rust is clamped or rejected **in Rust**: `commands::retention_from_hours` is the precedent (rejects non-finite and `<= 0`, returning `None` = "keep everything").
- Why: the Settings page's shared number input applies `min`/`max`/`step` to the DOM element only — its `onchange` stores whatever `parseInt` returns, unclamped — and the store is rehydrated from `localStorage`, which can hold anything an older profile or a hand edit left behind. Out-of-range values are normal, not exceptional.
- On the frontend, a settings number used as a timer delay is clamped at the point of use, because a delay above 2^31-1 ms overflows `setInterval` into a near-zero delay and becomes a tight loop: `portRefreshPeriodMs` clamps to 5–3600 s, `restartAutoUpdater` to 1–168 h. New timers do the same.
- SMS input is hostile in shape as well as content: `decoder::split_udh` bounds-checks the declared UDH length because `i + 1 + udhl` used to run past the buffer and panic the decode thread.

## CI
- `test.yml` — every push on every branch except the release-please branch, plus `pull_request`; `paths-ignore` skips `**.md` and `Memory/**`. `cargo test --locked` on ubuntu-22.04 + windows-latest, `cargo check --release --locked` on Linux only (it is what compiles the `cfg(not(debug_assertions))` file logger), and a Linux frontend job running `npm run check` + `npm test` + `npm run build`. Its `concurrency` group keys on the head SHA with `cancel-in-progress: true`, so pushing a branch and opening its PR produces one **cancelled** run alongside the real one — `gh pr checks` reports a cancelled run as `fail`. Read the annotation before believing it.
- `build-check.yml` — PRs into `main`, plus `workflow_dispatch`. Builds the real installers with `npx tauri build --no-sign` and uploads them for 7 days; `fail-fast: false`, `permissions: contents: read`, no `GITHUB_TOKEN` in any step, no tauri-action. It cannot publish and must stay that way: tauri-action creates a release from `tagName` when no `releaseId` is set, so bolting PR triggers onto the publish workflow instead would put an accidental release one bad `if:` away.
- `release-please.yml` — push to `main`, serialised by a `concurrency` group with `cancel-in-progress: false` (a half-finished release PR is worse than a queued one). Opens/updates the release PR using `secrets.PAT`, then `sync-cargo-lock` checks out the release branch and pushes the `Cargo.lock` version bump release-please cannot make; without it `--locked` fails every release PR (`Memory/03 §8`). Same PAT, so the PR's own checks re-run. That push retries up to 3 times, rebasing onto the branch tip between attempts, because release-please force-pushes the branch and the tip can move after checkout; it never uses `--force` — what release-please wrote there is authoritative (`Memory/02 §6`).
- `tauri-build.yml` — publish only, on `release: published`. `fail-fast: true`, signs with `TAURI_SIGNING_PRIVATE_KEY`, and passes `releaseBody` so `latest.json` carries notes for the in-app updater. Note `fail-fast` does **not** make publishing atomic: both matrix legs upload their own bundles and their own merged `latest.json`, so a Windows failure after the Linux leg finished leaves a published release advertising Linux only, and there is no `workflow_dispatch` to recover with (`Memory/02 §6`).
- Windows + Linux only. No macOS entry in any matrix — `Memory/02 §5`.
- `src-tauri/gen/schemas/*.json` is tracked and regenerated by the Tauri build script during `cargo check`. Never hand-edit; if it changes, commit the regenerated file.
- Action pins: `actions/checkout@v5`, `actions/setup-node@v5` with `node-version: '22'`, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2` (`workspaces: src-tauri`), `actions/upload-artifact@v5`, `googleapis/release-please-action@v4`, `tauri-apps/tauri-action@v0`. Node 22 is required, not preferred (`--experimental-strip-types`). GitHub removes Node 20 from hosted runners on 16 September 2026; nothing here pins Node 20, so there is nothing to do — do not add one.

## Documentation duty
Behaviour changes that touch README-documented facts (probe timeouts, AT flow, status formats, command lists) are synced in README.md in the same change. Real bug fixes get a `Memory/03` entry. For anything needing the bank physically attached, follow `Memory/04 §G` (Hardware Live-Check Playbook) rather than inventing a check.

## Reference
`Memory/01`–`06`, six docs, Burmese: 01 GitHub CLI/auth · 02 release automation (§5 platform policy) · 03 troubleshooting casebook + latent traps · 04 conventions, verification loop, §D command cheatsheet, §G hardware playbook, §H inert-switch rule · 05 roadmap, Settings decisions ledger, the two hard refusals · 06 git/feature-branch workflow. `Memory/README.md` is the index.

When searching, note that `src-tauri/src/.mimosa/hook-state/` holds gitignored snapshot copies of source files. Grep hits there are stale duplicates — ignore them.

