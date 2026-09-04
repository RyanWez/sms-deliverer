# 02 — Release Automation Pipeline

> The whole system that runs by itself off a single `git push`: version bump → changelog → tag → GitHub Release → signed installers
> → `latest.json` → in-app updater.

## Architecture

```
git push origin main                      (conventional commits)
        │
        ▼
[release-please workflow]                 (push event)
  analyzes the conventional commits
  ├─ if feat:/fix:/feat!: present ──▶ Release PR auto-open/bump
  │      "chore(main): release X.Y.Z"
  │      diff: package.json + CHANGELOG.md + tauri.conf.json + Cargo.toml (+ Cargo.lock manual cargo check)
  └─ if only chore:/docs:/ci:     ▶ no PR opened (no state change)
        │
        ▼  Human: PR title/diff review → Merge (squash recommended)
[autorelease]  ◀── release-please does tagging on merge
  Tag vX.Y.Z + GitHub Release + notes (from the commit messages)
        │
        ▼
[publish workflow]                        (release published event)
  matrix: ubuntu-22.04 + windows-latest
  tauri-action → build → sign (TAURI_SIGNING_PRIVATE_KEY) → upload to Release
  → *.exe / *.msi / AppImage / deb / rpm + *.sig + latest.json (updater manifest)
        │
        ▼
Update Check inside the app
  GET https://github.com/RyanWez/sms-deliverer/releases/latest/download/latest.json
  compare version vs runtime → the release-notes card in Settings → Updates
  → "Update Now" = download only (signature verify) → "Restart Now" = install + relaunch
```

> **From v1.3.0 onward** download and install are two separate steps
> (`update.download()` → `update.install()`). `downloadAndInstall()` is no longer used —
> the user has to read the release notes and then pick the restart timing themselves (so nobody is
> forced to restart the app while live ports are working). On Windows `install()`
> launches the installer and closes the process itself, so `relaunch()` is only ever reached on the Linux path.
> A manual check is held back by a `MANUAL_COOLDOWN_MS = 60s` cooldown,
> and the background timer by `BACKGROUND_MIN_GAP_MS = 15min` (`src/lib/utils/update-policy.ts`).

## Config Inventory (which file does what)

| File | Role |
|---|---|
| `release-please-config.json` | RP behavior: release-type=node, `include-component-in-tag:false`, extra-files (see below) |
| `.release-please-manifest.json` | Current released version pin — `{ ".": "1.0.1" }` |
| `.github/workflows/release-please.yml` | Push event → googleapis/release-please-action@v4 (do not forget the `config-file` + `manifest-file` args) |
| `.github/workflows/tauri-build.yml` | Release published event → tauri-apps/tauri-action@v0 matrix build |
| `src-tauri/tauri.conf.json` | `"createUpdaterArtifacts": true` (signed .sig output!), `plugins.updater.pubkey`, endpoints |

## Critical Config Details (the things worth double-checking)

### 1️⃣ Tag naming — `include-component-in-tag: false`
The upstream default is **true** (`base.ts:152 → options.includeComponentInTag ?? true`). Leave it out and the tag
becomes `sms-tauri-v1.0.2` → it diverges from the baseline `v1.0.1` convention → compare links break,
and RP can open a wrong PR because it does not "see" our own tag (the case that produced PR #4/#5).

### 2️⃣ Multi-file version bump — extra-files
```json
"extra-files": [
  { "type": "json", "path": "src-tauri/tauri.conf.json", "jsonpath": "$.version" },
  { "type": "toml", "path": "src-tauri/Cargo.toml",     "jsonpath": "$.package.version" }
]
```
The node type already bumps package.json on our behalf — these two extra-files are what hook the tauri side in.
Check that all four `"version"` lines are changing in the Release PR diff.

### 3️⃣ Signed updater artifacts chain (without it the latest.json upload is SKIPPED)
```
createUpdaterArtifacts:true (conf) + TAURI_PRIVATE_KEY + PASSWORD (secrets)
  → .sig files appear in the bundle output
  → only then can tauri-action generate/upload latest.json
if a piece is missing, the exact line in the CI log: "Signature not found for the updater JSON. Skipping upload..."
```

### 4️⃣ First/bootstrap release (manual)
Before any tag exists, cutting one by hand is the official way to trigger the publish workflow:
```bash
gh release create v1.0.1 --target main --title "v1.0.1" --notes-file notes.md
```
(release published event → the publish run starts immediately — lay the baseline down without needing RP)

### 5️⃣ Platform policy — Windows + Linux only (macOS is never shipped)

- The publish matrix (`tauri-build.yml`) carries exactly two runners, `ubuntu-22.04` + `windows-latest` —
  **however much the matrix is edited, never add a `macos-latest` row** (2026-08-27 cleanup: the leftover
  dead macOS cross-compile scaffold, the `targets: …apple-darwin…` conditional, was removed and a
  policy comment left in its place).
- Evidence (v1.0.1 live-check): of the 11 assets, `.dmg`/`.app` zero · in the `latest.json` platforms,
  darwin key zero → both the installer chain and the updater chain are clean.
- Gotcha: `"targets": "all"` (tauri.conf.json) means every bundle format of the *current OS* —
  linux→AppImage/deb/rpm, windows→NSIS/MSI. **The OS selection control is the matrix**, not the config.
- Old GitHub Releases = immutable snapshots — a workflow/policy change applies only to future
  releases (v1.0.1 assets untouched).

### 6️⃣ `sync-cargo-lock` race — `concurrency` + rebase-retry (fixed 2026-08-31)

- **The problem before:** `release-please.yml` had no `concurrency` block, and the push in
  `sync-cargo-lock` was a plain `git push origin "HEAD:${VERSION_REF}"` (not force, no retry).
  Push to `main` twice back to back and two runs race: run A checks the release
  branch out at SHA₁ → the release-please in run B force-pushes SHA₂
  on top → run A's push fails **non-fast-forward**. The result is a release PR left
  with an unsynced `Cargo.lock` and a red `--locked` check (case #8 coming back)
- **Two fixes:** (1) workflow-level `concurrency: { group: release-please-${{ github.ref }},
  cancel-in-progress: false }` — **queue instead of cancelling**, because a half-finished
  release PR is worse than a queued one. (2) the push loops over three attempts —
  on a reject it replays onto the branch tip with `git fetch` + `git rebase origin/<branch>`,
  and exits cleanly if release-please has already written the lockfile itself
- **Never use `--force`** — what release-please wrote on that branch is
  authoritative, and wiping it can lose the version bump itself
- **Two recommendations not yet acted on** (2026-08-31 pipeline audit — not code, so not part of
  this PR):
  1. **`main` has no branch protection** (`gh api .../branches/main/protection` → 404,
     rulesets → `[]`). With no required status check, `test.yml`/`build-check.yml` are
     **advisory only** — a red run does not block a merge, and nothing blocks a direct push.
     When adding it, take care: both `test.yml` and `build-check.yml` have `paths-ignore`,
     and a skipped job reports no check, so a **docs-only PR would deadlock**.
     Solve it by deleting `paths-ignore` or by adding a no-op "skip" companion job
  2. **The updater endpoint has a 404 window before the artifacts land.** v1.4.0's metadata:
     release `publishedAt` 11:36:40Z, earliest asset `createdAt` 11:44:13Z — **7m33s**.
     More important, the comment claiming `fail-fast: true` makes "publish all-or-nothing"
     is **not actually true** — the two matrix legs run in parallel and each uploads its own
     `latest.json`, so if the Windows leg fails after Linux has finished, a "latest" release
     advertising Linux only is left behind for good, and there is no `workflow_dispatch` either. The fix:
     make release-please create a **draft** (`draft: true` in
     `release-please-config.json`) and add a job in `tauri-build.yml` that flips it to
     published only once both legs succeed — `/releases/latest` excludes drafts, so for the whole
     window the previous good release stays in place

## 🚀 Developer Step-by-Step Release Cheatsheet (guide for future releases)

Every time code is newly written or amended, cutting a new App Version is only a matter of these three steps:

### 1. Commit & Push
Write the commit in Conventional Commit form and push it to `main`:
```bash
git add .
git commit -m "feat: the name of the feature you added"   # Minor version bump (e.g. 1.1.0 -> 1.2.0)
# or
git commit -m "fix: the name of the bug you fixed"           # Patch version bump (e.g. 1.1.0 -> 1.1.1)

git push origin main
```

### 2. Merge the Release-Please PR
As soon as the push lands, the `release-please` bot on GitHub does the version bump and changelog generation and opens the `chore(main): release X.Y.Z` PR.

Merge it either from the terminal or from the GitHub Web UI:
```bash
gh pr merge --merge
```

### 3. Check the build state & sync locally
As soon as the PR is merged, the `publish` workflow builds the Windows (`.exe`, `.msi`) and Linux (`.AppImage`, `.deb`, `.rpm`) installers and auto-uploads them onto the GitHub Release.

```bash
# to watch build progress
gh run list --limit 3

# to see whether the release assets came out
gh release view

# to sync the new tag and the changelog down to the local machine
git pull origin main --tags
```

---

## Operational Commands

```bash
gh pr list --state open                          # waiting release PR?
gh pr view <N> --json files -q '.files[].path'   # check bump coverage
gh pr close <N> --delete-branch                  # clear a stale release PR
gh run list --limit 5                            # workflow status
gh run view <id> --json jobs                     # job-level status/conclusion
gh run view --job <jobId> --log-failed           # ❗available only once the run is complete
curl -sIL https://github.com/RyanWez/sms-deliverer/releases/latest/download/latest.json | head -1
                                                 # ↑ 200 = updater healthy · 404 = broken
```
