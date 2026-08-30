# 06 — Git & Feature-Branch Workflow

> Goal: `main` ကို release-please အတွက် **clean** ထားပြီး၊ အလုပ်တွေကို feature branch ပေါ်မှာ
> လုပ်၊ PR နဲ့ squash-merge ဝင်စေတဲ့ workflow ရဲ့ အပြည့်အစုံ (commands အထိ)။

## 1️⃣ ဘာကြောင့် feature branch — ဒီ repo မှာ အထူးအရေးကြီးတာ

`release-please.yml` က `on: push: branches: [main]` — ဒါကြောင့် **`main` ပေါ်တင်လိုက်တဲ့
conventional commit တစ်ခုချင်းစီ က pending release PR ထဲ ချက်ချင်း ဝင်သွားတယ်**။

| `main` ပေါ်တိုက်တင်ရင် ဖြစ်တာ | အကျိုးဆက် |
|---|---|
| `feat:` / `fix:` commit တစ်ခု ရောက်တာနဲ့ | release-please က `release-please--branches--main--components--sms-tauri` branch + PR ကို create/update လုပ်တယ် |
| မပြီးသေးတဲ့ (half-done) commit ဖြစ်နေရင် | version bump + CHANGELOG entry ကို လူတွေ merge မလုပ်ခင်ကတည်းက **queue** ထဲထည့်ထားပြီးသား |
| ပြန်ဖျက်ချင်ရင် | `main` history ပေါ်က revert commit တစ်ခုထပ်လိုတယ် (changelog ထဲ noise) |
| `sync-cargo-lock` job | release PR update ဖြစ်တိုင်း ထပ်ပြေးတယ် — မလိုတဲ့ CI ကုန် |

> 💡 တစ်ခွန်းတည်းနဲ့: **`main` = release surface**, feature branch = work surface။
> "အလုပ်ပြီးမှ `main` ကို ရောက်ရမယ်" — release-please က `main` ကို ဖတ်နေတာမို့။

## 2️⃣ Branch naming convention

Commit log ထဲရှိပြီးသား type prefix တွေကို branch ပေါ်လည်း တူတူသုံး —
`<type>/<short-kebab-topic>`, scope ရှိရင် `<type>/<scope>-<topic>`:

| Branch | ဘယ်လို commit အတွက် |
|---|---|
| `fix/serial-write-vs-silent-modem` | `fix:` / `fix(sim):` / `fix(delete):` |
| `feat/updates-review-notes` | `feat:` / `feat(ports):` / `feat(ui):` |
| `ci/frontend-typecheck` | `ci:` |
| `docs/git-workflow-memory` | `docs:` |
| `chore/deps-bump`, `refactor/...`, `perf/...`, `style/...` | `chore:` `refactor:` `perf:` `style:` |

Rules: lowercase, `-` separator (`_`/space မသုံး)၊ တစ် branch = တစ် concern၊
`release-please--*` prefix ကို **လူက ဘယ်တော့မှ မသုံး** (§6 ကြည့်)။

## 3️⃣ Lifecycle — command အစအဆုံး

```bash
# 0) main ကို အရင် sync (branch က နောက်ဆုံး main ကနေ စရမယ်)
git switch main && git pull --ff-only

# 1) branch create
git switch -c fix/serial-write-vs-silent-modem

# 2) logical chunk အလိုက် stage (§7)
git add src-tauri/src/core/at.rs src-tauri/src/core/modem.rs
git diff --cached --stat            # ဘာတွေ ပါသွားလဲ အမြဲစစ်
git commit -m "fix: tell a failed serial write apart from a silent modem"

# 3) ပထမ push — upstream တွဲ
git push -u origin fix/serial-write-vs-silent-modem
# နောက်ပိုင်း push တွေ
git push

# 4) PR — title က conventional commit ဖြစ်ရမယ် (§5)
gh pr create --base main \
  --title "fix: tell a failed serial write apart from a silent modem" \
  --body "Symptom / root cause / fix ... Refs Memory/03-troubleshooting.md"

# 5) CI ကြည့် (test.yml က PR မှာမှ ပြေးတယ် — §4)
gh pr checks --watch
gh run list --limit 5

# 6) green ဖြစ်မှ squash merge + branch remote ဖျက်
gh pr merge --squash --delete-branch

# 7) local ပြန်ရှင်း
git switch main && git pull --ff-only
git branch -d fix/serial-write-vs-silent-modem     # remote ဖျက်ပြီးရင် -d နဲ့ ရ
git fetch --prune
```

## 4️⃣ CI matrix — feature branch vs `main` vs release PR

| Workflow | Trigger (တကယ်ရေးထားတာ) | Feature branch push | PR → `main` | `main` push | Release published |
|---|---|---|---|---|---|
| `test.yml` (`cargo-test` ubuntu+windows, `frontend` check/test) | `push: branches:[main]` + `pull_request` + `workflow_dispatch` | ❌ **မပြေးဘူး** | ✅ | ✅ | — |
| `release-please.yml` (+ `sync-cargo-lock`) | `push: branches:[main]` | ❌ | ❌ | ✅ | — |
| `tauri-build.yml` (`publish`) | `release: types:[published]` | ❌ | ❌ | ❌ | ✅ |

အရေးကြီးတဲ့ အချက်များ:

- **Feature branch ကို ဘဲ push လုပ်ထားရင် CI ဘာမှ မပြေးဘူး** — `test.yml` က `push` ကို `main` ပဲ
  listen လုပ်တယ်။ CI လိုချင်ရင် **PR ဖွင့်ရမယ်** (ဒါမှမဟုတ် `gh workflow run test.yml --ref <branch>`
  — `workflow_dispatch` ရှိတယ်)။
- `paths-ignore: ['**.md', 'Memory/**', 'docs/**']` — doc-only PR ဆို `test.yml` skip ဖြစ်တယ်
  (checks ဗလာ မြင်ရတာ normal, failure မဟုတ်)။
- `concurrency: test-${{ github.ref }} / cancel-in-progress` — branch တစ်ခုပေါ် ထပ်ပြေးရင်
  အရင် run က cancel ဖြစ်မယ် ("cancelled" = fail မဟုတ်)။
- Release PR ရဲ့ checks က `test.yml` (`pull_request`) ချည်းပဲ — installer တွေက tag/release
  published အခါမှ `tauri-build.yml` နဲ့ ထွက်တယ်။ **PR အဆင့်မှာ bundle test မရဘူး**;
  local `npm run build` + `cargo check --release --locked` က အနီးစပ်ဆုံး proxy။

## 5️⃣ Squash merge × release-please

Squash merge မှာ **PR title က `main` ပေါ်က commit subject ဖြစ်သွားတယ်** (`(#9)` suffix ပါ,
e.g. `751e4fa chore(main): release 1.3.1 (#9)`)။ ဒါကြောင့် —

> ⚠️ PR title က valid conventional commit မဟုတ်ရင် release-please က ဖတ်လို့မရ →
> **CHANGELOG ထဲ ပျောက်ကုန်မယ်**၊ version bump လည်း မဖြစ်ဘူး။ Branch ထဲက commit တွေ
> လှလှရေးထားပေမယ့် အလကား — squash ဖြစ်တာနဲ့ subject က PR title က ယူတယ်။

| Type (log ထဲ တကယ်သုံးထားတာ) | Bump | Changelog |
|---|---|---|
| `fix:` `fix(sim):` `fix(ci):` `fix(delete):` | patch | ✅ Bug Fixes |
| `feat:` `feat(ports):` `feat(ui):` `feat(updates):` | minor | ✅ Features |
| `feat!:` / `fix!:` / `BREAKING CHANGE:` footer | major | ✅ (breaking section) |
| `perf:` `refactor:` `style:` | ❌ bump မဖြစ် | (hidden by default) |
| `docs:` `ci:` `build:` `chore:` | ❌ bump မဖြစ် | ❌ |
| `chore(main): release X.Y.Z` | (bot's own commit) | — |

Merge မလုပ်ခင် PR title ကို `gh pr edit <n> --title "..."` နဲ့ ပြန်ပြင်လို့ရတယ် — merge ပြီးမှ
ပြင်ရင် `main` history rewrite လိုတယ် (မလုပ်နဲ့)။

## 6️⃣ `release-please--branches--main--components--sms-tauri` — machine-owned branch

- release-please action က create/update လုပ်တဲ့ **bot branch** (naming က
  `release-please--branches--<base>--components--<component>` pattern; component name က
  `package.json` ရဲ့ `sms-tauri`)။ ဒီ branch ရဲ့ PR = "Release PR"။
- Content: `.release-please-manifest.json`, `package.json`, `src-tauri/tauri.conf.json`,
  `src-tauri/Cargo.toml` (config ရဲ့ `extra-files`) + `CHANGELOG.md` — ဒါတွေ **လူ မထိရ**။
- `main` ကို push တိုင်း bot က ဒီ branch ကို **force-update** လုပ်တယ် — မင်း hand-commit
  တင်ထားရင် **သွေးဆေးပြီး ပျောက်နိုင်တယ်**။ ဒါကြောင့် local မှာလည်း track မလုပ်တာ ပိုကောင်း။
- တစ်ခုပဲ ခြင်းချက်: **Cargo.lock sync case** — `Memory/03-troubleshooting.md` case **#8**
  (`chore: sync Cargo.lock with version X.Y.Z`, commit `2d32e01`)။
  ဒါကို ယခု `release-please.yml` ရဲ့ `sync-cargo-lock` job က PAT နဲ့ auto လုပ်ပေးပြီ —
  bot က မလုပ်ပေးမှ / job fail ဖြစ်မှသာ လူက manual ဝင်လုပ်ရမယ်။

```bash
# Release PR ကို "ဖတ်" ဖို့ (checkout မလိုဘဲ)
gh pr list --state open --limit 5
gh pr diff <n>
git log --oneline origin/release-please--branches--main--components--sms-tauri -5
```

## 7️⃣ တစ် working tree → logical commit များ ခွဲနည်း

```bash
git status --short --branch
git add src-tauri/src/core/at.rs src-tauri/src/core/modem.rs   # concern A ချည်းပဲ
git diff --cached --stat        # staged ဘာလဲ
git diff --stat                 # ကျန်နေတာ ဘာလဲ
git commit -m "fix: ..."
git add .github/workflows/test.yml
git commit -m "ci: ..."
git add Memory/06-git-workflow.md
git commit -m "docs: ..."
```

ဒီ repo မှာ တစ် concern = တစ် commit ဖြစ်ရမယ့် အကြောင်း:

1. release-please က **commit type အလိုက်** changelog section ခွဲတယ် — `feat:` + `docs:` +
   `ci:` ကို commit တစ်ခုထဲထည့်ရင် release notes က မှားမယ် (သို့) noise ဖြစ်မယ်။
2. `docs:`/`Memory/**` only commit က `test.yml` `paths-ignore` နဲ့ skip ဖြစ်တယ် — code နဲ့
   ရောထားရင် အလကား CI matrix နှစ်ခုလုံး ပြေးတယ်။
3. Field bug တွေ (doc 03) ကို `git log --oneline` နဲ့ ရှာလို့ရတာ commit တွေ atomic ဖြစ်လို့။
4. Revert လိုအပ်ရင် concern တစ်ခုပဲ ပြန်ဖျက်လို့ရတယ်။

## 8️⃣ ❌ Don't do this

| မလုပ်ရ | ဘာကြောင့် |
|---|---|
| မပြီးသေးတဲ့ အလုပ်ကို `main` ကို တိုက်ရိုက် commit | release-please က ချက်ချင်း version bump PR ထဲ ဆွဲသွင်းတယ် (§1) |
| `git push --force` / `--force-with-lease` on `main` (နှင့် အခြား shared branch) | published history rewrite → tag/release/changelog mismatch |
| `release-please--*` branch ကို hand-commit / rebase | bot က force-update လုပ်တာမို့ ပျောက်မယ် — case #8 ခြင်းချက်တစ်ခုပဲ (§6) |
| `Cargo.lock` version bump ကို ကိုယ်တိုင် ရေး/ဖျက် | lock version က `cargo` ပဲ ရေးရမယ်; `sync-cargo-lock` job ရှိပြီးသား။ dependency ပြောင်းရင် `cargo update -p <crate>` သုံး |
| `.zcode/`, `target/`, `dist/`, `node_modules/`, `*.log`, `src-tauri/updater.key*` commit | build artifact / tooling / **signing key** |
| `git add .` ကို အကန့်အသတ်မရှိ | `.zcode/` က **`.gitignore` ထဲ မပါဘူး** — ယခု untracked only ဖြစ်နေတယ်၊ `git add .` ဆို ဝင်သွားမယ် |
| PR title ကို free-form ရေး | squash subject ဖြစ်သွားပြီး changelog ပျောက်မယ် (§5) |
| CI green မဖြစ်ခင် merge | `test.yml` က release profile compile + 49 rust tests + svelte-check ကို ဖမ်းတဲ့ တစ်ခုတည်း gate |

> ⚠️ `.gitignore` state (verified): `node_modules dist target src-tauri/target .svelte-kit
> *.log .env .env.* .DS_Store Thumbs.db .mimosa/ /src-tauri/updater.key /src-tauri/updater.key.pub`
> — **`.zcode/` မပါ**။ commit မဝင်စေချင်ရင် path ရွေး `git add` လုပ်၊ ဒါမှမဟုတ်
> `.gitignore` ထဲ `.zcode/` ထည့်တဲ့ `chore:` commit တစ်ခု လုပ်ပါ။

## 9️⃣ Cheat-sheet

| လုပ်ချင်တာ | Command |
|---|---|
| main sync | `git switch main && git pull --ff-only` |
| branch create | `git switch -c feat/<topic>` |
| staged ကို စစ် | `git diff --cached --stat` |
| commit | `git commit -m "fix(scope): subject"` |
| ပထမ push | `git push -u origin <branch>` |
| PR ဖွင့် | `gh pr create --base main --title "<conventional>" --body "..."` |
| CI ကြည့် | `gh pr checks --watch` · `gh run list --limit 5` |
| CI ကို branch ပေါ် manual ပြေး | `gh workflow run test.yml --ref <branch>` |
| PR title ပြင် | `gh pr edit <n> --title "fix: ..."` |
| squash merge + remote branch ဖျက် | `gh pr merge --squash --delete-branch` |
| local branch ဖျက် + prune | `git branch -d <branch> && git fetch --prune` |
| release PR ကြည့် | `gh pr list --state open` · `gh pr diff <n>` |
| convention history ဖတ် | `git log --oneline -40` |
