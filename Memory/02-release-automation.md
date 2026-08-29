# 02 — Release Automation Pipeline

> `git push` တစ်ခုတည်းနဲ့ version bump → changelog → tag → GitHub Release → signed installers
> → `latest.json` → in-app updater အထိ အလိုအလျောက်လည်နေတဲ့ system ရဲ့ အပြည့်အစုံ။

## Architecture

```
git push origin main                      (conventional commits)
        │
        ▼
[release-please workflow]                 (push event)
  Conventional commits ကို analyze
  ├─ feat:/fix:/feat!: ရှိရင် ──▶ Release PR auto-open/bump
  │      "chore(main): release X.Y.Z"
  │      diff: package.json + CHANGELOG.md + tauri.conf.json + Cargo.toml (+ Cargo.lock manual cargo check)
  └─ chore:/docs:/ci: ပဲရှိရင်  ▶ PR မဖွင့် (state မပြောင်း)
        │
        ▼  Human: PR title/diff review → Merge (squash recommended)
[autorelease]  ◀── release-please does tagging on merge
  Tag vX.Y.Z + GitHub Release + notes (commit messages ကနေ)
        │
        ▼
[publish workflow]                        (release published event)
  matrix: ubuntu-22.04 + windows-latest
  tauri-action → build → sign (TAURI_SIGNING_PRIVATE_KEY) → upload to Release
  → *.exe / *.msi / AppImage / deb / rpm + *.sig + latest.json (updater manifest)
        │
        ▼
App ထဲ Update Check
  GET https://github.com/RyanWez/sms-deliverer/releases/latest/download/latest.json
  compare version vs runtime → Settings → Updates ရဲ့ release-notes card
  → "Update Now" = download only (signature verify) → "Restart Now" = install + relaunch
```

> **v1.3.0 ကနေစပြီး** download နဲ့ install က သီးသန့် step နှစ်ခု
> (`update.download()` → `update.install()`)။ `downloadAndInstall()` မသုံးတော့ဘူး —
> user က release notes ဖတ်ပြီး restart timing ကို ကိုယ်တိုင် ရွေးနိုင်ရမယ် (live ports
> အလုပ်လုပ်နေချိန် app ကို ကိုယ်တိုင် restart မလုပ်ဖို့)။ Windows မှာ `install()` က
> installer ကို လွှတ်ပြီး process ကို ကိုယ်တိုင် ပိတ်တာမို့ `relaunch()` က Linux path မှာသာ ရောက်တယ်။
> Manual check ကို `MANUAL_COOLDOWN_MS = 60s` cooldown နဲ့ ကန့်သတ်ထားတယ်၊
> background timer က `BACKGROUND_MIN_GAP_MS = 15min` (`src/lib/utils/update-policy.ts`)။

## Config Inventory (ဘယ် file က ဘာလုပ်လဲ)

| File | Role |
|---|---|
| `release-please-config.json` | RP behavior: release-type=node, `include-component-in-tag:false`, extra-files (see below) |
| `.release-please-manifest.json` | Current released version pin — `{ ".": "1.0.1" }` |
| `.github/workflows/release-please.yml` | Push event → googleapis/release-please-action@v4 (`config-file`+`manifest-file` args မမေ့နဲ့) |
| `.github/workflows/tauri-build.yml` | Release published event → tauri-apps/tauri-action@v0 matrix build |
| `src-tauri/tauri.conf.json` | `"createUpdaterArtifacts": true` (signed .sig output!), `plugins.updater.pubkey`, endpoints |

## Critical Config Details (tdouble-check လုပ်စရာများ)

### 1️⃣ Tag naming — `include-component-in-tag: false`
Upstream default က **true** (`base.ts:152 → options.includeComponentInTag ?? true`)။ မထည့်ရင် tag က
`sms-tauri-v1.0.2` ဖြစ်သွားမယ် → baseline `v1.0.1` convention နဲ့ ကွာ → compare links ပျက်၊
RP က ကိုယ့် tag ကို "မတွေ့" လို့ မှားယွင်းတဲ့ PR ဖွင့်နိုင်တယ် (PR #4/#5 ဖြစ်ခဲ့တဲ့ case)။

### 2️⃣ Multi-file version bump — extra-files
```json
"extra-files": [
  { "type": "json", "path": "src-tauri/tauri.conf.json", "jsonpath": "$.version" },
  { "type": "toml", "path": "src-tauri/Cargo.toml",     "jsonpath": "$.package.version" }
]
```
node type က package.json ကို ကိုယ်စား bump ပြီးသား — ဒီ extra-files ၂ ခုက tauri side ချိတ်ပေးတာ။
Release PR diff ထဲမှာ `"version"` lines ၄ ခုလုံး ပြောင်းနေမယ်ဆိုတာ စစ်ထားပါ။

### 3️⃣ Signed updater artifacts chain (မဟုတ်ရင် latest.json upload SKIP ဖြစ်မယ်)
```
createUpdaterArtifacts:true (conf) + TAURI_PRIVATE_KEY + PASSWORD (secrets)
  → bundle output မှာ .sig files ရ
  → tauri-action မှ မှ latest.json generate/upload လုပ်နိုင်
missing piece ရှိရင် CI log မှာ exact line: "Signature not found for the updater JSON. Skipping upload..."
```

### 4️⃣ First/bootstrap release (manual)
Tag မရှိသေးခင် ကိုယ်တိုင် ထုတ်တာက publish workflow ကို trigger လုပ်တဲ့ official လမ်း:
```bash
gh release create v1.0.1 --target main --title "v1.0.1" --notes-file notes.md
```
(release published event → publish run ချက်ချင်း start — RP မလိုဘဲ baseline ချနဲ့)

### 5️⃣ Platform policy — Windows + Linux only (macOS ဘယ်တော့မှမထုတ်)

- Publish matrix (`tauri-build.yml`) ထဲ runner = `ubuntu-22.04` + `windows-latest` ၂ ခုပဲ —
  **matrix ဘယ်လောက်ပဲ ပြင်ပြင် `macos-latest` row မထည့်နဲ့** (2026-08-27 cleanup: ကျန်နေခဲ့တဲ့
  dead macOS cross-compile scaffold `targets: …apple-darwin…` conditional ကို ဖယ်ရှုံးပြီး၊
  တည်နေရာမှာ policy comment ချထား)။
- Evidence (v1.0.1 live-check): assets 11 ခုထဲ `.dmg`/`.app` zero · `latest.json` platforms ထဲ
  darwin key zero → installer chain + updater chain နှစ်ခုစလုံး သန့်။
- Gotcha: `"targets": "all"` (tauri.conf.json) က *current OS* ရဲ့ bundle format အားလုံးကို ဆိုတာ —
  linux→AppImage/deb/rpm, windows→NSIS/MSI။ **OS selection control က matrix တည်း**၊ config မဟုတ်။
- Old GitHub Releases = immutable snapshots — workflow/policy ပြောင်းလဲမှုက future releases
  ပေါ်မှာပဲ apply ဖြစ် (v1.0.1 assets untouched)။

## 🚀 Developer Step-by-Step Release Cheatsheet (အနာဂတ် Release လမ်းညွှန်ချက်)

ကုဒ်များ အသစ်ရေးသား/ပြင်ဆင်ပြီးတိုင်း App Version အသစ် ထုတ်ရန်အတွက် အောက်ပါ အဆင့် ၃ ဆင့်ကို လုပ်ဆောင်ရုံသာ ဖြစ်သည်-

### ၁။ Commit & Push ပြုလုပ်ခြင်း
Conventional Commit ပုံစံဖြင့် commit ရေးသားပြီး `main` သို့ push တင်ပါ-
```bash
git add .
git commit -m "feat: သင်ထည့်သွင်းလိုက်သည့် feature အမည်"   # Minor version bump (ဥပမာ 1.1.0 -> 1.2.0)
# သို့မဟုတ်
git commit -m "fix: သင်ပြင်ဆင်လိုက်သည့် bug အမည်"           # Patch version bump (ဥပမာ 1.1.0 -> 1.1.1)

git push origin main
```

### ၂။ Release-Please PR ကို Merge ပြုလုပ်ခြင်း
Push တင်လိုက်သည်နှင့် GitHub ပေါ်ရှိ `release-please` bot က Version bump, Changelog generation ပြုလုပ်ပြီး `chore(main): release X.Y.Z` PR ကို ဖွင့်ပေးပါမည်။

Terminal မှဖြစ်စေ၊ GitHub Web UI မှဖြစ်စေ Merge လုပ်ပါ-
```bash
gh pr merge --merge
```

### ၃။ Build အခြေအနေ စစ်ဆေးခြင်း & Local Sync ပြုလုပ်ခြင်း
PR Merge ပြီးသည်နှင့် `publish` workflow က Windows (`.exe`, `.msi`) နှင့် Linux (`.AppImage`, `.deb`, `.rpm`) installers များကို build လုပ်ပြီး GitHub Release ပေါ်သို့ auto-upload တင်ပေးပါမည်။

```bash
# Build progress ကြည့်ရန်
gh run list --limit 3

# Release assets ထွက်ရှိမှု ကြည့်ရန်
gh release view

# Tag အသစ်နှင့် Changelog ကို local စက်ထဲသို့ sync လုပ်ယူရန်
git pull origin main --tags
```

---

## Operational Commands

```bash
gh pr list --state open                          # waiting release PR?
gh pr view <N> --json files -q '.files[].path'   # bump coverage စစ်
gh pr close <N> --delete-branch                  # stale release PR ရှင်း
gh run list --limit 5                            # workflow status
gh run view <id> --json jobs                     # job-level status/conclusion
gh run view --job <jobId> --log-failed           # ❗run complete ဖြစ်မှ available
curl -sIL https://github.com/RyanWez/sms-deliverer/releases/latest/download/latest.json | head -1
                                                 # ↑ 200 = updater healthy · 404 = broken
```
