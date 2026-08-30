# 03 — Troubleshooting Casebook (တကယ်ဖြစ်ခဲ့တဲ့ ၁၃ ခု)

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

## 8️⃣ Release PR tests fail: `cargo test --locked` — Cargo.lock out of sync (ယခု auto-fix)

- **Symptom:** PR #7 (release 1.2.0) မှာ ubuntu + windows cargo-test jobs နှစ်ခုလုံး fail —
  `error: cannot update the lock file … because --locked was passed`
- **Root Cause:** release-please က `Cargo.toml` ရဲ့ `version` ကို bump ပေးပေမယ့် `Cargo.lock` ထဲက
  `sms-tauri` package version ကို ထိတ်လုပ်မပေးဘူး (doc 02 §2 မှာ extra-files က lock မပါ)။
  lockfile version ကို cargo ကိုယ်တိုင်ပဲ ရေးနိုင်တယ်၊ CI က `--locked` နဲ့ run လို့ mismatch ကို error ပြတယ်။
- **အရင် လက်နဲ့ လုပ်ခဲ့တာ (context အတွက် ကျန်ထား):** release branch checkout → `cargo check`
  (lock regenerate) → `chore: sync Cargo.lock with version X.Y.Z` commit → PR branch push →
  CI auto re-run → green → merge (commits `2d32e01` / `49a489a`)
- **ယခု အလိုအလျောက် ဖြစ်တာ:** `.github/workflows/release-please.yml` ထဲ `sync-cargo-lock` job
  ထည့်ထားပြီ။ release-please ရဲ့ `prs_created` / `pr` output ကို ကြည့်ပြီး
  `fromJSON(...).headBranchName` branch ကို `secrets.PAT` နဲ့ checkout →
  `cargo update --workspace --manifest-path src-tauri/Cargo.toml` →
  `cargo metadata --locked` နဲ့ verify → `git diff --quiet -- src-tauri/Cargo.lock` က
  ပြောင်းလဲမှု ရှိတဲ့အခါမှသာ `chore: sync Cargo.lock with version X.Y.Z` commit + push
  (loop guard — release-please က main push တိုင်း re-run လုပ်တာမို့)။ PAT နဲ့ push လုပ်တာက
  PR ရဲ့ checks ကို ပြန် trigger ဖြစ်စေတယ် (default `GITHUB_TOKEN` ဆိုရင် မဖြစ်ဘူး)။
- **Rule:** Release PR မှာ `--locked` error ထပ်တွေ့ရင် lockfile ကို **လက်နဲ့ အရင် မဖြေနဲ့** —
  (a) `sync-cargo-lock` job run ဖြစ်ခဲ့လား (release-please job က `prs_created=true` ထွက်ခဲ့လား)၊
  (b) `secrets.PAT` သက်တမ်း/scope ကျန်နေလား — ဒီနှစ်ခုကို အရင်စစ်။ PR ကို close/reopen မလုပ်နဲ့။
- **Unverified (2026-08-30):** ဒီ automation ကို ဒီ branch
  (`fix/reliability-and-privacy-hardening`) မှာ ထည့်ထားတာ ဖြစ်ပြီး တကယ့် release PR တစ်ခုမှ
  ဖြတ်သွားတာ **မရှိသေးဘူး**။ ဒါကြောင့် အထက်က manual procedure က fallback အဖြစ် ကျန်တယ်။
  (Original manual fix က v1.2.0, 2026-08-28 မှာ verified)

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

## 1️⃣1️⃣ Get SIM နှိပ်လိုက်ရင် `Found: 1/32` — registered modem တွေက USSD ကို ငြင်းပယ်တာ

- **Symptom:** 32 port ထဲ 1 ခုပဲ number ရ။ log မှာ port 22 ခုက `*88#` နဲ့ `*124#` နှစ်ခုလုံးကို
  `+CME ERROR: 100`၊ 7 ခုက `no reply within 9s`၊ 2 ခုပဲ `reg stat 2` (network မရ)
- **Root Cause (တစ်ခုမဟုတ် သုံးခု):**
  1. `+CME ERROR: 100` က **13 ms အတွင်း** ပြန်လာတယ် (14:27:36.811 → .824) — network timeout မဟုတ်ဘူး၊
     modem က command ကိုတင် ငြင်းတာ။ ရှေးက run တစ်ခုက USSD session ပိတ်မကျန်ခဲ့ရင် firmware က
     session ရှိနေတယ်လို့ ထင်ပြီး `AT+CUSD=1` အသစ်ကို ချက်ချင်း ငြင်းတယ်
  2. firmware အချို့က `,15` (dcs argument) ကို လက်မခံဘူး
  3. USSD code ကို `*88#`/`*124#` နှစ်ခုပဲ hardcode ထားတယ် — carrier လိုက် ကွဲတယ်
- **အရေးကြီးတာ:** failure 31 ခုထဲ 29 ခုက `reg stat 1` (home network, signal 10–22/31) ပြနေတယ် —
  ဒါကြောင့် "SIM လိုင်း မကောင်းလို့" က port 2 ခုအတွက်ပဲ မှန်တယ်။ ကျန်တာက modem/USSD ပြဿနာ
- **Fix:** `get_sim_number` order ကို ပြောင်း — (a) `AT+CUSD=2` နဲ့ stale session အရင်ရှင်း၊
  (b) `AT+CNUM` (EF_MSISDN — SIM ပေါ်က file၊ network မလို၊ ms အတွင်း ပြန်) ကို **အရင်** စမ်း၊
  (c) USSD ငြင်းခံရရင် တူတဲ့ code ကို dcs မပါဘဲ တစ်ခါ retry။ code list ကို
  `OWN_NUMBER_USSD_CODES` const အဖြစ် တစ်နေရာတည်းမှာ စုထား (Mytel: `*88#` → `*124#`)
- **Rule:** SIM ကနေ တိုက်ရိုက် ရနိုင်တာကို network ကို မတောင်းနဲ့။ `+CME ERROR` က ms အတွင်း ပြန်လာရင်
  network ပြဿနာ မဟုတ်ဘူး — modem state ကို ကြည့်။ `Found: x/y` က data ဆုံးရှုံးမှု မဟုတ်ဘူး၊
  cache ထဲ အရင်ရခဲ့တဲ့ number တွေ အတိအကျ ကျန်နေတယ် (`ussd_one_port` ရဲ့ `else` branch က log ပဲ ရေးတယ်)။
  ဒါပေမဲ့ **slot တစ်ခုထဲမှာ SIM လဲထည့်လိုက်ရင်** အရင် number က stable path key ပေါ်မှာ ကျန်နေမယ် —
  ဒါက cache ရဲ့ တစ်ခုတည်းသော အန္တရာယ်

## 1️⃣2️⃣ Port တစ်ခုက SIM နံပါတ် နှစ်ခု၊ With SIM 34 ဆိုပေမဲ့ modem 32 ပဲ ရှိတာ

- **Symptom:** ttyUSB39 (READY) နဲ့ ttyUSB43 (NO MODEM) နှစ်ခုလုံး `09651995803` ပြ။
  ttyUSB27/28 က `09652187632` တူ၊ ttyUSB30/31 က `09675797146` တူ (နှစ်ခုလုံး READY)။
  Badge မှာ `With SIM 34` ဒါပေမဲ့ `32 modems` / `Selected 32`
- **Root Cause:** `stable_id()` က **တစ်ခါမှ match မဖြစ်ခဲ့ဘူး**။ `serialport` က
  `/dev/ttyUSB7` ပြန်ပေးတယ်၊ `/dev/serial/by-path/...` symlink က `../../ttyUSB7` ကို ချိတ်တယ် —
  code မှာ `target.file_name() == name` လို့ တိုက်ကြည့်တာ ဆိုတော့ `"ttyUSB7" == "/dev/ttyUSB7"` →
  **အမြဲ false**။ ဒါကြောင့် `p.path` က port name ပဲ ဖြစ်နေတယ်၊ cache key က မတည်ငြိမ်တဲ့ tty name။
  ဒီအပေါ် `number_of(stable, legacy)` က legacy **name** key ကို fallback လုပ်တာ ထပ်ဆင့် ဆိုးတယ် —
  hotplug ပြီး နံပါတ် ရွှေ့သွားရင် အရင် SIM ရဲ့ number က အခု အဲ့ name ရတဲ့ တခြား stick ပေါ် ပေါ်လာတယ်။
  CSV ထဲ `/dev/ttyUSB64..67` row တွေ ရှိတာ (ttyUSB48..51 နဲ့ number တူ) က renumbering ဖြစ်ခဲ့တဲ့ သက်သေ
- **Fix (architecture ပြောင်း):** number ကို port ပေါ် မဖိုင်ဘူး၊ **ICCID** ပေါ် ဖိုင်တယ်။
  `probe_port` က `AT+CCID`/`AT+ICCID`/`AT^ICCID` နဲ့ card ကို ခွဲသိတယ်။ CSV format v2 =
  `sim,<iccid>,<number>` (durable) + `slot,<stable_path>,<iccid>` (hint ပဲ)။
  Modem မတွေ့တဲ့ slot က hint ပြုတ်တယ် (number က ICCID အောက်မှာ ကျန်နေတယ်)၊
  SIM ကို slot တခြားကို ရွှေ့ရင် number လိုက်လာတယ်။ v1 (port-keyed) file က migrate မလုပ်ဘူး —
  `sim_numbers.csv.v1-port-keyed` အဖြစ် ဘေးဖယ်ထားပြီး စွန့်ပစ်တယ် (key တွေ ယုံလို့ မရတော့ဘူး)။
  Frontend `hasValidSim()` က `alive === false` port ကို With SIM အဖြစ် မရေတွက်တော့ဘူး
- **Rule:** "stable id" လို့ နာမည်ပေးထားရုံနဲ့ stable မဖြစ်ဘူး — resolve ဖြစ်/မဖြစ် **test ရေးပါ**
  (`stable_id_resolves_a_by_path_symlink` က tmpdir symlink နဲ့ စစ်တယ်)။ Cache key ရွေးတဲ့အခါ
  **hardware ကိုယ်တိုင် ပြောပြတဲ့ identity** ကို ရွေး (ICCID)၊ OS က ပေးတဲ့ နာမည် မဟုတ်။
  Legacy fallback key ဆိုတာ silent data corruption ရဲ့ လမ်း — migrate မရရင် ဖယ်ပစ်တာ ပိုကောင်း။

## 1️⃣3️⃣ Windows မှာ modem 32/64 ပဲ တွေ့တာ — bank က boot မပြီးသေးတာ (misdiagnosis case)

- **Symptom:** Windows session (18:00–18:08) မှာ `Modem not responding (no reply to AT)` 32 ခု၊
  `Found: 31/64` → `34/64`။ Linux session (18:47–18:52) မှာ `63/64` + number 62။
  Windows log မှာ **`Cannot open` တစ်ခုမှ မရှိ** — port အားလုံး ပွင့်ပြီး တစ်ဝက်က `AT` ကို မပြန်။
  မပြန်တဲ့ set က ၈ မိနစ်လုံး ထပ်တူ (COM3–18, COM26–29, COM39–50)
- **ကျွန်တော် ပထမ လွဲမှားခဲ့တဲ့ conclusion:** "ထပ်တူ set + ၁၀၀% open success" ကို ကြည့်ပြီး
  static config fault (DTR/RTS low) လို့ ဖြေခဲ့တယ်။ `serialport` crate ရဲ့ platform asymmetry က
  တကယ် ရှိတယ် (အောက်တွင်) ဆိုတော့ ပိုပြီး ယုံလောက်စရာ ဖြစ်နေတယ်
- **တကယ့် Root Cause:** bank က **power-up/enumeration မပြီးသေးတာ**။ သက်သေ ၄ ချက်:
  1. **တူတူ Windows machine၊ တူတူ version၊ ၅၀ မိနစ် အကြာ session (18:56–19:24) မှာ
     `Modems OK: 62/64`, `Found: 57/64`, `Live ready 63`** — code မပြောင်းဘဲ အလိုလို ကောင်းသွားတာ
  2. 18:00 session ထဲမှာ COM10 က 18:03၊ COM18 က 18:04 မှာ **အလိုလို ပြန်လာတယ်** (staggered boot)
  3. Linux log မှာ node တွေ တဆင့်ချင်း ပေါ်တာ တိုက်ရိုက် တွေ့တယ်: 52 port → 32 node missing → 64
  4. DTR/RTS low ဆိုရင် ငါတို့ DCB က session တိုင်း တူတူ ဆိုတော့ **deterministic ပျက်ရမယ်** —
     မပျက်ဘူး။ ဒါက theory ကို ဖြတ်ပစ်တဲ့ အချက်
- **Windows က Linux ထက် ဆိုးပုံရတဲ့ အကြောင်း:** Linux မှာ module မပြင်ဆင်ရသေးရင် `/dev/ttyUSB*`
  node ကို မရှိ → `Cannot open` လို့ ရိုးသားစွာ ပြောတယ်။ Windows မှာ `COM` port က registry ထဲ
  အမြဲ ရှိတာကြောင့် ပွင့်တယ်၊ ဘာမှ မပြန်ဘူး → "modem မရှိ" လို့ မှားထင်စေတယ်
- **Fix (hardening ပဲ၊ diagnosed fault အတွက် မဟုတ်):** `open_port` မှာ `.flow_control(None)`
  explicit + `raise_modem_lines()` (DTR/RTS assert; Linux မှာ no-op)။ `at.rs::send` က write error
  ကို channel dead မှတ်ပြီး `Serial I/O failed: …` လို့ ပြောတယ် — `NOT_RESPONDING` နဲ့ ခွဲထားလို့
  scan/USSD/live သုံးခုလုံး alive=false မမှတ်တော့ဘူး (`windows/com.rs` က port timeout ကို
  `WriteTotalTimeoutConstant` အဖြစ်လည်း ထည့်လို့ TX stall မှာ AT မထွက်ဘဲ "modem မပြန်ဘူး" လို့ လိမ်တာ)။
  **`AT&K0` nudge ကို ထည့်ခဲ့ပြီး ပြန်ဖျက်တယ်** — မှားတဲ့ theory အပေါ် တည်ထားတာ၊ ပြီးတော့ boot
  မပြီးသေးတဲ့ module ပေါ် flow control ကို ပိတ်ပစ်တာက PDU dump ကြီးတွေမှာ FIFO overrun ဖြစ်နိုင်တယ်
- **Rule ၁:** **ကိုယ်တိုင် ပြန်ကောင်းသွားတဲ့ fault ဟာ static config fault မဟုတ်ဘူး။**
  Root cause ဖြေတဲ့အခါ "ဒီ theory မှန်ရင် ဘယ်လို ပျက်ရမလဲ" ကို အရင် စစ်ပါ — deterministic
  ဖြစ်ရမယ့် theory က intermittent symptom ကို မဖြေဆိုနိုင်ဘူး
- **Rule ၂:** Log ဖိုင် အသစ်ဆုံးကို **အရင်** ရှာပါ။ ကျွန်တော် 18:00 (ဆိုးတာ) နဲ့ Linux ကို
  နှိုင်းယှဉ်ခဲ့တယ်၊ 18:56 Windows log က တစ်ခုတည်း ဖြေရှင်းပေးမယ့် ဟာ ဖြစ်ပေမဲ့ မသုံးခဲ့ဘူး
- **Rule ၃:** "device မပြန်ဘူး" နဲ့ "ငါ ပို့လို့ မရဘူး" ကို ဘယ်တော့မှ တစ်ခုတည်း error အဖြစ်
  မဖေါ်ပါ — ပေါင်းထားတာက host bug ကို SIM ပြဿနာလို့ လိုက်ရှာစေတယ်

## ⚠️ Latent Traps (မဖြစ်သေးဘူး၊ ဒါပေမဲ့ ချောင်းနေတာ)

Case မဟုတ်သေးပါ — 2026-08-30 settings cleanup (`fbd7b8b`) လုပ်ရင်း တွေ့ခဲ့တဲ့ ချောင်း ၃ ခု
(T1–T2 settings layer၊ T3 decoder)။ Symptom မရှိသေးလို့ fix မလုပ်ခဲ့ဘူး၊ ဒါပေမဲ့ နောက်ဆို
ရှင်းပြရ ခက်တဲ့ bug ဖြစ်လာနိုင်တယ်။
**T3 က v1.4.0 မှာ ကုဒ် ပြင်ပြီးသွားပြီ** (အောက်တွင်) — T1/T2 က ပြင်ရသေး။

### T1. `deepMerge` က **stored** key တွေကို iterate တာ — ဖျက်ထားတဲ့ setting က profile ထဲ မပျောက်

`src/lib/stores/settings.svelte.ts` ရဲ့ `deepMerge(target, source)` မှာ loop က
`Object.keys(source)` — `source` က `localStorage` (`sms-reader-settings`) ကနေ လာတဲ့ **stored
profile**၊ `target` က defaults။ ဆိုတော့ merge က "DEFAULT မှာ ရှိတဲ့ key" နဲ့ မကန့်သတ်ဘူး —
stored key **တိုင်း** result ထဲ ရောက်တယ်။

**အကျိုးဆက်:** `SettingsState` ကနေ field ဖျက်လိုက်တာဟာ user တစ်ဦးရဲ့ ရှိပြီးသား profile ကို
**မရှင်းလင်းပေးဘူး**။ `otp.otpPattern`၊ `developer.logLevel` စတာတွေ localStorage ထဲ ကျန်နေတယ်၊
ပြီးတော့ `saveSettings` က object တစ်ခုလုံးကို stringify တာမို့ **save တိုင်း ပြန်ရေးခံရတယ်**။

- ယခု အထိ **အပြစ်မရှိ** — ဘယ်သူမှ မဖတ်တာမို့
- **အနာဂတ် ချောင်း:** နာမည် တူတဲ့ field အသစ်တစ်ခု (ဥပမာ `developer.logLevel` ကို ပုံစံ
  ပြောင်းပြီး ပြန်ထည့်တာ) က **stale value ကို အမွေရမယ်** — default ကို မဟုတ်ဘူး။ ဒါက user
  ရဲ့ machine မှာပဲ ဖြစ်တာမို့ "ငါ့ဆီမှာ မဖြစ်ဘူး" class bug
- **Rule:** field နာမည်ကို semantic ပြောင်းပြီး ဘယ်တော့မှ recycle မလုပ်ပါနဲ့။ တကယ် လိုရင်
  `migrate()` (retention migration နဲ့ တူတဲ့ ပုံစံ) ထဲမှာ key ကို explicit `delete` လုပ်ပါ။
  `deepMerge` ကို DEFAULT key အလိုက် iterate ဖြစ်အောင် ပြောင်းလည်း ရတယ် — ဒါပေမဲ့ အဲ့ဒါက
  legacy shape တွေအတွက် migration အားလုံး အရင် ပြေးရမယ်လို့ ဆိုလိုတယ်

### T2. Settings page က data-driven၊ binding path မှာ **type check မရှိ**

`src/lib/pages/Settings.svelte` က field descriptor array (`{ key, label, type, bind, … }`) ကနေ
render တယ်၊ value access က `getNestedValue(obj: any, path: string): any` /
`setNestedValue(obj: any, …)` — path က `` `${field.bind}.${field.key}` `` string concat။

ဆိုတော့ **မရှိတဲ့ `bind`/`key` pair က compile error မတက်ဘူး**၊ `svelte-check` လည်း မဖမ်းဘူး —
runtime မှာ `undefined` ပဲ။ Checkbox အတွက် အဲ့ဒါက "အမြဲ off ပုံပေါက်တာ"၊ `setterFor(field.bind)`
က `undefined` ပြန်ရင် persist မဖြစ်တာ။

- `fbd7b8b` မှာ field ၁၁ ခု ဖျက်တာ type-safe ဖြစ်ခဲ့တာ **တိုက်ဆိုင်မှုသာ** —
  `SettingsState` ကနေ ဖျက်ပြီး descriptor ကို ချန်ခဲ့ရင် build အောင်မြင်ပြီး switch က
  တိတ်တဆိတ် ပျက်နေမယ်
- **Rule:** control အသစ် ထည့်/ဖျက် ရင် **Settings page ကို ကိုယ်တိုင် နှိပ်ပြီး စမ်းပါ** —
  reload ပြီးရင် value ကျန်လား၊ consumer တကယ် တုံ့ပြန်လား။ Type checker က ဒီအလွှာမှာ
  မကာကွယ်ပေးဘူး (doc 04 §H)

### T3. `KW_CONFIRM` keyword constant က စာလုံးပေါင်း မှားခဲ့တယ် (OTP gate — **v1.4.0 မှာ ပြင်ပြီး**)

**အရင်က ဒီလို ဖြစ်ခဲ့တယ် (context အတွက် ကျန်ထား):** `src-tauri/src/core/decoder.rs` ရဲ့
keyword constant က

```rust
const KW_CONFIRM: &str = "\u{1021}\u{1010}\u{1014}\u{103A}\u{1015}\u{103C}\u{102F}"; // = အတန်ပြု
```

`\u{1014}` က **န**။ မြန်မာ "confirm" က **အတည်ပြု** — `\u{100A}` (**ည**) ဖြစ်ရမယ်။ ဆိုတော့
`KEYWORD_RE` gate ထဲက ဒီ alternative က **တကယ့် SMS ဘယ်တော့မှ မ match ခဲ့ဘူး** (မရှိတဲ့ စာလုံးပေါင်း)။

- **အရင် Impact:** အခြား keyword (`otp`, `code`, `pin`, `ကုဒ်`, `လုံခြုံ`, `verify`…) မပါဘဲ
  **"အတည်ပြု" တစ်ခုတည်း** သုံးတဲ့ မြန်မာ OTP SMS က gate ကို မဖြတ်ဘူး → OTP မတွေ့ဘူး
  (silent miss — decoder က `None` ပြန်တာမို့ error မတက်)
- Unit test တစ်ခုမှ ဒီ constant ကို မထိထားခဲ့ဘူး၊ ဒါကြောင့် green test suite က ဒါကို မဖမ်းခဲ့ဘူး
- **အခု ပြင်ပြီးသွားပြီ (v1.4.0):** `src-tauri/src/core/decoder.rs:7` က `\u{100A}` သုံးပြီ —
  constant က **အတည်ပြု** ဖြစ်သွားပြီ။ Regression test လည်း ပါလာပြီ:
  `src-tauri/src/core/decoder.rs:943` `otp_myanmar_confirm_keyword` — "အတည်ပြု" body နဲ့
  `extract_otp` က OTP ပြန်ရမယ်၊ ပြီးတော့ **keyword ဖြုတ်ထားတဲ့ တူတူ body** က `None`
  ပြန်ရမယ် (negative control — gate ကို တကယ် ဖွင့်ပေးတာ ဒီ keyword ဆိုတာ သက်သေ)
- **ချောင်းအဖြစ် ဒီအတိုင်း မှတ်ထား** — bug class က ကျန်နေတယ်။ **Keyword constant ထဲက Unicode
  escape sequence ကို မျက်လုံးနဲ့ စစ်လို့ မရဘူး**: `\u{1014}` ↔ `\u{100A}` က code point တစ်ခုပဲ
  ကွာတယ်၊ diff/review မှာ ဘယ်သူမှ မမြင်ဘူး၊ compiler နဲ့ regex ၂ ခုလုံးအတွက်လည်း valid ပဲ။
  **render ဖြစ်လာတဲ့ စာလုံးကို assert လုပ်တဲ့ test** ပဲ ဖမ်းနိုင်တာ — escape ကို ပြန်ဖတ်ကြည့်တာ မဟုတ်ဘူး
- **Rule:** မြန်မာ (ဒါမှမဟုတ် non-ASCII) keyword constant အသစ် ထည့်တိုင်း **သူ့ကို match
  ဖြစ်ရမယ့် body တစ်ခု + မဖြစ်ရမယ့် body တစ်ခု** နဲ့ test ထည့်ပါ။ Constant တစ်ခုတည်းကို
  မှန်လား စစ်တာ မလုံလောက်ဘူး — gate တစ်ခုလုံးကို ပြေးခိုင်းပါ

## Bonus UX Notes

- **User-reported 404 on releases page** but server-side probes said HTTP 200 (public repo) →
  browser cache/transient CDN — hard refresh/incognito/direct tag URL နဲ့ cross-check လုပ်ဘိုး။
- **`gh` CLI:** `export GH_PAGER=cat PAGER=cat` မလုပ်ရင် interactive pager block ဖြစ်နိုင် (non-interactive shell trap);
  long polls ကို background process + `/tmp/*.txt` capture pattern သုံး။
