# 🧠 Memory — Developer Knowledge Base

> **Project:** SIM Bank SMS Reader (`sms-tauri`) · Repo: `RyanWez/sms-deliverer`
> **Created:** 2026-08-27 · Cline AI coding session မှ စုဆောင်းထားတဲ့ အတွေ့အကြုံများ
> **Purpose:** GitHub / gh CLI / Release Automation / Tauri Updater နဲ့ ပတ်သက်ပြီး ထူထောင်ခဲ့တဲ့
> system တွေ၊ ကြုံခဲ့ရတဲ့ ပြဿနာတွေရဲ့ root cause + fix တွေကို နောက် developer အမြန်နားလည်နိုင်အောင် မှတ်တမ်းတင်ထားခြင်း။

## 📁 File Index

| File | Contents |
|---|---|
| [01-github-setup.md](./01-github-setup.md) | gh CLI install → login → scopes → git push credentials |
| [02-release-automation.md](./02-release-automation.md) | Release pipeline architecture + configs (release-please / tauri-action / updater) |
| [03-troubleshooting.md](./03-troubleshooting.md) | တကယ်ကြုံခဲ့ရတဲ့ bug ၇ ခု — symptom → root cause → fix |
| [04-conventions.md](./04-conventions.md) | Commit စံနှုန်း၊ verification loop၊ security rules၊ command cheatsheet |
| [05-feature-roadmap.md](./05-feature-roadmap.md) | အနာဂတ်တွင် ဆက်လက် အကောင်အထည်ဖော်မည့် Features & Roadmap စာရင်း |
| [06-git-workflow.md](./06-git-workflow.md) | Feature branch workflow — branch → commit → push → PR → squash merge, CI trigger matrix |

## ⚡ TL;DR — ဒီ Project ရဲ့ Golden Rules

1. **Conventional Commits အမြဲသုံး** — `feat:` → minor · `fix:` → patch · `chore:`/`docs:` → release မဖြစ်
2. **Version = 4-place sync** — `package.json` == `src-tauri/tauri.conf.json` == `Cargo.toml` == `.release-please-manifest.json` (manual edit မလုပ်ဘဲ release-please PR merge နဲ့ bump)
3. **Tags က plain `vX.Y.Z`** — config ထဲ `"include-component-in-tag": false` ကြောင့် (default က prefix ထည့်မယ်!)
4. **Secrets ဘယ်တော့မှ chat/code ထဲမထည့်** — `gh auth login` + keyring က စီစဉ်ပေးတယ်
5. **Edit → Validate → Commit → Push → Verify CI** — ဒီ order ကို ဘယ်တော့မှ ခုံးမထားနဲ့
