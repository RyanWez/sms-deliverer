# 03 — Troubleshooting Casebook (တကယ်ဖြစ်ခဲ့တဲ့ ၁၇ ခု)

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

## 1️⃣4️⃣ Live mode message ကို Delete → "Deleted 1 message(s) (1 SIM slot(s) freed)" ပြပေမယ့် SIM ပေါ် ကျန်နေတာ

- **Symptom:** Live mode ပြေးနေစဉ် ရောက်လာတဲ့ SMS ကို Inbox ကနေ Delete လုပ်ရင် row က ပျောက်တယ်၊
  status line က `Deleted 1 message(s) (1 SIM slot(s) freed)` ဆိုတဲ့ **clean success** လိုင်း တင်တယ်။
  ဒါပေမယ့် message က SIM ပေါ်မှာ ကျန်နေတယ်။ `live::seen` က fingerprint မှတ်ထားတာမို့ အဲ့ session
  အတွင်း ပြန်မပေါ်လာဘူး — slot က **မမြင်ရဘဲ ပိတ်နေတယ်**။ SIM slot ၂၀–၅၀ ပဲရှိတဲ့ card မှာ
  တဖြည်းဖြည်း ပြည့်လာပြီး modem က SMS အသစ် လက်ခံရာမှာ တိတ်တဆိတ် ငြင်းလာမယ်
- **Root Cause:** `decoder::parse_cmgr` နဲ့ `parse_pdu_cmgr` နှစ်ခုလုံး `index: 0` ကို **hardcode**
  ထားတယ်။ `+CMGR` header က status ပါပေမယ့် index မပါဘူး — slot ကို သိတာ `AT+CMGR={idx}` ပို့တဲ့
  caller (`live::handle_cmgr`) တစ်ခုတည်းပဲ၊ ဒါပေမယ့် parse ပြီးတဲ့ message ထဲ ပြန်မရေးထားဘူး။
  ဒါကြောင့် live mode ကာလအတွင်း ရောက်လာတဲ့ SMS တိုင်း `index == 0`၊ concat ဆိုရင်
  `part_indices == [0]` (`Reassembler` က fragment ရဲ့ slot ကို `msg.index` ကနေ ယူတာမို့)
- **`confirmed_removals` invariant ကို ဘယ်လို ဖြတ်သွားလဲ** — ဒါက အရေးကြီးဆုံး အပိုင်း:
  1. `message_slots` က `[0]` ပြန်ပေး → `delete_each` က `AT+CMGD=0` ပို့
  2. SIM slot က **1 ကနေ** စတာမို့ modem က ငြင်း
  3. `slots_still_present` က `AT+CMGL="ALL"` ပြန်ဖတ်တယ် — slot 0 က list ထဲ **မပါဘူး**၊
     ဘာလို့လဲဆိုတော့ **ဒါ မရှိတဲ့ slot ဖြစ်တာမို့**
  4. "list ထဲ မပါဘူး" ကို `confirm_delete` က "ပျက်သွားပြီ" ဆိုတဲ့ **သက်သေ** အဖြစ် ဖတ်တယ်
  → `ok: true, deleted: 1, indices: [0]` → `confirmed_removals` က row ကို ခွင့်ပြု။
  SIM ပြန်ဖတ်တဲ့ confirmation တစ်ခုလုံး **အလုပ်လုပ်ပေမယ့် အဖြေ လွဲတယ်** — မရှိတဲ့ slot ရဲ့ absence
  က ဘာမှ မသက်သေဘူး
- **Blast radius (တိတိကျကျ):** live mode ကနေ ရလာတဲ့ row ကို **manual delete** လုပ်တဲ့ path
  တစ်ခုတည်းပဲ။ Scan path (`parse_pdu_list`/`parse_text_mode_list`) က `+CMGL` header ကနေ slot
  မှန်မှန် ထည့်တယ်။ Retention sweep (`live::sweep_expired`, `modem::expire_old`) က SIM ကို
  **ကိုယ်တိုင် ပြန်ဖတ်ပြီး** list parser သုံးတာမို့ မထိခိုက်ဘူး
- **Fix (v1.5.0):** `parse_cmgr(resp, port, idx)` / `parse_pdu_cmgr(resp, port, idx)` — slot ကို parameter
  အဖြစ် သွင်း၊ default မထား။ `live::handle_cmgr` က `idx` ကို `asm.push` **မတိုင်မီ** ပေးတာမို့
  `finish()` ရဲ့ `part_indices` က တကယ့် slot တွေ စုမယ်။ `decode_deliver` ရဲ့ `index: 0` က
  ကျန်ထားပေမယ့် "raw PDU မှာ slot မပါဘူး၊ caller နှစ်ခုလုံး overwrite လုပ်တယ်" ဆိုတဲ့ comment ထည့်
- **Second line of defence:** `message_slots` နဲ့ `models::expired_indices` နှစ်ခုလုံး
  non-positive index ကို ဖယ်ပစ်တယ်။ `confirmed_removals` က `!idxs.is_empty()` စစ်ပြီးသားမို့
  slot မရှိတဲ့ row က **KEPT** ဖြစ်မယ် — အနာဂတ်မှာ ဘယ် path ကနေ 0 ရောက်လာလာ တိတ်တဆိတ်
  ဖျက်ခြင်း မဖြစ်တော့ဘူး
- **Rule ၁:** **"မရှိတာကို သက်သေ အဖြစ် သုံးရင် sentinel တွေကို အရင် ဖယ်ပါ။"** Absence-based
  confirmation (`slots_still_present`) က ရှာနေတဲ့ key ဟာ တကယ် ရှိနိုင်တဲ့ key ဖြစ်မှ အလုပ်လုပ်တယ်။
  မရှိနိုင်တဲ့ key ဆိုရင် အဖြေက အမြဲ "ပျက်သွားပြီ" ဖြစ်တယ်
- **Rule ၂:** `0` ကို "မသိဘူး" အဖြစ် double meaning **မပေးပါ**။ `index: Option<i32>` ဆိုရင်
  ဒီ bug က compile error ဖြစ်နေမှာ။ `i32` + hardcoded `0` က တိတ်တဆိတ် လွဲသွားတယ်
- **Rule ၃:** Test က `text`/`from` ကို စစ်တာ မလုံလောက်ဘူး။ `pdumgr_single_read` က ရှိပြီးသား
  ဖြစ်ပေမယ့် `index` ကို မစစ်ခဲ့တာမို့ bug က ဖမ်းမမိခဲ့ဘူး — **delete/cleanup ရဲ့ input ဖြစ်တဲ့
  field တိုင်းကို assert လုပ်ပါ**

## 1️⃣5️⃣ Inbox Search box က ရိုက်လိုက်တဲ့ စာကို ဖျက်ပစ်တာ — search က လုံးဝ အလုပ်မလုပ်ဘူး

- **Symptom:** Inbox ရဲ့ Search messages ကွက်လပ်ထဲ ရိုက်ရင် စာ မတည်ဘူး။ Browser preview
  (localhost:1420) မှာ တကယ် စမ်းကြည့်တာ — `+469602294397` (၁၃ လုံး) ကို တစ်လုံးချင်း ရိုက်ပြီးတဲ့အခါ
  **field က ဗလာ**၊ message count က `35 msgs` အတိုင်း ကျန်၊ filter က တစ်ခါမှ မသက်ဝင်ဘူး။
  "အက္ခရာ တစ်ချို့ ကျော်သွားတာ" မဟုတ်ဘူး — **search feature တစ်ခုလုံး အလုပ်မလုပ်တာ**
- **Root Cause:** `FilterBar.svelte` ရဲ့ sync `$effect`:

  ```js
  if (messagesStore.query === '' && localQuery !== '') {
    localQuery = '';                       // ← debounceTimer guard မပါ
  } else if (messagesStore.query !== localQuery && debounceTimer === null) {
  ```

  Branch ၂ က typing အတွင်း clobber မဖြစ်အောင် `debounceTimer === null` နဲ့ ကာထားတယ်။
  **Branch ၁ မကာဘူး**။ `debounceTimer` က `$state` မဟုတ်တာမို့ effect က မ track ဘူး —
  dependency က `messagesStore.query` နဲ့ `localQuery` ပဲ။ ဆိုတော့ keystroke တိုင်းက
  `localQuery` ကို ပြောင်းတယ် → effect ပြန်ပြေးတယ် → `query` က `''` ရှိသေးတာမို့
  `localQuery = ''` → `value={localQuery}` က DOM ကို ပြန်ရေးတယ်။ ဒါဟာ `query` က `''`
  ဖြစ်နေတဲ့ အချိန်တိုင်း (= စရိုက်တဲ့အခါ၊ clear လုပ်ပြီးတဲ့အခါ) ဖြစ်တယ်
- **ဒီ effect က မဖြစ်နိုင်တဲ့ ကိစ္စကို ကာနေတာ:** comment က "store is cleared externally"
  လို့ ဆိုပေမယ့် `messagesStore.query` ကို ရေးတဲ့သူ က `FilterBar` **တစ်ခုတည်း** ပဲ
  (`onSearchInput` နဲ့ `clearSearch` — repo တစ်ခုလုံး grep ပြီး အတည်ပြုတယ်)။
  ဆိုတော့ mirror လုပ်ရမယ့် external update ဆိုတာ မရှိဘူး။ Page navigation ကို
  `let localQuery = $state(messagesStore.query)` initialiser ကိုယ်တိုင် ဖြေရှင်းပေးပြီးသား
  (Inbox က remount ဖြစ်တဲ့အခါ store က query ကို ကျန်ထားတယ်)
- **Fix (v1.5.0):** effect ကို ဖျက်လိုက်တယ်။ Input က `localQuery` ကို ပိုင်တယ်၊ debounce က
  store ကို ရေးတယ် — one-way။ `src/lib/pages/Ports.svelte:28-42` က ဒီပုံစံ မှန်မှန်
  ရေးထားပြီးသား (`rawQuery`/`debouncedQuery`, sync effect မထား)
- **အတည်ပြုမှု (A/B, browser preview):** effect ရှိတဲ့အခါ — field ဗလာ၊ `35 msgs` အတိုင်း။
  ဖျက်ပြီးတဲ့အခါ — field က `+469602294397` အပြည့် ကိုင်တယ်၊ `1 msgs` ဆီ ကျဆင်းတယ်၊
  ကျန်တဲ့ row က အဲ့ sender ကိုယ်တိုင်။ Clear button ပြီးရင် `35 msgs` ပြန်လာတယ်၊
  clear လုပ်ပြီး ပြန်ရိုက်တာလည် အလုပ်လုပ်တယ် (effect အဆိုးဆုံး ဖြစ်တဲ့ ကိစ္စ)
- **Rule ၁:** **Runes မှာ plain `let` က effect ရဲ့ guard မဖြစ်နိုင်ဘူး။** `debounceTimer` က
  reactive မဟုတ်တာမို့ effect က သူ့ကို မ track ဘူး — ဒါပေမယ့် effect **အထဲမှာ** သူ့ကို
  ဖတ်တာက "ကာထားပြီ" ဆိုတဲ့ အမြင် ပေးတယ်။ Branch တစ်ခုမှာ guard ရေးပြီး နောက်တစ်ခုမှာ
  မရေးထားတာက ဒီ bug ရဲ့ တိကျတဲ့ ပုံစံ
- **Rule ၂:** **DOM ရဲ့ value ကို source of truth နှစ်ခု မထားပါ။** `value={localQuery}` က
  binding ဖြစ်ပြီး input event က `localQuery` ကို ရေးတယ် — ဒီအလယ်မှာ ဝင်ပြီး `localQuery`
  ကို ရေးတဲ့ effect တိုင်းက user ရဲ့ လက်ကို ယှဉ်ပြိုင်တယ်
- **Rule ၃:** "external update ကို sync လုပ်တဲ့ effect" ရေးမယ်ဆိုရင် **external writer
  တကယ် ရှိလား grep လုပ်ပါ**။ မရှိရင် အဲ့ effect က ကာကွယ်မှု မဟုတ်ဘဲ bug ဖြစ်တယ်
- **Rule ၄:** Frontend ကို unit test မရသေးတဲ့ အလွှာ (component) ဖြစ်ရင် **browser preview
  မှာ A/B စမ်းပါ** — fix ကို stash လုပ်ပြီး အရင်အခြေအနေ ပြန်ပြေးခိုင်းတာက "ပြင်ပြီး
  အလုပ်လုပ်တယ်" နဲ့ "ဒါ တကယ် bug ဖြစ်ခဲ့တယ်" နှစ်ခုလုံးကို သက်သေပြတယ်

## 1️⃣6️⃣ Port ခဏ busy ဖြစ်တာနဲ့ "NO MODEM" ဖြစ်ပြီး SIM နံပါတ် disk ပေါ်ကနေ ပျောက်တာ

- **Symptom ၂ မျိုး၊ တစ်ခုတည်းသော အကြောင်းရင်း:**
  1. **Detect** ပြေးတဲ့အချိန် ModemManager ဒါမှမဟုတ် တခြား process က port ကို ခဏ ဖမ်းထားရင်
     (ဒါမှမဟုတ် EBUSY ခံရင်) အဲ့ port က `NO MODEM` ဖြစ်တယ်၊ deselect ခံတယ်၊ ပြီးတော့
     **slot→ICCID mapping က `sim_numbers.csv` ကနေ အပြီးအပိုင် ပျောက်တယ်** — SIM နံပါတ်
     ပြန်ရဖို့ Get SIM Numbers ပြန်ပြေးရမယ်
  2. **Refresh** (background timer, ၃၀ စက္ကန့်တစ်ခါ default) ပြီးတဲ့အခါ
     "Reconnecting: Port lost: EIO" / "Serial I/O failed: …" ဆိုတဲ့ error text က **ပျောက်တယ်**၊
     ပြီးတော့ **ပြန်မလာဘူး** — port က outage ကျန်တဲ့အချိန်တစ်လျှောက် ရိုးရိုး idle row အဖြစ် ပြတယ်
- **Root Cause (၁):** `detect_ports` က probe ရဲ့ ရလဒ် ၄ မျိုးကို **နှစ်မျိုးအဖြစ် ချုံ့**ခဲ့တယ်:
  ```rust
  let (alive, iccid) = match probed {
      Ok(Ok(r)) => (r.alive, r.iccid),
      _ => (false, None),          // ← Err နဲ့ panic နှစ်ခုလုံး "SIM မရှိ" ဖြစ်သွားတယ်
  };
  ```
  ပြီးတော့ `ProbeResult` မှာ error field **မရှိခဲ့ဘူး**၊ ဒါကြောင့် `probe_failure_reason` ရဲ့
  "silence vs transport failure" ခွဲခြားမှု (scan/USSD/live သုံးခုလုံး လိုက်နာတဲ့ဟာ) က ဒီ path
  တစ်ခုတည်းမှာ လုံးဝ ပျောက်ခဲ့တယ်။ `alive == false` ဖြစ်တာနဲ့ `sim_dir.clear_slot(&path)`
- **Root Cause (၂):** `merge_ports` က `live_error: None` ကို **unconditional** ထားခဲ့တယ်။
  `OutageLatch` က outage တစ်ခုအတွက် event တစ်ခုပဲ ပို့တာမို့ (ထပ်ခါထပ်ခါ မပို့ဘူး) ဖျက်လိုက်တာ
  **ပြန်မလာဘူး**။ `alive` ကို carry လုပ်ပြီးသားမို့ **silence တစ်ခုတည်း** သာ ကျန်ပြီး
  operator အလိုအရေးဆုံး ဖြစ်တဲ့ နှစ်ခု က ပျောက်တဲ့ နှစ်ခု ဖြစ်တယ်
- **Fix (၁) — v1.5.0:** `ProbeResult` မှာ `failure: Option<String>` + `proved_empty()` ထည့်၊ ပြီးတော့
  `ProbeVerdict` enum ၃ မျိုး:
  | Verdict | `alive` | `checked` | `iccid` | `sim_dir` | `live_error` |
  |---|---|---|---|---|---|
  | `Alive(iccid)` | `Some(true)` | `true` | set (ရရင်) | `set_slot` | `None` |
  | `Empty` (silence ပဲ) | `Some(false)` | `false` | `None` | `clear_slot` | `None` |
  | `Inconclusive(why)` | **မထိ** | **မထိ** | **မထိ** | **မထိ** | `Some(why)` |

  `Empty` က `alive = Some(false)` သတ်မှတ်ခွင့်ရှိတဲ့ **တစ်ခုတည်းသော** verdict
- **Fix (၂) — v1.5.0:** `live_error` ကို refresh အတွင်း carry လုပ်တယ် — `live_ready` လိုပဲ **tty name
  တူမှ**။ Renumber ဖြစ်ပြီးရင် အဲ့ message က မရှိတော့တဲ့ name ရဲ့ worker အကြောင်း ဖြစ်တာမို့
  လွဲမှားစေတယ်။ **Expire လုပ်စရာ မလိုဘူး**: `start_live`/`stop_live` နှစ်ခုလုံး boundary မှာ
  `live_error` အားလုံး ရှင်းတယ်၊ `detect_ports` က port အလိုက် ကိုယ်တိုင် overwrite တယ်
- **Trade-off (တမင် ရွေးထားတာ):** Inconclusive port က **selected အတိုင်း ကျန်**တယ်၊ ဒါကြောင့်
  နောက် scan က သူ့ timeout ကို ပေးရမယ်။ ICCID mapping ပျောက်တာနဲ့ စာရင် ဒါက အများကြီး သက်သာတယ် —
  ပြီးတော့ status line က `N port(s) could not be probed — left as they were` လို့ ရှင်းရှင်း ပြောတယ်
- **လက်ရှိ ကျန်နေတဲ့ အပိုင်း:** အရင် `Empty` ဖြစ်ခဲ့တဲ့ port (`alive == Some(false)`) မှာ
  Inconclusive verdict တင်ရင် `portStatus` က `alive === false` branch ကို အရင် စစ်တာမို့
  `NO MODEM` ပဲ ပြပြီး reason text ကို မပြဘူး။ **တမင် ချန်ထားတာ** — branch order ပြောင်းရင်
  transient failure တစ်ခုအတွက် bank ထဲက slot ဗလာ အားလုံး အနီ ဖြစ်မယ် (`utils/port.ts` ရဲ့
  comment ကိုယ်တိုင် အဲ့ဒါကို သတိပေးထားတယ်)။ ရှိပြီးသား evidence က "ဗလာ" ဆိုတာမို့ အဲ့ဒါ ပြတာ မှန်တယ်
- **Rule ၁:** **"မသိဘူး" ကို "မရှိဘူး" နဲ့ တစ်ခုတည်း မလုပ်ပါ။** State ၂ ခုပဲ ရှိတဲ့
  `bool`/`(bool, Option<_>)` က ဒီ ချုံ့မှုကို ဖိတ်ခေါ်တယ် — verdict ၃ မျိုးဆိုရင် enum ထားပါ၊
  compiler က arm တိုင်း ဖြေရှင်းခိုင်းလိမ့်မယ်
- **Rule ၂:** `match probed { Ok(Ok(r)) => …, _ => … }` ဆိုတဲ့ **catch-all arm က ခွဲခြားမှုကို
  တိတ်တဆိတ် စားတယ်**။ `Err` variant တွေကို တစ်ခုချင်း ဖြေရှင်းပါ
- **Rule ၃:** State ကို "ရှင်းလင်းတယ်" ဆိုတဲ့ code ရေးတဲ့အခါ **အဲ့ဒါ ပြန်လာမလား** ကို စစ်ပါ။
  Latch/one-shot event ကနေ လာတဲ့ state ကို ဖျက်တာက **အပြီးအပိုင် ဖျက်တာ** ဖြစ်တယ်

## 1️⃣7️⃣ Live SIM sweep က "deleted N" လို့ log တင်ပေမယ့် SIM ပြည့်လာတာ — copy နှစ်ခု drift ဖြစ်တာ

- **Symptom:** Live mode ဆက်တိုက် ပြေးနေတဲ့ bank မှာ log က `SIM cleanup deleted N expired
  message(s)` လို့ ၁၀ မိနစ်တစ်ခါ (`SIM_SWEEP_EVERY`) တင်နေတယ်၊ ဒါပေမယ့် SIM က တဖြည်းဖြည်း
  ပြည့်လာပြီး modem က SMS အသစ် လက်ခံရာမှာ တိတ်တဆိတ် ငြင်းလာတယ်။ **sweep ရှိတဲ့ အကြောင်းရင်း
  အတိအကျ ဖြစ်တဲ့ failure**
- **Root Cause — operation တစ်ခုကို implementation နှစ်ခု ရေးထားပြီး တစ်ခုက drift ဖြစ်ခဲ့တာ:**
  | | `modem.rs` (scan path) | `live.rs::delete_indices` (drifted copy) |
  |---|---|---|
  | `AT+CMGD` ရလဒ် စစ်တာ | `l.trim() == "OK"` (line တစ်ခုလုံး) | `resp.contains("OK")` |
  | SIM ပြန်ဖတ်ပြီး အတည်ပြုတာ | `slots_still_present` + `confirm_delete` | **ဘာမှ မရှိ** |

  `contains("OK")` က `+CMS ERROR: 321 ... NOT OK` စတဲ့ စာသား၊ command echo၊ ဒါမှမဟုတ်
  unsolicited line တစ်ခုထဲ ပါလာတဲ့ `OK` ကိုပါ ခံယူတယ်။ ပြီးတော့ confirmation မရှိတာမို့
  ရေတွက်တာက modem ကို **ခိုင်းလိုက်တဲ့** အရေအတွက်၊ တကယ် **ပျက်သွားတဲ့** အရေအတွက် မဟုတ်ဘူး
- **Fix (v1.5.0) — helper ကို ပေါင်းလိုက်တာ (structural)**: `modem::delete_confirmed(ch, port, indices,
  list_cmd)` ကို `pub(crate)` ထုတ်ပြီး `live::delete_indices` ကို **ဖျက်**လိုက်တယ်။
  `delete_messages` (port ကို ကိုယ်တိုင် ဖွင့်တယ်) နဲ့ live sweep (မဖွင့်နိုင်ဘူး) နှစ်ခုလုံး
  အခု entry point တစ်ခုတည်း သုံးတယ် — **ထပ် drift ဖြစ်လို့ မရတော့ဘူး**
- **`list_cmd` parameter ဘာလို့ လိုလဲ:** `confirm_delete` က `AT+CMGL="ALL"` (text mode form)
  ကို hardcode ထားခဲ့တယ်။ Live worker က ရနိုင်သရွေ့ **PDU mode** ပြေးတယ် — အဲ့မှာ quoted form
  က `ERROR` ပြန်တယ် → `slots_still_present` က `None` → per-command count ဆီ တိတ်တဆိတ်
  ပြန်ကျမယ် (= ပြင်ချင်တဲ့ bug ကို ပြန်ရမယ်)။ `list_all_cmd(pdu_mode)` က `AT+CMGL=4` /
  `AT+CMGL="ALL"` ရွေးပေးတယ်
- **Test:** `the_sweep_deletes_high_slots_first_and_confirms_against_the_sim` (highest-first
  order + **re-read က တကယ် ပို့တာ** — အရင် live loop က မပို့ခဲ့တာ),
  `the_sweep_confirms_with_the_list_form_for_the_mode_it_is_in`
- **Rule ၁:** **Operation တစ်ခုကို implementation နှစ်ခု မထားပါ။** ဒီ repo မှာ `setup_sms_mode`
  ကို scan/live နှစ်ခု ခွဲရေးထားခဲ့တာ drift ဖြစ်ပြီး ပေါင်းခဲ့ရတယ် (`modem.rs` doc comment မှာ
  မှတ်ထား) — ဒါက **တူတဲ့ သင်ခန်းစာ ဒုတိယအခေါက်**။ Channel-taking helper အဖြစ် ထုတ်ပြီး
  ခေါ်ပါ၊ ကူးမရေးပါ
- **Rule ၂:** AT reply ကို `contains("OK")` နဲ့ **ဘယ်တော့မှ** မစစ်ပါ။ `lines().any(|l| l.trim()
  == "OK")` ပဲ သုံးပါ — `OK` က result code ဖြစ်ရမယ်၊ စာသား substring မဟုတ်
- **Rule ၃:** Helper ကို ပေါင်းတဲ့အခါ **mode-dependent constant တွေ ဝှက်နေလား** စစ်ပါ။
  `AT+CMGL="ALL"` က hardcode ဖြစ်နေတာမို့ ပေါင်းလိုက်တာနဲ့ PDU-mode caller ဆီမှာ တိတ်တဆိတ်
  degrade ဖြစ်မယ် — parameter အဖြစ် ထုတ်လိုက်တာက အဲ့ဒါကို ဖြေရှင်းတယ်

## 1️⃣8️⃣ `reqwest` ကို dependency အသစ် ထည့်လိုက်တာနဲ့ crate ၂၂ ခု + cmake/C toolchain တက်လာတာ၊ ပြီးတော့ client တည်ဆောက်ရင် panic ဖြစ်တာ

- **Symptom (ဆက်တွဲ ၂ ခု):** Telegram forwarder အတွက် `reqwest = { version = "0.13.4",
  features = ["blocking", "json", "socks"] }` လို့ ရေးလိုက်တယ်။ Compile က အောင်မြင်တယ်၊
  test အားလုံး pass တယ် — ဒါပေမယ့်
  ၁။ `Cargo.lock` က package **501 → 523** ဖြစ်သွားတယ် (`aws-lc-rs`, `aws-lc-sys`, `cmake`,
  `fs_extra`, `h2`, `encoding_rs`, `chacha20`, `core-foundation` …)။ `aws-lc-sys` က
  **cmake + C compiler** လိုတယ် — Windows CI leg အတွက် failure mode အသစ်
  ၂။ `default-features = false` + `rustls-no-provider` နဲ့ ပြန်ပြင်လိုက်တာနဲ့ crate မတက်တော့ဘူး၊
  ဒါပေမယ့် `build_client()` က **panic** ဖြစ်တယ်:
  `No rustls crypto provider is configured. When using the rustls-no-provider feature you must
  install a crypto provider before building a Client`
- **Root Cause:** `reqwest` ရဲ့ default features ထဲ `default-tls` ပါတယ် → `rustls` →
  `__rustls-aws-lc-rs` (aws-lc-rs provider)၊ ပြီးတော့ `charset` (encoding_rs)၊ `http2` (h2)၊
  `system-proxy`။ ဒါပေမယ့် `tauri-plugin-updater` က reqwest ကို **`default-features = false`,
  features `["json", "stream"]` + `rustls-no-provider`** နဲ့ သုံးပြီး `rustls` ကို
  `features = ["ring"]` နဲ့ ဆွဲတယ် — ဆိုတော့ ဒီ binary ထဲ **ring-backed rustls ရှိပြီးသား**၊
  ငါက ဒုတိယ provider တစ်ခုလုံး ဆွဲထည့်လိုက်တာ။
  `rustls-no-provider` က process-level provider ကို caller ဆီ လွှဲတယ် — updater က
  `updater.rs:446` မှာ `CryptoProvider::get_default().is_none()` ဆိုရင် install တယ်၊ ဒါပေမယ့်
  **updater ပြေးမှ**။ Operator က Verify ကို updater မပြေးခင် နှိပ်ရင် provider မရှိဘူး → panic
- **Fix:** feature set ကို updater နဲ့ **အတိအကျ တူ**စေပြီး လိုတာ ၂ ခုပဲ ထပ်ထည့်တယ်
  (`blocking` — ဒီ crate က OS thread နဲ့ blocking I/O သုံးတယ်၊ async task မဟုတ်ဘူး ·
  `socks` — feature list က `socks = []`၊ crate တစ်ခုမှ မတက်ဘူး)။ ပြီးတော့ `rustls` ကို
  direct dep အဖြစ် (ring, default-features off — lock ထဲ ရှိပြီးသား) ထည့်ပြီး
  `telegram::ensure_crypto_provider()` (`std::sync::Once`) က `build_client` အတွင်းမှာ
  install လုပ်တယ် — updater ကို မမှီခိုဘူး။ `install_default()` က ရှိပြီးသားဆိုရင် `Err`
  ပြန်တယ်၊ အဲဒါ မျှော်လင့်ထားတာမို့ `let _ =`
- **ရလဒ်:** `Cargo.lock` diff က **line ၃ ခုပဲ** (`futures-sink`, `futures-channel` edge ၂ ခု +
  `sms-tauri` ရဲ့ `reqwest` entry)။ Crate အသစ် **သုည**
- **Test:** `build_client_succeeds_with_no_proxy` (provider ကို ဖမ်းတာ — panic ဖြစ်လို့
  ဒီ trap ကို CI မှာ တွေ့တယ်၊ operator က Verify နှိပ်တဲ့အခါ မဟုတ်ဘူး)၊
  `build_client_accepts_a_socks5h_proxy` (`socks` feature ကျွတ်သွားရင် fail — ပိတ်ထားတဲ့
  network မှာ ဒီ feature ကို အသုံးဝင်စေတဲ့ setting တစ်ခုတည်း)၊
  `build_client_treats_a_blank_proxy_as_direct`၊ `build_client_rejects_a_malformed_proxy`
- **Rule ၁:** Dependency အသစ် ထည့်ပြီးတာနဲ့ **`Cargo.lock` ရဲ့ package အရေအတွက်ကို တိုက်ပါ**
  (`grep -c '^\[\[package\]\]'`)။ "Cargo.lock ထဲ ရှိပြီးသားပါ" ဆိုတာ **feature set တူတာကို
  မဆိုလိုဘူး** — crate တူပေမယ့် feature မတူရင် transitive tree က တခြားဖြစ်တယ်
- **Rule ၂:** Crate တစ်ခုကို plugin တစ်ခုက ဆွဲထားပြီးသားဆိုရင် **အဲဒီ plugin ရဲ့ Cargo.toml ကို
  ဖတ်ပြီး feature set ကို ကူးပါ**။ ခန့်မှန်းရင် ဒုတိယ TLS stack တစ်ခုလုံး ဆွဲမိတယ်
- **Rule ၃:** `*-no-provider` / `*-no-default` လို feature တွေက **runtime setup ကို caller ဆီ
  လွှဲတယ်**။ Compile ဖြစ်တာက အလုပ်လုပ်တာ မဟုတ်ဘူး — client/handle တည်ဆောက်တဲ့ test
  တစ်ခု ရေးပါ (network မလိုဘူး၊ hardware မလိုဘူး)

## 1️⃣9️⃣ Network ခဏ ပြတ်တာနဲ့ OTP က Telegram ကို ဘယ်တော့မှ မရောက်တော့ဘူး — error အားလုံးကို တစ်မျိုးတည်း သတ်မှတ်ခဲ့တာ

- **Symptom:** Field run (2026-09-03 20:58) မှာ live OTP တစ်ခု ဝင်လာတယ်၊ Inbox မှာ ပေါ်တယ်၊
  ဒါပေမယ့် Telegram ထဲ **ဘယ်တော့မှ မရောက်ဘူး**။ Log မှာ WARN တစ်ခုပဲ:
  `Telegram forward failed: Could not reach api.telegram.org: error sending request for url
  (https://api.telegram.org/bot<token redacted>/sendMessage)`。
  ၁၀ မိနစ်အလိုမှာ (20:48) တူတဲ့ path က အလုပ်လုပ်ခဲ့တယ်
- **အရင် အတည်ပြုလိုက်တာ — token redaction က တကယ့် failure ပေါ်မှာ ကိုင်တယ်:**
  `<token redacted>` ဆိုတာ `03 §18` ရဲ့ ကာကွယ်မှု ဖြစ်တယ်။ အဲဒါ မရှိရင် bot token က
  `app.log` (5 MB မှ rotate) နဲ့ Logs page ပေါ် **အတိအကျ** ရေးမိမယ်
- **Root Cause — `SendError::Other` က မတူတဲ့ အရာ ၂ ခုကို ကိုယ်စားပြုနေတာ:**

  | အမျိုးအစား | ဥပမာ | ပြန်ကြိုးစားရင် |
  |---|---|---|
  | Telegram က **ငြင်း**တာ | `401 Unauthorized`၊ `chat not found` | ဘယ်တော့မှ မရဘူး |
  | **လမ်း မရောက်**တာ | DNS၊ route ပျက်၊ timeout၊ ISP block page | ခဏနေ **ရနိုင်တယ်** |

  `forwarder::run` က `Err(e) => { report(); }` နဲ့ **နှစ်ခုလုံးကို လက်လွှတ်**ခဲ့တယ် —
  comment မှာ "keeping it would block every code behind it" လို့ ရေးထားတာ ငြင်းပယ်တာ
  အတွက်ပဲ မှန်တယ်၊ transport failure အတွက် **queue ရှိတဲ့ အကြောင်းရင်း ကိုယ်တိုင်ကို ဖျက်**လိုက်တာ
- **ဘာလို့ ခဏ ပြတ်သွားလဲ (စစ်ပြီး):** `api.telegram.org` က A + AAAA နှစ်မျိုးလုံး ရှိတယ်။
  ဒီစက်မှာ **IPv6 route က ပျက်နေတယ်** (`curl -6` → 0 ms မှာ "Could not connect"၊
  `curl -4` → `302`, 0.3 s)၊ ပြီးတော့ system resolver က IPv6 ကို ဦးစားပေးတယ်
  (`getent hosts` က AAAA ကို အရင် ပြန်တယ်)။
  **ဒါပေမယ့် connector ကို IPv4 အတင်း မခိုင်းရ:** 20:48 က အလုပ်လုပ်ခဲ့တာက hyper ရဲ့
  happy-eyeballs fallback ကိုင်နေတာ သက်သေပြတယ်။ IPv6-only network တစ်ခုပေါ်မှာ
  `local_address(0.0.0.0)` က forwarding တစ်ခုလုံး ပိတ်စေမယ် — ဖြေရှင်းချက်က retry ဖြစ်တယ်၊
  address family မဟုတ်ဘူး
- **Fix:** `SendError::Other` ကို **`Unreachable` နဲ့ `Rejected`** ခွဲလိုက်တယ် +
  `is_transient()`。 `post()` ရဲ့ `.send()` / `.text()` failure နဲ့ non-JSON body
  (block page — Telegram မဟုတ်တဲ့ တစ်ခုခု ဖြေတာ) နဲ့ Telegram ရဲ့ **5xx** က `Unreachable`၊
  4xx `ok:false` က `Rejected`。 `Unreachable` ဆိုရင် queue ထဲ ပြန်ထည့်ပြီး
  `RETRY_BASE` 5 s ကနေ `MAX_BACKOFF` 60 s အထိ ခုန်တက်တယ် (success မှာ reset)
- **Test:** `only_network_and_rate_limit_failures_are_retryable` (classification ကိုယ်တိုင်)၊
  `interpret_treats_a_telegram_5xx_as_retryable`၊
  `interpret_reports_a_non_json_body_with_a_bounded_preview` (block page = `Unreachable`)၊
  `interpret_surfaces_a_rejection_description` (401 = `Rejected`)
- **Rule ၁:** Error enum ရဲ့ variant က **caller ရဲ့ ဆုံးဖြတ်ချက်ကို** ကိုယ်စားပြုရမယ်၊
  message အလွှာကို မဟုတ်ဘူး။ `Other` က "ဒီ error ကို ဘာလုပ်ရမလဲ" ဆိုတာ ဖျောက်ပစ်တယ် —
  retry လုပ်လို့ ရ/မရ ဆိုတာ type ကနေ ဖတ်လို့ ရရမယ်
- **Rule ၂:** Queue တစ်ခု ရေးတဲ့အခါ **transport failure ကို drop path မှာ ထည့်မိလား** စစ်ပါ။
  Queue ရှိတာက အဲဒီ failure အတွက် ဖြစ်တယ်
- **Rule ၃:** "ခဏ ပြတ်တယ်" ကို ခန့်မှန်းမနေဘဲ **တကယ် စစ်ပါ** (`curl -4` / `curl -6` /
  `getent ahostsv4|v6`)。 ဒီ case မှာ IPv6 route ပျက်နေတာ တွေ့တယ် — ဒါပေမယ့် တွေ့ပြီးမှ
  fallback က ကိုင်နေတာလည် သက်သေရလို့ **connector ကို မထိတာ** ဆုံးဖြတ်နိုင်ခဲ့တယ်

## ⚠️ Latent Traps (မဖြစ်သေးဘူး၊ ဒါပေမဲ့ ချောင်းနေတာ)

Case မဟုတ်သေးပါ — 2026-08-30 settings cleanup (`fbd7b8b`) လုပ်ရင်း တွေ့ခဲ့တဲ့ ချောင်း ၃ ခု
(T1–T2 settings layer၊ T3 decoder)၊ ပြီးတော့ 2026-08-31 audit ကနေ T4 (retention layer) နဲ့
T5 (busy-flag layer)။
Symptom မရှိသေးလို့ fix မလုပ်ခဲ့ဘူး၊ ဒါပေမဲ့ နောက်ဆို ရှင်းပြရ ခက်တဲ့ bug ဖြစ်လာနိုင်တယ်။
**T3 က v1.4.0 မှာ၊ T4/T5 က v1.5.0 မှာ ကုဒ် ပြင်ပြီးသွားပြီ** — T1/T2 က ပြင်ရသေး။

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

### T4. `retentionHours` က Rust ကို unclamped ရောက်ပြီး panic ဖြစ်တာ (retention layer — **v1.5.0 မှာ ပြင်ပြီး**)

**Symptom (field မှာ မဖြစ်သေးဘူး — test နဲ့ reproduce လုပ်ခဲ့တာ):** `retentionHours` က
`1e13` ဝန်းကျင် ဖြစ်ရင် `purge_expired_messages` / `cleanup_sim_storage` က panic တယ်။
ပိုဆိုးတာက `start_live` — `live.rs:265`/`:361` မှာ port တစ်ခုချင်းစီ panic ဖြစ်ပြီး
`catch_unwind` က `Closed { error: "Worker crashed" }` ပြောင်းပေးတာမို့ **live mode က
port အားလုံးမှာ ရှင်းလင်းချက်မရှိဘဲ ရပ်သွားမယ်**။ Process က မသေဘူး (`panic = "abort"` မထားဘူး)၊
feature ပဲ သေတယ်

- **Root Cause ၂ ဆင့်:**
  1. `retention_from_hours` က `!h.is_finite()` နဲ့ `h <= 0.0` ကို ကာထားပေမယ့် **အပေါ်ဘက်
     မကာထားဘူး**။ `Duration::from_secs_f64` က overflow မှာ panic တယ် (`h` ≳ 5.1e15)
  2. အဲ့ဒါ လွတ်လာရင် `models::retention_cutoff_ms` မှာ `chrono::Duration::seconds(i64::MAX)`
     က "out of bounds" panic တယ် (`i64::MAX / 1000` စက္ကန့် ကျော်ရင်)
- **ဘယ်လို ရောက်လာလဲ:** Settings UI က fixed-option `select` ဖြစ်တာမို့ UI ကနေ မရောက်ဘူး။
  ဒါပေမယ့် store က `localStorage` ကနေ rehydrate ဖြစ်တယ် — profile အို၊ လက်ပြင်၊ corrupt entry
  ကနေ ရောက်တယ်။ `api.ts` က `<= 0` ပဲ စစ်တယ်။ App ရဲ့ purge timer က **၆၀ စက္ကန့်တစ်ခါ**
  ပြန်ခေါ်တာမို့ ဖြစ်ရင် ထပ်ခါထပ်ခါ ဖြစ်မယ်
- **Fix (v1.5.0):** `MAX_RETENTION_HOURS = 87_600.0` (၁၀ နှစ်) — အဲ့ဒါ ကျော်ရင် `None` ပြန်တယ်။
  ၁၀ နှစ်ကျော် retention က "keep everything" နဲ့ **semantically အတူတူ** ဖြစ်တာမို့ ရှိပြီးသား
  "0 = off" precedent နဲ့ တစ်ထပ်တည်း ကျတယ် — error ပြန်တာထက် ပိုကောင်းတယ်၊ ဘာလို့လဲဆိုတော့
  မှားတဲ့ value ဟာ exception မဟုတ်ဘဲ **သာမာန်** input ဖြစ်တာမို့။ `Duration::try_from_secs_f64`
  သုံးတယ်။ `retention_cutoff_ms` က backstop အနေနဲ့ `try_seconds` + `checked_sub_signed` နဲ့
  `i64::MIN` ကို saturate တယ် — "ဘာမှ မအိုသေးဘူး" ဆိုတဲ့ လုံခြုံတဲ့ အဖြေ
- **Test:** `an_absurd_retention_window_is_off_not_a_panic` (commands),
  `an_absurd_retention_window_saturates_instead_of_panicking` (models)
- **Rule ၁:** Numeric setting ကို ကာတဲ့အခါ **အောက်ဘက် တစ်ဖက်တည်း မကာပါ**။ `<= 0` စစ်ပြီးရင်
  "ကာပြီး" လို့ ထင်လွယ်တယ် — ဒါပေမယ့် panic က အပေါ်ဘက်မှာ ရှိတယ်
- **Rule ၂:** `localStorage` ကနေ လာတဲ့ value တိုင်းကို **Rust မှာ** clamp/reject ပါ။ Frontend က
  validator မဟုတ်ဘူး: Settings ရဲ့ number input က `min`/`max` ကို DOM element ပေါ်ပဲ ထားတယ်၊
  `onchange` က `parseInt` ရလာတာကို unclamped သိမ်းတယ်
- **Rule ၃:** Worker thread ထဲက panic ကို `catch_unwind` နဲ့ ဖမ်းထားတာက **root cause ကို
  ဖုံးတယ်**။ `Worker crashed` ဆိုတဲ့ message က operator အတွက် ဘာမှ မဖြေရှင်းပေးဘူး —
  panic ဖြစ်နိုင်တဲ့ input ကို worker ထဲ မရောက်ခင် ဖယ်ပါ

### T5. Busy flag က panic path မှာ မရှင်းတာ — restart မလုပ်မချင်း "Busy" (busy-flag layer — **v1.5.0 မှာ ပြင်ပြီး**)

**Symptom (field မှာ မဖြစ်သေးဘူး):** operation တစ်ခု panic ဖြစ်ရင် `port_busy()` က ထာဝရ
`true` ကျန်တယ် → Scan / Live / Get SIM / Delete / Cleanup / Detect **အားလုံး** `Busy` ပြန်တယ်၊
app restart မလုပ်မချင်း ပြန်မကောင်းဘူး။ Log မှာ panic တစ်ခုပဲ ပါမယ်၊ ဘာလို့ ပိတ်နေတာလဲ ဆိုတာ
UI မှာ ဘာမှ မပြဘူး

- **Root Cause:** busy flag ၆ ခုလုံးကို **ordinary statement** နဲ့ ရှင်းခဲ့တယ် (`:369, :506,
  :642, :1051, :1222, :1395`) — unwind မှာ မဟုတ်ဘူး။ ဆိုတော့ per-port `catch_unwind`
  အပြင်ဘက်မှာ panic ဖြစ်ရင် flag က ကျန်တယ်။ ကိစ္စ ၂ ခု အထင်ရှားဆုံး:
  1. **`delete_selected`** — `catch_unwind` က modem loop ကိုပဲ ဝိုင်းထားတယ်။ အဲ့နောက်က
     bookkeeping (`confirmed_removals`, `kept` တွက်တာ) မှာ panic ဖြစ်ရင် `delete_busy` ကျန်
  2. **`start_live`** — supervisor thread မှာ `catch_unwind` **လုံးဝ မရှိခဲ့ဘူး**၊ ပြီးတော့
     သူက `live_on` **နဲ့** `live_stop` နှစ်ခုလုံး ပိုင်တယ်။ Join နဲ့ clear အကြား panic ဖြစ်ရင်
     နှစ်ခုလုံး ကျန်တယ် — `live_stop.is_some()` က `port_busy()` ရဲ့ အစိတ်အပိုင်း ဖြစ်တာမို့
     gate က နှစ်ဆ ပိတ်တယ်
  3. Live per-port worker မှာလည် `catch_unwind` မရှိခဲ့ဘူး — panic ဖြစ်ရင် `join()` က
     တိတ်တဆိတ် စားလိုက်တယ်၊ port ကို failed လို့ မမှတ်ဘူး၊ **LIVE badge က အစိမ်း ကျန်ပြီး
     message တစ်ခုမှ မဖမ်းဘူး**
- **Fix (v1.5.0) — `BusyGuard` (Drop guard):** `{ state: SharedState, clear: fn(&mut AppStateInner) }`။
  `lock_state` က poison-recovering ဖြစ်တာမို့ panic ပြီးလည် lock ရတယ် (ဒါက guard ကို
  အလုပ်လုပ်စေတဲ့ အချက်)။ **Repo ရဲ့ ပထမဆုံး `impl Drop`** ဖြစ်တယ်
- **အရေးကြီး — happy path ကို မထိဘူး:** ရှိပြီးသား inline clear တွေ အတိုင်း ထားတယ်။
  Command တစ်ချို့က အဲ့ တူတဲ့ lock ထဲမှာ ထပ်အလုပ် လုပ်တယ် (`sim_dir.save()`, status line
  တည်တာ) — အဲ့ ordering ကို ပြောင်းတာ ဒီ guard ရဲ့ အလုပ် မဟုတ်ဘူး။ ဆိုတော့ `clear` က
  **idempotent** ဖြစ်ရမယ်၊ guard က **panic path မှာပဲ** တကယ် အလုပ်လုပ်တယ်
- **Guard ကို command ထဲမှာ တည်ပြီး closure ထဲ `move` လုပ်ပါ** — `thread::spawn` ကိုယ်တိုင်
  panic ဖြစ်ရင်တောင် closure (နဲ့ guard) က အဲ့ unwind အတွင်း drop ဖြစ်ပြီး flag ရှင်းမယ်။
  Closure **အထဲမှာ** တည်ရင် ဒီကိစ္စ လွတ်သွားမယ်
- **`AppStateInner` shape မပြောင်းဘူး** (guard က `Arc` ကိုင်တယ်) — ဒါကြောင့် struct literal
  ၃ ခု (`new_shared_state`, `idle_state`, `live_state`) နဲ့
  `port_busy_covers_every_operation_that_owns_a_port` ရဲ့ `[fn(&mut AppStateInner); 6]`
  array က မပြင်ဘဲ compile ဖြစ်တယ်။ **ရည်ရွယ်ချက်ရှိရှိ ဒီဇိုင်း**
- **Live worker panic ကို မြင်စေတယ်:** `live::WORKER_PANIC` ("Live worker crashed — see the
  log") ကို `live_error` အဖြစ် တင်ပြီး `live_failed` ထဲ ထည့်၊ `ports:updated` emit တယ်။
  **`modem::NOT_RESPONDING` နဲ့ ရောမထားပါ** — အဲ့ဒါ တစ်ခုတည်းသာ `alive = Some(false)`
  သတ်မှတ်ခွင့် ရှိတယ်၊ worker crash က slot ထဲ SIM ရှိလား မရှိလား ဘာမှ မပြောဘူး
- **Test ၄ ခု:** `busy_guard_releases_the_gate_on_an_unwind`,
  `..._on_a_normal_return`, `busy_guard_is_a_no_op_once_the_flag_is_already_clear`
  (guard က တခြား operation ရဲ့ flag ကို မထိရ), `busy_guard_releases_both_live_flags`
- **Rule ၁:** **"exit path တိုင်းမှာ ရှင်းတယ်" ဆိုတာ နေရာ ၅-၆ ခုမှာ မှတ်ထားရတာ မဟုတ်ဘဲ
  structural ဖြစ်ရမယ်။** Flag ကို ပိုင်တဲ့ type တစ်ခု ထားလိုက်ရင် "မေ့သွားတာ" က compile
  ဖြစ်နိုင်တဲ့ အခြေအနေ မဟုတ်တော့ဘူး
- **Rule ၂:** `catch_unwind` ရဲ့ **အတိုင်းအတာ** ကို စစ်ပါ။ Loop ကိုပဲ ဝိုင်းထားတာက loop
  အပြင်က bookkeeping ကို မကာဘူး — `delete_selected` က အဲ့ဒီ ကိစ္စ
- **Rule ၃:** **Scope မဟုတ်တာကို scope မဟုတ်ဘူးလို့ ရေးထားပါ။** Supervisor ၄ ခုကို
  `run_port_pool` အဖြစ် ပေါင်းတာ ဒီ change ထဲ မပါဘူး: panic policy **၄ မျိုး မတူ**တယ်
  (scan: `failed_notes` push + `done` တိုး · ussd: `done` ပဲ တိုး, ဆိုတော့ panic ဖြစ်တဲ့ port က
  "မတွေ့ဘူး" လို့ ဖတ်တယ် · cleanup: failure counter တိုး · detect: **v1.5.0 (#19) ကတည်းက
  `ProbeVerdict::Inconclusive` — `alive`/`checked`/`iccid`/`sim_dir` မထိဘူး**; အရင်က dead လို့
  သတ်မှတ်ပြီး `sim_dir` slot ရှင်းခဲ့တာ ဒီ case §16 ကိုယ်တိုင်)။ ပေါင်းရင် policy တစ်ခု ရွေးရမယ်
  ဒါမှမဟုတ် per-port on-panic callback ထည့်ရမယ် — ဒါက behaviour-normalising refactor ဖြစ်တာမို့
  သီးသန့် change။ **တမင် ရွှေ့ထားတာ၊ entry က doc 05 §C.10**

## Bonus UX Notes

- **User-reported 404 on releases page** but server-side probes said HTTP 200 (public repo) →
  browser cache/transient CDN — hard refresh/incognito/direct tag URL နဲ့ cross-check လုပ်ဘိုး။
- **`gh` CLI:** `export GH_PAGER=cat PAGER=cat` မလုပ်ရင် interactive pager block ဖြစ်နိုင် (non-interactive shell trap);
  long polls ကို background process + `/tmp/*.txt` capture pattern သုံး။
