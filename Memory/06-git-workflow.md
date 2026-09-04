# 06 — Git & Feature-Branch Workflow

> Goal: the whole workflow (down to the commands) that keeps `main` **clean** for release-please,
> does the work on a feature branch, and lets it in through a PR with a squash-merge.

## 1️⃣ Why a feature branch — why it matters especially in this repo

`release-please.yml` is `on: push: branches: [main]` — which means **every single
conventional commit pushed onto `main` goes straight into the pending release PR**.

| What happens if you commit straight onto `main` | Consequence |
|---|---|
| the moment one `feat:` / `fix:` commit arrives | release-please creates/updates the `release-please--branches--main--components--sms-tauri` branch + PR |
| if that commit is half-done | the version bump + CHANGELOG entry are already **queued** before anyone has merged anything |
| if you want to take it back | it takes one more revert commit on `main` history (noise in the changelog) |
| the `sync-cargo-lock` job | reruns on every release PR update — CI burned for nothing |

> 💡 In one sentence: **`main` = release surface**, feature branch = work surface.
> "Only finished work reaches `main`" — because release-please is reading `main`.

## 2️⃣ Branch naming convention

Use the same type prefixes that already exist in the commit log on the branch too —
`<type>/<short-kebab-topic>`, or `<type>/<scope>-<topic>` when there is a scope:

| Branch | For which kind of commit |
|---|---|
| `fix/serial-write-vs-silent-modem` | `fix:` / `fix(sim):` / `fix(delete):` |
| `feat/updates-review-notes` | `feat:` / `feat(ports):` / `feat(ui):` |
| `ci/frontend-typecheck` | `ci:` |
| `docs/git-workflow-memory` | `docs:` |
| `chore/deps-bump`, `refactor/...`, `perf/...`, `style/...` | `chore:` `refactor:` `perf:` `style:` |

Rules: lowercase, `-` separator (no `_`/space), one branch = one concern,
and the `release-please--*` prefix is **never used by a human** (see §6).

## 3️⃣ Lifecycle — the commands from start to finish

```bash
# 0) sync main first (the branch has to start from the latest main)
git switch main && git pull --ff-only

# 1) branch create
git switch -c fix/serial-write-vs-silent-modem

# 2) stage by logical chunk (§7)
git add src-tauri/src/core/at.rs src-tauri/src/core/modem.rs
git diff --cached --stat            # always check what went in
git commit -m "fix: tell a failed serial write apart from a silent modem"

# 3) first push — set the upstream
git push -u origin fix/serial-write-vs-silent-modem
# later pushes
git push

# 4) PR — the title has to be a conventional commit (§5)
gh pr create --base main \
  --title "fix: tell a failed serial write apart from a silent modem" \
  --body "Symptom / root cause / fix ... Refs Memory/03-troubleshooting.md"

# 5) watch CI (test.yml only runs on the PR — §4)
gh pr checks --watch
gh run list --limit 5

# 6) squash merge + delete the remote branch, only once green
gh pr merge --squash --delete-branch

# 7) clean up locally
git switch main && git pull --ff-only
git branch -d fix/serial-write-vs-silent-modem     # -d works once the remote one is deleted
git fetch --prune
```

## 4️⃣ CI matrix — feature branch vs `main` vs release PR

| Workflow | Trigger (as actually written) | Feature branch push | PR → `main` | `main` push | Release published |
|---|---|---|---|---|---|
| `test.yml` (`cargo-test` ubuntu+windows, `frontend` check/test) | `push: branches:[main]` + `pull_request` + `workflow_dispatch` | ❌ **does not run** | ✅ | ✅ | — |
| `release-please.yml` (+ `sync-cargo-lock`) | `push: branches:[main]` | ❌ | ❌ | ✅ | — |
| `tauri-build.yml` (`publish`) | `release: types:[published]` | ❌ | ❌ | ❌ | ✅ |

The points that matter:

- **Push a feature branch on its own and no CI runs at all** — `test.yml` listens for `push` on
  `main` only. If you want CI you **have to open a PR** (or `gh workflow run test.yml --ref <branch>`
  — `workflow_dispatch` exists).
- `paths-ignore: ['**.md', 'Memory/**', 'docs/**']` — on a doc-only PR `test.yml` is skipped
  (seeing the checks empty is normal, not a failure).
- `concurrency: test-${{ github.ref }} / cancel-in-progress` — push again on the same branch and
  the earlier run is cancelled ("cancelled" is not a fail).
- A release PR's checks are `test.yml` (`pull_request`) and nothing else — the installers only come
  out with `tauri-build.yml` at tag/release published time. **There is no bundle test at PR stage**;
  local `npm run build` + `cargo check --release --locked` are the closest proxy.

## 5️⃣ Squash merge × release-please

On a squash merge **the PR title becomes the commit subject on `main`** (with a `(#9)` suffix,
e.g. `751e4fa chore(main): release 1.3.1 (#9)`). Therefore —

> ⚠️ If the PR title is not a valid conventional commit, release-please cannot read it →
> **it goes missing from the CHANGELOG**, and there is no version bump either. However nicely
> the commits inside the branch were written, it is wasted — on a squash the subject is taken from the PR title.

| Type (the ones actually used in the log) | Bump | Changelog |
|---|---|---|
| `fix:` `fix(sim):` `fix(ci):` `fix(delete):` | patch | ✅ Bug Fixes |
| `feat:` `feat(ports):` `feat(ui):` `feat(updates):` | minor | ✅ Features |
| `feat!:` / `fix!:` / `BREAKING CHANGE:` footer | major | ✅ (breaking section) |
| `perf:` `refactor:` `style:` | ❌ no bump | (hidden by default) |
| `docs:` `ci:` `build:` `chore:` | ❌ no bump | ❌ |
| `chore(main): release X.Y.Z` | (bot's own commit) | — |

Before merging, the PR title can still be fixed with `gh pr edit <n> --title "..."` — fixing it after
the merge needs a `main` history rewrite (do not).

## 6️⃣ `release-please--branches--main--components--sms-tauri` — machine-owned branch

- The **bot branch** the release-please action creates/updates (the naming follows the
  `release-please--branches--<base>--components--<component>` pattern; the component name is
  `sms-tauri` from `package.json`). The PR off this branch = the "Release PR".
- Content: `.release-please-manifest.json`, `package.json`, `src-tauri/tauri.conf.json`,
  `src-tauri/Cargo.toml` (the config's `extra-files`) + `CHANGELOG.md` — **no human touches these**.
- On every push to `main` the bot **force-updates** this branch — if you have put a hand-commit
  on it, it **can be washed away and lost**. Which is also why it is better not to track it locally.
- One exception only: **the Cargo.lock sync case** — `Memory/03-troubleshooting.md` case **#8**
  (`chore: sync Cargo.lock with version X.Y.Z`, commit `2d32e01`).
  That is now done automatically with the PAT by the `sync-cargo-lock` job in `release-please.yml` —
  a human only steps in by hand when the bot does not do it / when the job fails.

```bash
# to "read" the Release PR (without checking it out)
gh pr list --state open --limit 5
gh pr diff <n>
git log --oneline origin/release-please--branches--main--components--sms-tauri -5
```

## 7️⃣ How to split one working tree → logical commits

```bash
git status --short --branch
git add src-tauri/src/core/at.rs src-tauri/src/core/modem.rs   # concern A only
git diff --cached --stat        # what is staged
git diff --stat                 # what is left over
git commit -m "fix: ..."
git add .github/workflows/test.yml
git commit -m "ci: ..."
git add Memory/06-git-workflow.md
git commit -m "docs: ..."
```

Why one concern = one commit has to hold in this repo:

1. release-please splits the changelog sections **by commit type** — put `feat:` + `docs:` +
   `ci:` into a single commit and the release notes come out wrong (or noisy).
2. A `docs:`/`Memory/**` only commit is skipped by `test.yml`'s `paths-ignore` — mix it with code
   and both CI matrix legs run for nothing.
3. Field bugs (doc 03) can be found with `git log --oneline` precisely because the commits are atomic.
4. If a revert is needed, only the one concern has to be taken back.

## 8️⃣ ❌ Don't do this

| Do not | Why |
|---|---|
| commit unfinished work straight onto `main` | release-please pulls it into a version bump PR immediately (§1) |
| `git push --force` / `--force-with-lease` on `main` (and any other shared branch) | published history rewrite → tag/release/changelog mismatch |
| hand-commit / rebase the `release-please--*` branch | the bot force-updates it, so it will be lost — case #8 is the one exception (§6) |
| write/delete the `Cargo.lock` version bump yourself | the lock version is `cargo`'s to write; the `sync-cargo-lock` job already exists. When changing a dependency use `cargo update -p <crate>` |
| commit `.zcode/`, `target/`, `dist/`, `node_modules/`, `*.log`, `src-tauri/updater.key*` | build artifact / tooling / **signing key** |
| `git add .` without limits | `.zcode/` is **not in `.gitignore`** — right now it is only untracked, so a `git add .` takes it in |
| write the PR title free-form | it becomes the squash subject and the changelog entry goes missing (§5) |
| merge before CI is green | `test.yml` is the only gate that catches release-profile compile + 49 rust tests + svelte-check |

> ⚠️ `.gitignore` state (verified): `node_modules dist target src-tauri/target .svelte-kit
> *.log .env .env.* .DS_Store Thumbs.db .mimosa/ /src-tauri/updater.key /src-tauri/updater.key.pub`
> — **`.zcode/` is not there**. If you do not want it committed, `git add` by chosen path, or
> make one `chore:` commit that adds `.zcode/` to `.gitignore`.

## 9️⃣ Cheat-sheet

| What you want to do | Command |
|---|---|
| main sync | `git switch main && git pull --ff-only` |
| branch create | `git switch -c feat/<topic>` |
| check what is staged | `git diff --cached --stat` |
| commit | `git commit -m "fix(scope): subject"` |
| first push | `git push -u origin <branch>` |
| open a PR | `gh pr create --base main --title "<conventional>" --body "..."` |
| watch CI | `gh pr checks --watch` · `gh run list --limit 5` |
| run CI manually on a branch | `gh workflow run test.yml --ref <branch>` |
| fix a PR title | `gh pr edit <n> --title "fix: ..."` |
| squash merge + delete the remote branch | `gh pr merge --squash --delete-branch` |
| delete the local branch + prune | `git branch -d <branch> && git fetch --prune` |
| look at the release PR | `gh pr list --state open` · `gh pr diff <n>` |
| read the convention history | `git log --oneline -40` |
