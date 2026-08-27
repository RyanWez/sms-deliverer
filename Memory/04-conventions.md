# 04 — Conventions & Working Standards

## A. Commit Message Standard (Conventional Commits — enforced by pipeline)

| Pattern | Effect | Example |
|---|---|---|
| `fix:` | patch bump (v1.0.1 → v1.0.2) | `fix(updater): surface results via toasts` |
| `feat:` | minor bump (v1.0.1 → v1.1.0) | `feat: sidebar animations` |
| `feat!:` / `fix!:` | major bump | `feat!: new data model` |
| `chore:` `docs:` `ci:` `build:` | **no release PR** | `chore(release): …`, `docs: add memory notes` |
| Body/footer | release notes ထဲ auto copy ဖြစ် | commit subject ကို human-readable ရေးပါ |

Subject သည် lowercase imperative + scope recommend; multi-change commit ဆို footers (`BREAKING CHANGE:`) နဲ့။

## B. The Verification Loop (မင်းဘယ်တော့မှ ခုန်မပြါးနဲ့)

```
Edit(s) → local validation (JSON valid? svelte-check? cargo check?)
        → focused review diff (git show)
        → commit (conventional)
        → push → CI watch (gh run list)
        → merged effect verify (release assets / endpoint / running app)
```

Case study: CI failure တစ်ခု debug လုပ်တုန်း — job databaseId → `--log-failed` → root cause `-lxdo` →
minimal workflow patch → re-run green. Root-cause-fix ပဲ၊ symptom patch မဟုတ်။

## C. State Consistency Checklist (release မထုတ်ခင်)

- [ ] `package.json` == `tauri.conf.json` == `Cargo.toml` == manifest JSON
- [ ] Local clean (`git status`), remote synced (`main...origin/main`)
- [ ] Open release-please PR ရှိ/မရှိ နားလည်ထား
- [ ] Tag list (`git ls-remote --tags origin`) နဲ့ မျှောမှန်း convention match
- [ ] Secrets present: `PAT`, `TAURI_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

## D. Command Cheatsheet

```bash
# Git/GitHub daily
git status --short --branch
gh pr list --state all --limit 5
gh run list --limit 5 && gh run view <id> --json jobs

# Validation
python3 -m json.tool <file>                       # JSON syntax
npx svelte-check --tsconfig ./tsconfig.json       # frontend types
npm run build                                     # vite production build
cargo check --manifest-path src-tauri/Cargo.toml  # rust + lockfile sync

# Release ops
gh release view v1.0.1 --json assets -q '[.assets[].name] | sort | .[]'
gh release create vX.Y.Z --target main --notes-file f.md   # bootstrap trigger
curl -sIL .../releases/latest/download/latest.json -o /dev/null -w '%{http_code}\n'
```

## E. Agent/CLI Environment Tips (non-interactive shells)

1. Interactive commands (password prompts, pagers) **never assume** — use flags/env (`GH_PAGER=cat`, `--no-pager`)
2. Long-running (>~30s): background + redirect to `/tmp/name.txt` → poll/read later; timestamps log ထည့်
3. Network/API truth အရင်စစ် (curl/api probes) — assumption နဲ့ fix မလုပ်ပါနဲ့ (transient 404 case က သာဓက)
4. `sleep N && cat /tmp/result.log` pattern နဲ့ async task harvest; exit-code မေ့မနေနဲ့

## F. When Things Look Broken — Order of Trust

```
server-side probe (curl/gh api) > local reproduce > CI logs > user screenshot/memory
```
Cache/CDN illusions ကို အရင်ပယ် — ပြီးမှ configuration hunt စ။ ဒီ approach နဲ့
404-releases-page scare ကို ၅ မိနစ်အတွင်း resolve လုပ်နိုင်ခဲ့တယ်။
