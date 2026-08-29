# 03 — Troubleshooting Casebook (တကယ်ဖြစ်ခဲ့တဲ့ ၁၀ ခု)

> Format: Symptom → Root Cause → Fix → Preventive Rule. Debug ခင် ဒီထဲ အရင်ရှာပါ။

---

## 1️⃣ In-app "Check Now" → loading spin ပြီးရင် ဘာမှ မပေါ်

- **Symptom:** UI button spinner လည်၊ result popup/notification zero
- **Root Cause:** Result rendering က browser `alert()` / `confirm()` — Tauri v2 webview (esp. Linux WebKitGTK) မှာ
  **silently swallow** ဖြစ်တယ် (error မှ မထ)
- **Fix:** `src/lib/services/updater.ts` → toast system (`liveStore.addToast`) + `@tauri-apps/plugin-dialog` native confirm နဲ့ အစားထိုး
- **Rule:** Tauri frontend မှာ browser dialog API သုံးမနေနဲ့ — plugin-dialog/dialog wrapper ပဲ။
- **v1.3.0 update:** native confirm ကိုပါ ဖြုတ်လိုက်ပြီ။ Update ရှိတာကို Settings → Updates ရဲ့
  in-app card (release notes + Update Now + Restart Now) နဲ့ ပြတယ် — dialog ထဲမှာ release notes
  ရှည်ကို မဖတ်နိုင်၊ platform တစ်ခုစီ ပုံစံ လုံးဝ မတူဘူး။ State machine က
  `src/lib/stores/updater.svelte.ts`၊ rate limit + byte/percent format က
  `src/lib/utils/update-policy.ts` (pure, tested)။

## 2️⃣ Plugin call တွေ အလုပ်မလုပ် (updater/process/dialog)

- **Symptom:** invoke fail / denied / silent
- **Root Cause:** Tauri v2 capabilities ACL — plugin register လုပ်ထားတာနဲ့ permission မရှိသေးဘူး
- **Fix:** `src-tauri/capabilities/default.json` → `"updater:default"`, `"process:default"`, `"dialog:default"` ထည့်
- **Rule:** Plugin အသစ် `Cargo.toml` + `lib.rs` ထည့်တိုင်း capabilities permissions ပါ ထည့်ပါ၊ ပြီးရင် rebuild.

## 3️⃣ latest.json 404 + .sig assets မရှိ

- **Symptom:** Release ထဲ installer တွေပဲရှိ၊ `latest.json`/`*.sig` zero → updater endpoint 404
- **Root Cause chain:** sign key/password secret (or) `bundle.createUpdaterArtifacts` flag မရှိ → .sig မထွက် →
  tauri-action log: *"Signature not found for the updater JSON. Skipping upload..."*
- **Fix:** `"createUpdaterArtifacts": true` ထည့် + secrets verify (pubkey ↔ private pair match!)
- **Health probe:** `releases/latest/download/latest.json` → HTTP 200 expected

## 4️⃣ Ubuntu CI build: `rust-lld: error: unable to find library -lxdo`

- **Symptom:** windows job success, ubuntu job `linking with cc failed`
- **Root Cause:** Tauri `linux-libxdo` feature (global-shortcut dependent crate) system lib လိုအပ်၊ runner မှာ default မပါ
- **Fix:** workflow apt step ထဲ `libxdo-dev` ထည့်
- **Rule:** Linux runner အသစ်လိုအပ်ချက် (gtk/webkit/appindicator/rsvg/patchelf/**libudev**/**libxdo**) လို့ known-set သတိထား။

## 5️⃣ Update မမြင်ရ (update ရှိပေမယ့် "already latest")

- **Symptom:** New release publish ပြီးလည်း app က update မတွေ့
- **Root Cause:** Version mismatch — updater က app runtime version vs latest.json payload ကို compare လုပ်တာ။
  History: npm=1.x / tauri.conf=2.0.0 ရောနေချိန် — release-please က npm field ပဲ bump လုပ်တာမို့ tauri version 2.0.0 ပဲကျန်
- **Fix:** extra-files config (doc 02 §2) နဲ့ 4-file အားလုံး auto-bump chain; baseline reset 1.0.1
- **Rule:** Release PR diff မှာ 4 version line လုံး ပြောင်းနေမလား မြင်ရင် merge.

## 6️⃣ Release PR မှားနေတယ် / tag scheme confusion (sms-tauri-v… prefix)

- **Symptom:** RP PR #4/#5 diff ထဲ compare link `sms-tauri-v1.0.1...sms-tauri-v1.1.0`; PR open → close → reopen cycle
- **Root Cause:** upstream default `includeComponentInTag=true`; manual bootstrap tag `v1.0.1` နဲ့ match မဖြစ်
- **Fix:** config ထဲ `"include-component-in-tag": false` → RP က stale PR auto-close + recompute (verified live)
- **Rule:** Tag naming config ကို baseline နဲ့ အရင်ညှိပြီးမှ ပထမ release မထုတ်နဲ့။

## 7️⃣ Ghost publish run "queued" 14+ hours / delete လုပ်ထားတဲ့ tag

- **Symptom:** Run list ထဲ old-tag run pending forever ပုံစံ
- **Diagnose:** `gh run list` + `gh run cancel <id>` (completed ဖြစ်နေရင် cancel error ပေးတယ် — harm none);
  Releases/tags ဘယ်ဟာတွေကျန်လဲ: `gh api repos/<o>/<r>/releases` , `git ls-remote --tags origin`
- **Rule:** Full purge (delete releases → delete tags w/ branch cleanup) လုပ်ပြီး run history stale ဖြစ်နိုင် — panic မလုပ်၊ API/state machine verify ဦး။

## 8️⃣ Release PR tests fail: `cargo test --locked` — Cargo.lock out of sync

- **Symptom:** PR #7 (release 1.2.0) မှာ ubuntu + windows cargo-test jobs နှစ်ခုလုံး fail —
  `error: cannot update the lock file … because --locked was passed`
- **Root Cause:** release-please က `Cargo.toml` ရဲ့ `version` ကို bump ပေးပေမယ့် `Cargo.lock` ထဲက
  `sms-tauri` package version ကို ထိတ်လုပ်မပေးဘူး (doc 02 §2 မှာ extra-files က lock မပါ)။
  CI က `--locked` နဲ့ run လို့ lock mismatch ကို error ပြတယ်။
- **Fix:** release branch checkout → `cargo check` (lock regenerate) →
  `chore: sync Cargo.lock with version X.Y.Z` commit → PR branch push → CI auto re-run → green → merge
- **Rule:** Release PR ဖွင့်တာနဲ့ test jobs က fail ရင် log ကို အရင်ကြည့် —
  `--locked` error ဖြစ်နေရင် lock sync commit နဲ့ fix လုပ်လို့ရတယ်၊ PR ကို close/reopen မလုပ်နဲ့။
  (v1.2.0, 2026-08-28 မှာ verified)

## 9️⃣ Scan / Live / Get SIM အရမ်းကြာ — 64 port ရှိပေမယ့် SIM ၇ ခုပဲ

- **Symptom:** SIM bank မှာ SIM ၇ ခုပဲ စိုက်ထားချိန် Scan ~97s, Get SIM ~3 မိနစ်;
  log ထဲ `modem not responding` ၅၇ ကြိမ်; Live မှာ port ၆၄ လုံးလုံး `Live ready` ထွက်နေတယ်
- **Root Cause:** SIM bank က channel တစ်ခုစီအတွက် tty device ဖန်တီးတယ် — **SIM ရှိမရှိ မသက်ဆိုင်ဘူး**။
  `available_ports()` က device ၆၄ လုံး တွေ့တာ မှားတာ မဟုတ်၊ ဒါပေမဲ့ liveness စစ်တဲ့ အဆင့် မရှိလို့
  `read_port` / `get_sim_number` / live worker သုံးခုလုံး AT sequence အပြည့်ကို တိတ်ဆိတ်တဲ့ port ဆီ ပို့ပြီး
  timeout တွေ အားလုံးကို အစအဆုံး ဆပ်တယ် — Scan 24s, Get SIM 35s, Live 22s per empty slot
  (× `MAX_CONCURRENT_PORTS = 16` အသုတ် ၄ ခု)
- **Secondary bug:** `network_problem(None, None)` က `None` ပြန်တယ် ("ပြဿနာ မရှိ")။
  ဒါကြောင့် `AT+CREG?` ကို ဘာမှ ပြန်မထူးတဲ့ modem က USSD 2×9s ကို ဆက်သွားတယ် —
  အဲ့ timeout ကို ချွေဖို့ ရည်ရွယ်ထည့်ထားတဲ့ pre-check က လိုအပ်ဆုံး port တွေမှာ **လုံးဝ မလုပ်**ဘူး
- **Fix:** `modem::probe_channel()` — `AT` ကို 800ms × ၂ ခါ။ Final result code (OK/ERROR/+CME ERROR)
  တစ်ခုခု ရရင် alive။ ဒီ gate ကို `read_port`, `get_sim_number`, `delete_messages`, live worker
  အားလုံးရဲ့ ရှေ့မှာ ထားတယ် → dead port ၂၄s/၃၅s အစား **~1.6s**။
  `AT+CREG?` က result code မပြန်ရင်လည်း USSD skip။
  Live worker မှာ probe ကျရင် `Ready` အစား `LiveEvent::Offline` (60s တစ်ခါ re-probe)။
  `detect_ports` command + `PortInfo.alive` + Ports page "Detect Modems" ခလုတ် →
  dead port တွေကို auto-uncheck (probe concurrency သီးသန့် `MAX_CONCURRENT_PROBES = 32`)
- **Rule:** Port တစ်ခုကို heavy AT sequence မပို့ခင် **ရှင်မရှင် အရင်စစ်**။
  Device node ရှိတာ = modem ရှိတာ **မဟုတ်ဘူး**။ Timeout constant တွေကို လျှော့တာနဲ့ မဖြေရှင်းနဲ့ —
  gate ထည့်တာက အမြစ်ဖြေရှင်းချက်။ (v1.2.0 log, 2026-08-29 မှာ verified)

## 🔟 SMS ရှည် (concatenated) တွေ စာလုံးပေါက်ကုန်တာ — GSM-7 + UDH

- **Symptom:** Export ထဲ `iAX§OOKIARÑi§AΘsΦÇAB...` ပုံစံ အမှိုက်စာ; OTP `None`;
  log မှာ `live SMS read (idx 3) [concat]` + `(idx 4) [concat]` ဆက်စပ်ပြီးမှ ဖြစ်တာ
- **Root Cause:** `decoder.rs` GSM-7 branch မှာ UDH ကို **နှစ်ခါ ခုန်ကျော်**နေတယ် —
  byte cursor `i += 1 + udhl` နဲ့ header ကျော်ပြီး၊ `decode_gsm7(&bytes, i, septets, skip)` မှာ
  `skip * 7` bit ကို **ထပ်** ကျော်ခိုင်းတယ်။ ရလဒ် = bit alignment ၁ bit လွဲ → စာလုံးအားလုံး ပြောင်း
  (space တွေ `A` ဖြစ်သွားတာ ဒီ shift ရဲ့ လက်မှတ်)
- **ဘာကြောင့် မတွေ့ခဲ့လဲ:** concat test တစ်ခုတည်းသာ ရှိပြီး UCS-2 (Myanmar) ကို စမ်းထားတာ။
  GSM-7 + UDH လမ်းကြောင်း — အင်္ဂလိပ် OTP စာ အများစု လာတဲ့ လမ်းကြောင်း — မှာ coverage **သုည**
- **Fix:** bit cursor ကို UDHL byte ကနေ စတင်ခိုင်း (`ud_start`)၊ `i` ကို UCS-2 recovery probe အတွက်ပဲ ဆက်တွန်း။
  Test ၃ ခု ထည့်: UDHL=5 (fill bit ၁), UDHL=6 (fill bit ၀), UDH မပါတဲ့ GSM-7 regression
- **Rule:** GSM-7 septet count တွေက UDH ရဲ့ **အစ**ကနေ ရေတွက်တယ် (header + fill bits ပါ)၊
  payload byte ကနေ မဟုတ်ဘူး။ Encoding branch အသစ် ထည့်တိုင်း UDH ရှိ/မရှိ **နှစ်မျိုးလုံး** test ရေးပါ။
  (commit `431dcaf`, 2026-08-29)

## Bonus UX Notes

- **User-reported 404 on releases page** but server-side probes said HTTP 200 (public repo) →
  browser cache/transient CDN — hard refresh/incognito/direct tag URL နဲ့ cross-check လုပ်ဘိုး။
- **`gh` CLI:** `export GH_PAGER=cat PAGER=cat` မလုပ်ရင် interactive pager block ဖြစ်နိုင် (non-interactive shell trap);
  long polls ကို background process + `/tmp/*.txt` capture pattern သုံး။
