# 🧠 Memory — Developer Knowledge Base

> **Project:** SIM Bank SMS Reader (`sms-tauri`) · Repo: `RyanWez/sms-deliverer`
> **Created:** 2026-08-27 · Cline AI coding session မှ စုဆောင်းထားတဲ့ အတွေ့အကြုံများ
> **Purpose:** GitHub / gh CLI / Release Automation / Tauri Updater နဲ့ ပတ်သက်ပြီး ထူထောင်ခဲ့တဲ့
> system တွေ၊ ကြုံခဲ့ရတဲ့ ပြဿနာတွေရဲ့ root cause + fix တွေကို နောက် developer အမြန်နားလည်နိုင်အောင် မှတ်တမ်းတင်ထားခြင်း။
> **လက်ရှိ version:** `package.json` / `CHANGELOG.md` က authoritative (2026-09-04 မှာ **v1.6.1**) —
> ဒီ doc တွေထဲ ရေးထားတဲ့ version နံပါတ်ကို အဲဒီ ၂ ခုနဲ့ တိုက်စစ်ပါ။

## 📁 File Index

| File | Contents |
|---|---|
| [01-github-setup.md](./01-github-setup.md) | gh CLI install → login → scopes → git push credentials |
| [02-release-automation.md](./02-release-automation.md) | Release pipeline architecture + configs (release-please / tauri-action / updater) |
| [03-troubleshooting.md](./03-troubleshooting.md) | တကယ်ကြုံခဲ့ရတဲ့ bug ၂၂ ခု — symptom → root cause → fix (§18–§20 Telegram forwarding၊ §21–§22 OTP false positive ၂ ခု) · + latent trap ၅ ခု (T1/T2 settings layer — **အသက်ဝင်နေတုန်း** · T3 decoder keyword typo၊ T4 retention clamp၊ T5 busy flag — ပြင်ပြီး) |
| [04-conventions.md](./04-conventions.md) | Commit စံနှုန်း၊ verification loop၊ security rules၊ command cheatsheet၊ hardware live-check၊ **§H Settings control wiring rule** |
| [05-feature-roadmap.md](./05-feature-roadmap.md) | Feature backlog + **Settings Controls Decisions Ledger** (ဖျက်ခဲ့တာ ၁၁ ခု၊ hard refusal ၂ ခု၊ deferred order၊ desktop-notification feasibility) |
| [06-git-workflow.md](./06-git-workflow.md) | Feature branch workflow — branch → commit → push → PR → squash merge, CI trigger matrix |
| [07-next-release-plan.md](./07-next-release-plan.md) | v1.5.0 field test ရဲ့ အဖြေ → နောက် release ၂ ခု (v1.6.2 fix ၄ ခု၊ v1.7.0 live command mailbox) — **ဘယ်တစ်ခုမှ မ ship သေးဘူး**၊ v1.6.0/v1.6.1 က Telegram forwarding နဲ့ hotline OTP guard သွားလို့ နံပါတ် ရွေ့ခဲ့တာ |
| [08-telegram-stage2-plan.md](./08-telegram-stage2-plan.md) | **Telegram forwarding — Stage 1 + Stage 2 implementation record (v1.6.0 မှာ ship ပြီး)** · ဆုံးဖြတ်ချက် အကြောင်းရင်း ၄ ခု (hook point၊ 20/min limit၊ thread model၊ config lifetime) · **မလုပ်ရတာ ၉ ခု (အသက်ဝင်နေတုန်း)** · hardware test ၄ ခု **အတည်ပြီး** · §G field test မှာ တွေ့တဲ့ OTP false positive ၂ ခု (ပြင်ပြီး) |

## ⚡ TL;DR — ဒီ Project ရဲ့ Golden Rules

1. **Conventional Commits အမြဲသုံး** — `feat:` → minor · `fix:` → patch · `chore:`/`docs:` → release မဖြစ်
2. **Version = 4-place sync** — `package.json` == `src-tauri/tauri.conf.json` == `Cargo.toml` == `.release-please-manifest.json` (manual edit မလုပ်ဘဲ release-please PR merge နဲ့ bump)။ ပဉ္စမနေရာ `src-tauri/Cargo.lock` ကို release-please မရေးနိုင်လို့ `sync-cargo-lock` job က release branch ပေါ် push တယ် (02 §6 · 03 §8) — ငါးခုလုံး လက်နဲ့ မထိရ
3. **Tags က plain `vX.Y.Z`** — config ထဲ `"include-component-in-tag": false` ကြောင့် (default က prefix ထည့်မယ်!)
4. **Secrets ဘယ်တော့မှ chat/code ထဲမထည့်** — `gh auth login` + keyring က စီစဉ်ပေးတယ်
5. **Edit → Validate → Commit → Push → Verify CI** — ဒီ order ကို ဘယ်တော့မှ ခုံးမထားနဲ့
6. **Inert UI control ကို ဘယ်တော့မှ မ ship လုပ်နဲ့** — setting အသစ်ကို control ထည့်တဲ့ change
   တစ်ခုတည်းအတွင်း wire ပြီးရမယ်၊ မဟုတ်ရင် လုံးဝ မထည့်ရ။ ဘာမှ မလုပ်တဲ့ switch က operator ကို
   "Settings က လိမ်တယ်" လို့ သွန်သင်လိုက်တာ — field failure debug လုပ်နေချိန်မှာ အဲ့ဒါက
   အဆိုးဆုံး (04 §H · ledger 05 §Settings Decisions Ledger)
