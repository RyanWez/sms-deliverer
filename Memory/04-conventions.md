# 04 — Conventions & Working Standards

## A. Commit Message Standard (Conventional Commits — enforced by pipeline)

| Pattern | Effect | Example |
|---|---|---|
| `fix:` | patch bump (v1.0.1 → v1.0.2) | `fix(updater): surface results via toasts` |
| `feat:` | minor bump (v1.0.1 → v1.1.0) | `feat: sidebar animations` |
| `feat!:` / `fix!:` | major bump | `feat!: new data model` |
| `chore:` `docs:` `ci:` `build:` | **no release PR** | `chore(release): …`, `docs: add memory notes` |
| Body/footer | copied automatically into the release notes | write the commit subject so a human can read it |

The subject is lowercase imperative and a scope is recommended; a commit carrying several changes declares them in footers (`BREAKING CHANGE:`).

## B. The Verification Loop (never skip a step)

```
Edit(s) → local validation (JSON valid? svelte-check? cargo check?)
        → focused review diff (git show)
        → commit (conventional)
        → push → CI watch (gh run list)
        → merged effect verify (release assets / endpoint / running app)
```

Case study: while debugging one CI failure — job databaseId → `--log-failed` → root cause `-lxdo` →
minimal workflow patch → re-run green. A root-cause fix, not a symptom patch.

## C. State Consistency Checklist (before cutting a release)

- [ ] `package.json` == `tauri.conf.json` == `Cargo.toml` == manifest JSON
- [ ] Local clean (`git status`), remote synced (`main...origin/main`)
- [ ] Know whether an open release-please PR exists
- [ ] The tag list (`git ls-remote --tags origin`) matches the expected convention
- [ ] Secrets present: `PAT`, `TAURI_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

## D. Command Cheatsheet

```bash
# Git/GitHub daily
git status --short --branch
gh pr list --state all --limit 5
gh run list --limit 5 && gh run view <id> --json jobs

# Validation
python3 -m json.tool <file>                       # JSON syntax
npm run check                                     # frontend types (svelte-check)
npm test                                          # frontend unit tests (node:test, no framework)
npm run build                                     # vite production build
cargo check --manifest-path src-tauri/Cargo.toml  # rust + lockfile sync
cargo test --manifest-path src-tauri/Cargo.toml   # rust unit tests

# Release ops
gh release view v1.0.1 --json assets -q '[.assets[].name] | sort | .[]'
gh release create vX.Y.Z --target main --notes-file f.md   # bootstrap trigger
curl -sIL .../releases/latest/download/latest.json -o /dev/null -w '%{http_code}\n'
```

> Frontend tests deliberately use Node's built-in runner + `--experimental-strip-types`
> (Node 22+) instead of vitest: vitest pulls a critical-severity advisory chain
> through this project's `vite@5` pin, and the tested units are pure functions.

## E. Agent/CLI Environment Tips (non-interactive shells)

1. Interactive commands (password prompts, pagers) **never assume** — use flags/env (`GH_PAGER=cat`, `--no-pager`)
2. Long-running (>~30s): background + redirect to `/tmp/name.txt` → poll/read later; log timestamps
3. Check network/API truth first (curl/api probes) — never fix on an assumption (the transient 404 case is the precedent)
4. Harvest async tasks with the `sleep N && cat /tmp/result.log` pattern; do not forget the exit code

## F. When Things Look Broken — Order of Trust

```
server-side probe (curl/gh api) > local reproduce > CI logs > user screenshot/memory
```
Rule out cache/CDN illusions first — only then start the configuration hunt. This approach is
what resolved the 404-releases-page scare inside five minutes.


## G. Hardware Live-Check Playbook (on the machine with the SIM bank attached)

The layer-by-layer method validated today (2026-08-27, 64-port SIM bank):

1. **Hardware layer** — is the device there? are permissions OK?
   ```bash
   ls /dev/ttyUSB* /dev/serial/by-id    # a zsh glob no-match aborts the whole command — mind setopt nullglob
   groups | grep dialout                # membership is required for rw (crw-rw---- root dialout)
   # AT handshake probe (parallel, read-only ATI → OK):
   stty -F $p 115200 raw -echo -icanon min 0 time 20; exec 3<>$p; printf 'ATI\r' >&3
   ```
   Result day-X: **64/64 RESPOND, zero silent/fail** → USB hub + modems healthy.
2. **Backend logic** — `cargo test --manifest-path src-tauri/Cargo.toml` → PDU decode/OTP/reassemble/AT-channel unit tests (49 passed **on that day**; the current count lives in `AGENTS.md`'s validation block — 226 as of v1.8.0. Mock transport harness `at.rs::with_transport` via cfg(test)).
3. **Frontend build** — `npm run build` + `npx svelte-check` (the vite dynamic-import chunk warnings are harmless noise).
4. **Live boot** — `setsid bash -c 'exec npm run tauri dev' >/tmp/app_dev.log 2>&1 &` (debug build logs → stderr w/ timestamps),
   look for `SIM Bank SMS Reader starting...` + zero panic; bonus: `tauri_plugin_updater` lines = updater E2E free-proof.
5. **Cleanup discipline** — dev instance kill group: `kill -TERM -$PGID`; **multiple instances check first**:
   `ps -o pid,ppid,lstart,args -C sms-tauri` (the user's own running session ≠ your test one — tell them apart by PPID/lstart and kill only your own!)
   **Since v1.8.0 "no window on screen" no longer means "not running".** Closing the window hides
   the app to the tray with its ports still held, so `ps` is the only honest answer to "is a copy
   of this running?" — check it before starting a probe *and* before assuming a port is free.
6. **Tray layer (v1.8.0, needs a desktop shell — not a bank)** — the one class of bug in this app
   that no gate in the repo can catch, because it lives in the shell's AppIndicator/DBus path
   (`03 §27`). Walk it by eye:
   - the icon appears in the tray at all — on GNOME this needs the AppIndicator shell extension,
     see README's Linux requirements note. **No icon + close-to-tray on = a hidden app you can
     only kill from a terminal**
   - the menu opens with **readable labels** (`Open SMS Reader`, `Quit`) — a blank rectangle is
     `03 §27` regressing, and it regresses silently because the icon still draws
   - left-click restores the window on Windows; on Linux the menu item does it
   - `✕` hides rather than exits with the setting on, and with the setting **off** while Live is
     running it still hides (the deliberate `|| s.live_on` override — `AGENTS.md`, tray section)
   - *Quit* really ends the process (`ps` again), and know what it costs: `03 T6`

> ⚠️ Gotcha: at startup every port in the list is auto-checked (`checked:true`). **Press "Detect Modems" first** —
> the `AT` probe (800ms × 2) leaves only the ports that really have a modem checked and auto-unchecks the rest.
> Pressing "Scan & Read All" without detecting spawns a thread for every checked port — but since v1.3+ has the
> probe gate, one dead port now costs only ~1.6s (it used to be 24s). One more caution while you are here:
> a device node existing is not the same as a modem existing (see doc 03 §9).

## H. Settings Control Rule — Never Add an Inert Switch

> **Rule:** a new setting is **wired inside the very change that adds the control,
> or it is not added at all.**

A switch that does nothing is **worse** than no switch at all — because what it teaches the operator
is that "Settings lies". That lesson bites back exactly while a field failure is being debugged —
the moment trust is needed most: seeing the switch flipped on and off with no change in behaviour,
the operator goes hunting through modem/SIM/network, while the real answer is
"nobody reads this switch at all".

A control touches at least three places — do not commit if the third is missing:

| # | Place | What happens without it |
|---|---|---|
| 1 | `SettingsState` + `DEFAULT_SETTINGS` (`src/lib/types.ts`) | no persistence / undefined |
| 2 | Settings page field descriptor (`src/lib/pages/Settings.svelte`) | never appears in the UI |
| 3 | **the consumer that actually reads it** (store getter → component / `api.ts` → Rust command) | **inert switch — this is what breaks the rule** |

Example (the good shape): `notifications.enabled` was wired in `f88d6d0` through the
`notifyOtp()` helper in `src/lib/services/api.ts` — it gates the two OTP announcements only and
never touches the generic `toast` (silencing an operational error is not what
"Enable Notifications: off" means).

On 2026-08-30, eleven fields were deleted under this rule (`fbd7b8b`) and two were designated
**hard refusals** — the ledger and every reason behind it live in **doc 05 §Settings Decisions Ledger**.
The two remaining traps, no type check on the binding path and a deleted field lingering in
`localStorage`, are in **doc 03 §Latent Traps**.
