# 🧠 Memory — Developer Knowledge Base

> **Project:** SIM Bank SMS Reader (`sms-tauri`) · Repo: `RyanWez/sms-deliverer`
> **Created:** 2026-08-27 · experience accumulated over Cline AI coding sessions · **written in English** (the whole knowledge base was translated out of Burmese in this change)
> **Purpose:** to record the systems built around GitHub / gh CLI / Release Automation / Tauri Updater,
> together with the root cause + fix of every problem actually hit, so that the next developer comes up to speed fast.
> **Current version:** `package.json` / `CHANGELOG.md` are authoritative (**v1.8.0** as of 2026-09-05, with the v1.8.1 tray hardening merged and awaiting its release PR) —
> cross-check any version number written in these docs against those two.
> Everything written as **v1.8.0** has **shipped** — `feat: add system tray icon and minimize on close (#35)`
> plus `fix(tray): persist tray menu state and guard left-click menu suppression for windows (#37)`,
> released by `chore(main): release 1.8.0 (#36)` and tagged `v1.8.0` (`03 §27`).
> **v1.8.1** is the follow-up that closed the three tray traps the audit found (`03 T6`–`T8`).
> **v1.7.0** shipped the new app icon, the in-app Changelog page, the title-bar trim and the
> one-hour retention pin; **v1.6.2** shipped the four status/log-accuracy fixes (`03 §23`–`§26`).

## 📁 File Index

| File | Contents |
|---|---|
| [01-github-setup.md](./01-github-setup.md) | gh CLI install → login → scopes → git push credentials |
| [02-release-automation.md](./02-release-automation.md) | Release pipeline architecture + configs (release-please / tauri-action / updater) |
| [03-troubleshooting.md](./03-troubleshooting.md) | 27 bugs actually hit — symptom → root cause → fix (§18–§20 Telegram forwarding, §21–§22 the two OTP false positives, §23–§26 the four status/log accuracy cases — shipped in v1.6.2: §23 cleanup counter, §24 live ready count + the three buckets, §25 `msg(s)`/`slot(s)` units, §26 USSD form marker, **§27 the blank Linux tray menu — shipped fixed in v1.8.0**) · + 8 latent traps (T1/T2 settings layer — **still live** · T3 decoder keyword typo, T4 retention clamp, T5 busy flag — fixed · **T6–T8 the v1.8.0 tray consequences — all three fixed in v1.8.1**: T6 tray *Quit* now waits for the forwarder flush, T7 `tauri-plugin-single-instance`, T8 the `tray-icon` feature declared rather than inherited) |
| [04-conventions.md](./04-conventions.md) | Commit standards, verification loop, security rules, command cheatsheet, hardware live-check, **§H Settings control wiring rule** |
| [05-feature-roadmap.md](./05-feature-roadmap.md) | Feature backlog + **Settings Controls Decisions Ledger** (11 fields deleted, 2 hard refusals, deferred order, desktop-notification feasibility) · §C.6 (L3) is **closed in v1.6.2** — the limitations that remain are C.4/C.5/C.7 · `general.minimizeToTray` is the one deleted field that came **back**: re-added and wired to the real tray in v1.8.0, so the ledger row records both halves |
| [06-git-workflow.md](./06-git-workflow.md) | Feature branch workflow — branch → commit → push → PR → squash merge, CI trigger matrix |
| [07-next-release-plan.md](./07-next-release-plan.md) | What the v1.5.0 field test answered → the four v1.6.2 fixes, **all shipped and released as v1.6.2** (cases `03 §23`–`§26`) · **§B, the live worker command mailbox, is DECIDED AGAINST — see §B.3 before proposing it again**: it would have put `AT+CMGD` and a 15 s confirming `AT+CMGL` inside the loop that catches every OTP, so v1.7.0 went to the app icon, the in-app Changelog page and two UI trims instead, and v1.8.0 went to the system tray (`03 §27`, traps `T6`–`T8`). The §B.1/§B.2 diagnosis of *why* Delete is disabled during live stays on the record. The numbers moved because v1.6.0/v1.6.1 went to Telegram forwarding and the hotline OTP guard |
| [08-telegram-stage2-plan.md](./08-telegram-stage2-plan.md) | **Telegram forwarding — Stage 1 + Stage 2 implementation record (shipped in v1.6.0)** · the reasoning behind four decisions (hook point, 20/min limit, thread model, config lifetime) · **nine standing refusals (still in force)** · four hardware tests **confirmed** · the two OTP false positives found in the §G field test (fixed) |

## ⚡ TL;DR — This Project's Golden Rules

1. **Always use Conventional Commits** — `feat:` → minor · `fix:` → patch · `chore:`/`docs:` → no release
2. **Version = 4-place sync** — `package.json` == `src-tauri/tauri.conf.json` == `Cargo.toml` == `.release-please-manifest.json` (bumped by merging the release-please PR, never by a manual edit). The fifth place, `src-tauri/Cargo.lock`, is one release-please cannot write, so the `sync-cargo-lock` job pushes it onto the release branch (02 §6 · 03 §8) — all five are off-limits to hands
3. **Tags are plain `vX.Y.Z`** — thanks to `"include-component-in-tag": false` in the config (the default would add a prefix!)
4. **Never put secrets into chat or code** — `gh auth login` plus the keyring arrange that for you
5. **Edit → Validate → Commit → Push → Verify CI** — never skip a step in this order
6. **Never ship an inert UI control** — a new setting has to be wired to real behaviour inside the
   single change that adds the control, or it does not go in at all. A switch that does nothing
   teaches the operator that "Settings lies" — and that costs most while they are debugging a
   field failure (04 §H · ledger 05 §Settings Decisions Ledger)
