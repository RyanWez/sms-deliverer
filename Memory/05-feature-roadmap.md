# 🚀 Planned Features & Roadmap (အနာဂတ် လုပ်ဆောင်ချက်များ)

> **Project:** SIM Bank SMS Reader (`sms-tauri`) · Repo: `RyanWez/sms-deliverer`  
> **Status:** Backlog / Future Enhancements (အနာဂတ်တွင် ဆက်လက် အကောင်အထည်ဖော်မည့် Features စာရင်း)
>
> ဒီဖိုင် ၂ ပိုင်း ပါတယ်: (၁) အောက်က **Feature Backlog** — မလုပ်ရသေးတဲ့ အကြံအစည်များ၊
> (၂) **§Settings Controls — Decisions Ledger** — ဆုံးဖြတ်ပြီးသား keep/delete/defer/refuse များ။
> Ledger ထဲက ဟာကို **ပြန်မဆွေးနွေးပါနဲ့**၊ အထူးသဖြင့် §B hard refusal ၂ ခု။

---

## 📋 Feature Backlog List

### ၁။ Telegram Bot & HTTP Webhook SMS Forwarding *(High Priority / High Value)*
* **ဖော်ပြချက်:** SIM Bank မှ SMS သို့မဟုတ် OTP အသစ် လက်ခံရရှိချိန်တိုင်း ကွန်ပျူတာရှေ့တွင် မရှိရင်တောင် **Telegram Bot**၊ **Discord Channel** သို့မဟုတ် **Custom REST API Endpoint (Webhook)** သို့ အလိုအလျောက် ချက်ချင်း Forward ပို့ပေးသည့် စနစ်။
* **အဓိက ပါဝင်မည့် အချက်များ:**
  * Settings ထဲတွင် `Forwarding` tab တစ်ခု ထည့်သွင်းပေးခြင်း။
  * Telegram Bot Token, Chat ID, Thread ID သတ်မှတ်နိုင်ခြင်း။
  * Custom HTTP Webhook URL (POST JSON payload) နှင့် Header Authorization ထည့်သွင်းနိုင်ခြင်း။
  * `Forward OTP Only` သို့မဟုတ် `Forward All Messages` စိတ်ကြိုက် ရွေးချယ်နိုင်ခြင်း။
  * Retry mechanism (Network ကျနေပါက ၃ ကြိမ်အထိ အလိုအလျောက် ပြန်ပို့ပေးခြင်း)။

---

### ၂။ Interactive AT Command Console & Signal Strength Indicator
* **ဖော်ပြချက်:** Port တစ်ခုချင်းစီ၏ Detail Drawer ထဲတွင် သက်ဆိုင်ရာ GSM Modem ဆီသို့ တိုက်ရိုက် AT Command ပို့ပြီး စမ်းသပ်နိုင်သည့် Terminal Console နှင့် လှိုင်းအား (Signal Quality) ပြသပေးသည့် စနစ်။
* **အဓိက ပါဝင်မည့် အချက်များ:**
  * Port Detail modal တွင် **Interactive AT Console** ထည့်သွင်းပေးခြင်း (ဥပမာ: `AT+CSQ` လှိုင်းအားစစ်ရန်, `AT+CUSD=1,"*124#",15` ငွေလက်ကျန်စစ်ရန်, `AT+CPIN?` SIM PIN စစ်ရန်)။
  * SIM ကတ် တစ်ခုချင်းစီ၏ လှိုင်းအား (Signal Strength - RSSI / dBm) ကို Port Card ပေါ်တွင် အစိမ်းရောင် ဘားလေးများ (Signal Bars) ဖြင့် ပြသပေးခြင်း။
  * Quick AT Command Presets (မကြာခဏ သုံးလေ့ရှိသော Command များကို တစ်ချက်နှိပ်ရုံဖြင့် ပို့နိုင်ခြင်း)။

---

### ၃။ Message Batch Actions & Date-Range Filtering
* **ဖော်ပြချက်:** Inbox ထဲရှိ SMS များကို အများအပြား ရွေးချယ်၍ တစ်ပြိုင်နက် လုပ်ဆောင်နိုင်ခြင်းနှင့် ရက်စွဲအလိုက် ရှာဖွေနိုင်ခြင်း။
* **အဓိက ပါဝင်မည့် အချက်များ:**
  * Message Table တွင် Multi-select Checkboxes ထည့်သွင်းပေးခြင်း။
  * **Batch Actions:** Select All, Batch Delete Selected, Batch Copy Selected Text, Export Selected Only။
  * **Date-Range Filter:** "ယနေ့", "ယမန်နေ့", "ပြီးခဲ့သည့် ၇ ရက်", "ယခုလ", "Custom Date Range (From - To)" ဖြင့် အလွယ်တကူ Filter ပြုလုပ်နိုင်ခြင်း။

---

### ၄။ Modern Notification Chimes & Audio Themes
* **ဖော်ပြချက်:** SMS သို့မဟုတ် OTP အသစ် ဝင်ရောက်လာချိန်တွင် နားဝင်ချိုပြီး ခေတ်မီသော Notification Chime Sound (Pop/Bell/Chime) အသံ ထွက်ပေါ်စေခြင်း။
* **အဓိက ပါဝင်မည့် အချက်များ:**
  * Settings -> Notifications တွင် Notification Sound ဖွင့်/ပိတ်နှင့် Volume Slider ထည့်သွင်းပေးခြင်း။
  * Sound Presets ရွေးချယ်နိုင်ခြင်း (e.g. `Modern Pop`, `Subtle Bell`, `Crisp Chime`, `Muted Click`)။
  * OTP ဝင်ချိန်တွင် သာမန် SMS ထက် ပိုမိုထင်ရှားသော အထူး အသံသီးသန့် ထွက်ပေါ်စေခြင်း။

---

### ၅။ Quick Stats & Analytics Dashboard Overview
* **ဖော်ပြချက်:** SIM Bank တစ်ခုလုံး၏ နေ့စဉ် လုပ်ငန်းဆောင်ရွက်မှု အခြေအနေများကို ဇယားနှင့် ဂရပ်များဖြင့် ခြုံငုံသုံးသပ်နိုင်သည့် Dashboard။
* **အဓိက ပါဝင်မည့် အချက်များ:**
  * တစ်နေ့တာ လက်ခံရရှိသော SMS စုစုပေါင်းနှင့် ယမန်နေ့နှင့် နှိုင်းယှဉ်ချက် (Growth Rate)။
  * Port အလိုက် အများဆုံး SMS လက်ခံရရှိသည့် Top 5 Active SIMs / Ports။
  * Error တက်နေသော Port များ (Dead / Timeout Modems) အကျဉ်းချုပ်။
  * OTP Detection Success Rate (%)။

---

# ⚖️ Settings Controls — Decisions Ledger (2026-08-30)

> Commits: `fbd7b8b` (field ၁၁ ခု ဖျက်) · `f88d6d0` (`notifications.enabled` wire) · `10e0058` (Developer Mode ပိတ်ရင် Logs page ကနေ ထွက်)
> **ဒီ ledger ရဲ့ ရည်ရွယ်ချက်:** အောက်ပါ ဆုံးဖြတ်ချက်တွေကို နောက် session မှာ **ပြန်မဆွေးနွေးရ**။
> Rule ကိုယ်တိုင်က doc 04 §H — *inert switch ကို လုံးဝ မထည့်ရ*။

## A. ဖျက်ပစ်လိုက်တဲ့ field ၁၁ ခု

`otp` group တစ်ခုလုံး + သူ့ `setOtp` setter ပါ ပျောက်တယ်။ `otpPattern` က `type: "text"` တစ်ခုတည်း
ဖြစ်ခဲ့လို့ Settings page ရဲ့ shared text-input branch ပါ လိုက်ဖျက်ခဲ့တယ်။

| Field | အရင် label | ဘာလို့ ဖျက်လိုက်တာလဲ |
|---|---|---|
| `general.minimizeToTray` | Minimize to System Tray | Tray icon/window-hide code လုံးဝ မရှိ — inert |
| `notifications.soundEnabled` | Play Sound | audio pipeline မရှိ။ Feature အဖြစ် backlog §၄ မှာ ရှိပြီး |
| `notifications.desktopNotifications` | Desktop Notifications | Native notification path မရှိ — §E feasibility ကြည့် |
| `notifications.otpOnlyNotifications` | OTP Messages Only | Toast က OTP အတွက်ပဲ ထွက်တာမို့ semantics ကိုယ်တိုင် အလွဲ |
| `otp.autoCopy` | Auto-copy OTP to Clipboard | Feature မရှိ — §D (dropped as a switch) |
| `otp.showInTable` | Show OTP Column | OTP column က `MessageTable.svelte` မှာ condition မရှိဘဲ အမြဲ ပြတယ် |
| `otp.highlightNewOtp` | Highlight New OTP | Highlight က `item.is_new` ပေါ် တည်တယ် (`new-msg-highlight`)၊ setting ကို မဖတ် |
| **`otp.otpPattern`** | OTP Detection Pattern (Regex) | **Hard refusal — §B.1** |
| `appearance.compactMode` | Compact Mode | CSS surface မရှိ — §D (dropped) |
| **`developer.logLevel`** | Capture Log Level | **Hard refusal — §B.2** |
| `developer.maxLogs` | *(label မရှိ)* | UI မှာ render မဖြစ်ခဲ့ဘူး; ring buffer က Rust ဘက် hardcode (`MAX_RING_BUFFER = 1000`) |

ကျန်ခဲ့တဲ့ inert field ၂ ခု (`general.portRefreshInterval`, `developer.autoScroll`) က
**တမင် ချန်ထားတာ** — §C ကြည့်။ (`portRefreshInterval` က **v1.4.0 မှာ wire ပြီးသွားပြီ** —
inert မဟုတ်တော့ဘူး၊ §C.2။ `autoScroll` ကတော့ inert အတိုင်း ကျန်နေတယ်။)

## B. Hard Refusals — ဒီ ၂ ခုကို editable field အဖြစ် **ပြန်မထည့်ရ**

ဒါတွေက "အချိန်မရလို့ ရွှေ့ထားတာ" မဟုတ်ဘူး၊ **ငြင်းပယ်ထားတာ**။ ဖျက်ရတဲ့ အကြောင်းရင်းက
implementation cost မဟုတ်ဘူး — control ကိုယ်တိုင်က တကယ် အလုပ်လုပ်လိုက်ရင် ပိုဆိုးတာ။

### B.1 `otp.otpPattern` — operator-editable OTP regex

OTP detection က `src-tauri/src/core/decoder.rs::extract_otp` — regex တစ်ခုတည်း မဟုတ်ဘူး၊
**keyword gate + ordered cascade ၄ ဆင့်** (အားလုံး `LazyLock<Regex>` statics):

1. `normalize_myanmar_digits()` — မြန်မာ digit (`U+1040`–`U+1049`) → ASCII
2. `KEYWORD_RE` **gate** — `otp|one.?time|code|pin|verification|verify|confirm` + မြန်မာ keyword
   constants (`KW_KODE` = ကုဒ်၊ `KW_CONFIRM`၊ `KW_SECURE` = လုံခြုံ)။ **မ match ရင် ချက်ချင်း `None`**
   *(`KW_CONFIRM` စာလုံးပေါင်း အမှား ရှိခဲ့တယ် — v1.4.0 မှာ ပြင်ပြီး၊ doc 03 §T3)*
3. `P1` (keyword ၂၄ char အတွင်း digit 4–8) → `P2` (digit + `is|as your|`ဖြစ်) → `P3` (bare 6-digit)
   → `P4` (bare 4–8 digit) — ဒီ **order** ကိုယ်တိုင် precision ကို ထိန်းတာ

UI ရဲ့ placeholder ကိုယ်တိုင် `\b(\d{4,8})\b` — ဒါက `P4` ချည်းသက်သက်၊ **gate မပါ**။ Operator က
တစ်ခုခု ရိုးရိုး ရိုက်ထည့်လိုက်ရင် keyword gate ပျောက်တယ်၊ ပြီးတော့ promotional SMS ရဲ့ ငွေလက်ကျန်၊
ရက်စွဲ၊ ဖုန်းနံပါတ် အပိုင်းအစ တွေ OTP အဖြစ် match လာတယ်။

**ဆိုးဆုံးအချက်: silent failure ဖြစ်တယ်။** ဘာမှ error မတက်၊ UI က ကျန်းမာနေတယ်၊ ဒါပေမဲ့
**မှားတဲ့ နံပါတ်** က clipboard ထဲ ရောက်တယ် — operator က ကိုယ်ဖျက်ထားတဲ့ ဟာမှန်း မသိဘူး။

> Transparency လိုချင်ရင်: active pattern တွေကို **read-only** ပြပါ (edit မလုပ်နိုင်တဲ့ list)၊
> input field မလုပ်ပါနဲ့။

### B.2 `developer.logLevel` — capture-level switch

Masking က **sink မှာ မဟုတ်ဘူး** — `src-tauri/src/logging.rs` ရဲ့ `mask_number()` / `otp_summary()`
ကို Info-level line **တစ်ခုချင်းစီ** မှာ ကိုယ်တိုင် ခေါ်ထားတာ။ Capture gate က hardcoded Info:
`capture_entry` (`level > Level::Info` → drop)၊ `Log::enabled` impl ၂ ခု (ring buffer + file)၊
`set_max_level(LevelFilter::Info)`။

အဲ့ gate အောက်မှာ **mask မထားတဲ့** debug line တွေ ရှိတယ်:

| နေရာ | ဘာ ပါလဲ |
|---|---|
| `core/at.rs` `>> {cmd}` | AT command အားလုံး |
| `core/at.rs` `<< {preview(&text, 160)}` | reply တိုင်းရဲ့ ၁၆၀ char preview — `AT+CMGL=4`/`AT+CMGR` အတွက် **raw PDU hex**: sender MSISDN + message body (OTP အပါ)၊ `AT+CUSD` အတွက် subscriber ကိုယ်ပိုင် နံပါတ် |
| `core/at.rs` `++ {preview(&line, 120)}` | unsolicited notification line |
| `core/modem.rs` USSD reply body | parse မရတဲ့ USSD reply body — **တမင် `debug!` ကို ရွှေ့ထားတာ** (comment ကိုယ်တိုင် "debug never reaches a sink" လို့ ဆိုထား) |

Gate ကို နှိမ့်လိုက်ရင် ဒါတွေ အားလုံး (၁) Logs page မှာ အတိအကျ ပြတဲ့ 1000-entry ring buffer၊
(၂) `app.log` — **5 MB ပြည့်မှ rotate၊ အသက်အရွယ်အလိုက် ဘယ်တော့မှ မဖျက်** ဆိုတော့ inbox retention
window (default ၂ နာရီ) ထက် ပိုအသက်ရှည်တဲ့ ဖိုင် — ၂ ခုထဲ ရေးကုန်တယ်။ Privacy masking work
တစ်ခုလုံး switch တစ်ချက်နဲ့ ပြုတ်တာ။

> Debug mode တကယ် လိုလာရင်: **sink မှာ redact လုပ်ရမယ်** (capture\_entry ထဲ PDU/number scrubber)၊
> ရှိပြီးသား debug line တွေကို ဖွင့်ပေးတာ မဟုတ်ဘူး။

## C. Deferred — သဘောတူထားတဲ့ **အစဉ်လိုက်** (next session ဒီ order နဲ့ ဆက်ပါ)

> **v1.5.0 အခြေအနေ:** C.1 (theme) နဲ့ C.2 (port auto-refresh) က **v1.4.0 မှာ ပြီးသွားပြီ**၊
> C.8 (backend outcome တွေ operator ဆီ ရောက်အောင်) နဲ့ C.9 (toast cap + coalesce) က
> **v1.5.0 မှာ ပြီးသွားပြီ** — ဆုံးဖြတ်ရသေးတဲ့ setting က **C.3 တစ်ခုပဲ** ကျန်တယ်။
> ပြီးသွားတဲ့ဟာတွေကို history အဖြစ် ချန်ထားတယ် (ဖျက်လိုက်ရင် blocker တွေက ဘာလို့ blocker
> ဖြစ်ခဲ့တာလဲ ဆိုတာ ပျောက်သွားမယ်)။
> **C.4–C.7 (L1–L4) က v1.5.0 မှာလည် ပွင့်နေတဲ့ limitation အတိုင်း** — v1.5.0 ကုဒ်ပေါ်
> ပြန်စစ်ပြီး၊ ဒါပေမဲ့ သတိထားရမှာ ၂ ချက်:
> (၁) အဲ့ entry တွေရဲ့ file:line သက်သေက **v1.4.0 ကုဒ်ပေါ် မှတ်ခဲ့တာ** — #17–#20 က
> `src-tauri/src/commands/mod.rs` ကို ရှည်စေတာမို့ လိုင်းနံပါတ်တွေ ရွှေ့သွားပြီ
> (`merge_ports` → `:187`၊ `live_status` → `:532`၊ `start_live` ရဲ့ per-port spawn → `:1007`၊
> `Closed` arm → `:1184`) — item ကို **နာမည်နဲ့** ရှာပါ၊ နံပါတ်နဲ့ မရှာနဲ့။
> (၂) အကြောင်းအရာ တကယ် ပြောင်းသွားတာ **C.4 တစ်ခုပဲ**: #19 ကနေ `live_error` က refresh အတွင်း
> tty name တူမှ carry ဖြစ်သွားပြီ (`mod.rs:236`) ဆိုတော့ "error text ပျောက်တယ်" အပိုင်း ပိတ်ပြီ။
> ဒါပေမဲ့ `live_ready` က name ပြောင်းရင် carry မလုပ်တာ (`:221`) နဲ့ `Reconnecting` arm က
> row ကို **name နဲ့ ရှာနေတာ** (`:1067`၊ lookup `:1072`) ကျန်နေတာမို့ **`CONNECTING…` ထာဝရ
> ကျန်တဲ့ symptom က မပြေဘူး**။
> **v1.5.0 ရဲ့ ကျန် PR (#12/#13/#15/#18/#19/#20) က backlog feature မဟုတ်ဘူး၊ bug fix တွေ** —
> ဒါကြောင့် သူတို့ အသေးစိတ်က ဒီဖိုင်မှာ မဟုတ်ဘဲ **doc 03** မှာ ရှိတယ် (case §14–§17၊ trap T4/T5)။

### C.1 ✅ **DONE (v1.4.0):** Theme Dark/Light တကယ် အလုပ်လုပ်အောင် လုပ်ပြီး

Control က Settings ပေါ် ရှိခဲ့တယ် (System / Dark / Light)၊ **Light က no-op ဖြစ်ခဲ့တယ်**။
တစ်ခုတည်း မဟုတ်ဘူး — **သီးသန့် blocker ၄ ခု** ရှိခဲ့တယ် (ledger မှာ ၃ ခု လို့ မှတ်ခဲ့တာ
**မပြည့်စုံခဲ့ဘူး** — Logs console ရဲ့ hardcoded hex က စတုတ္ထ blocker၊ အောက်တွင်)၊
၄ ခုလုံး ဖြေပြီးမှ theme လာတယ်:

| # | Blocker | အရင် အခြေအနေ | v1.4.0 ဖြေရှင်းချက် |
|---|---|---|---|
| 1 | **Class strategy မပြည့်စုံ** | `applyTheme()` (`src/lib/stores/settings.svelte.ts`) က `dark` class ကို add/remove ပဲ လုပ်တယ် — `light` class ဆိုတာ မရှိ၊ ပြီးတော့ `src/` တစ်ခုလုံးမှာ **`dark:` Tailwind variant တစ်ခုမှ မသုံးထားဘူး** (grep → zero)။ Component တွေက `bg-surface` စတဲ့ CSS-variable token တွေပဲ သုံးတယ် | `setResolved()` (`settings.svelte.ts:154`) က `dark`/`light` ၂ ခုလုံးကို `classList.toggle` နဲ့ explicit ရေးတယ် + `root.style.colorScheme` ပါ set တယ် (native scrollbar / `<select>` popup အတွက်) |
| 2 | **Light token တွေ OS ကို ချုပ်ခံထား** | `src/app.css` ရဲ့ light palette က `@media (prefers-color-scheme: light) { :root:not(.dark) { … } }` အောက်။ OS က dark ဖြစ်တဲ့ machine မှာ Light ရွေးရင် — `dark` class ဖြုတ်ပေမဲ့ media query က မ match တာမို့ — `:root` ရဲ့ dark token တွေ ကျန်နေတယ်၊ **ဘာမှ မပြောင်း** | `src/app.css:21` `:root, :root.dark` နဲ့ `src/app.css:48` `:root.light` ၂ ခုလုံး **variable ၂၀ လုံး အပြည့်** သတ်မှတ်တယ် (partial override မရှိ)၊ palette က **class ပေါ်ပဲ** မှီတယ်။ `prefers-color-scheme` က app.css ထဲမှာ comment အဖြစ်ပဲ ကျန်တယ် — ဘယ် class တင်မလဲ ဆုံးဖြတ်တဲ့အခါ ၂ နေရာ (flash guard + `applyTheme`) မှာပဲ ဖတ်တယ် |
| 3 | **Shell က dark ကို pin ထား** | `index.html`: `<html class="dark" style="…color-scheme: dark">` + inline `<style>` မှာ `html, body, #app { background-color: #171717 !important }`။ `!important` ကြောင့် app background ကို token နဲ့ override မရဘူး | `index.html` ရဲ့ flash guard က **synchronous inline IIFE** ဖြစ်သွားပြီ — stylesheet/bundle မတင်ခင် `localStorage` `sms-reader-settings` ကနေ theme ဖတ်၊ `dark`/`light` class + `colorScheme` ကို တင်၊ corrupt JSON / storage ပိတ်ထားရင် `<html>` ပေါ် ကြေညာပြီးသား dark default ကို ချန်။ Pre-stylesheet paint colour ကို `html` / `html.light` ၂ ခုနဲ့ ရေးတယ်၊ **`!important` ပါ ဖြုတ်လိုက်ပြီ** |
| 4 | **Logs console က hex hardcode** *(ledger မှာ "Bonus trap" လို့ မှတ်ခဲ့တာ — တကယ်က blocker)* | `src/lib/pages/Logs.svelte` က `bg-[#0d1117] text-[#e6edf3]` — `src/` တစ်ခုလုံးမှာ **hardcoded hex တစ်ခုတည်း**။ အထဲက log line တွေက token colour သုံးတာမို့ light theme လာရင် **အလင်းပေါ်အလင်း — လုံးဝ မဖတ်နိုင်** | Console အတွက် သီးသန့် token ၃ ခု ထည့်ပြီး (`--console-bg` / `--console-fg` / `--console-row-hover`၊ theme ၂ ခုလုံးမှာ ရှိ)၊ `Logs.svelte:223` က `bg-[rgb(var(--console-bg))] text-[rgb(var(--console-fg))]`၊ row hover က `:242` မှာ `--console-row-hover`။ Console က တမင် `--surface` ကို မလိုက်ဘူး (terminal surface — GitHub canvas pair) |

> ⚠️ **`index.html` ရဲ့ flash guard ကို ဘယ်တော့မှ မဖျက်ပါနဲ့** — Vite က CSS ကို inject မလုပ်ခင်
> webview က `<html>` default ကို ခဏ ပြတယ်၊ အဲ့ဒါက အလင်းပြက်ခြင်း အဖြစ် မြင်ရတယ်။ v1.4.0 မှာ
> **အစားထိုးလိုက်တာ၊ ဖျက်လိုက်တာ မဟုတ်ဘူး** — အခု guard က persist ထားတဲ့ theme အလိုက်
> class ရွေးပေးတယ်။ ဒါက `settings.svelte.ts` ရဲ့ resolution logic (storage key၊ JSON shape၊
> class နာမည်၊ `color-scheme`) ကို **တမင် duplicate ထားတာ** — bundle မတင်ခင် run ရတာမို့။
> တစ်ဖက် ပြောင်းရင် နောက်တစ်ဖက် လိုက်ပြောင်းပါ (`index.html` ရဲ့ comment ကိုယ်တိုင် ဒါကို ဆိုထား)။

**မှတ်ထားရမယ့် အချက် ၂ ခု:**
- **OS media listener က အရင်ကတည်းက ရှိခဲ့တယ် — ဒါပေမဲ့ ဘယ်တော့မှ unsubscribe မလုပ်ခဲ့ဘူး**။
  v1.3.1 မှာ startup တစ်ခါ `addEventListener('change', …)` တင်ထားပြီး `removeEventListener`
  မရှိ၊ callback ထဲမှာ `theme === 'system'` လား စစ်တာနဲ့ ကာခဲ့တာ။ အခု listener က
  **System ရွေးထားစဉ်မှာပဲ တင်ရှိတယ်** — `detachSystemListener()` (`settings.svelte.ts:136`)
  က attach တိုင်း အရင် ဖြုတ်တာမို့ `applyTheme` က idempotent ဖြစ်တယ် (listener မထပ်နိုင်)၊
  ပြီးတော့ Dark/Light pin ထားတဲ့ user ကို OS ညနေ dark ပြောင်းလိုက်တာနဲ့ ပြန်မဆွဲတော့ဘူး
- `matchMedia` မရှိတဲ့ embedded webview မှာ **dark ကို fallback** (`settings.svelte.ts:173`) —
  OS-less light branch ကို မဟုတ်ဘူး၊ shipped default က dark ဖြစ်လို့

### C.2 ✅ **DONE (v1.4.0):** `general.portRefreshInterval` — wire လုပ်ပြီး၊ `live_ready` trap ပိတ်ပြီး

**အရင်က ဒီလို ဖြစ်ခဲ့တယ် (context အတွက် ကျန်ထား):** Field က `types.ts` နဲ့ Settings page ၂ ခုလုံးမှာ
ရှိခဲ့တယ် (default `30`)၊ timer တစ်ခုမှ မဖတ်ခဲ့ဘူး — တမင် ချန်ထားခဲ့တာ။ Trap က:
`refresh_ports` (`src-tauri/src/commands/mod.rs`) က port အားလုံးအတွက် **`live_ready: false`** နဲ့
`PortInfo` အသစ် ပြန်တည်ခဲ့တယ် (`checked`/`alive`/`iccid` ကိုပဲ stable `path` နဲ့ carry over)။
ဆိုတော့ live mode ဖွင့်ထားစဉ် background timer ပြေးလိုက်ရင် **LIVE badge အားလုံး ပြုတ်သွားမယ်** —
modem တွေ ကောင်းနေတဲ့ အခိုက်မှာ (လက်နဲ့ Refresh နှိပ်တာနဲ့တောင် ဖြစ်ခဲ့တာ)။

**အခု ဖြေရှင်းပြီးသွားပြီ (v1.4.0)** — ledger မှာ "၂ ခုထဲ တစ်ခု" လို့ ရေးခဲ့ပေမဲ့ တကယ်က
**၂ ခုလုံး** လုပ်လိုက်တယ်:

| အလွှာ | ဖိုင် | ဘာ ဖြစ်သွားလဲ |
|---|---|---|
| Timer | `src/App.svelte:78` `restartPortRefresh()` + `$effect` (`:99`) | `portRefreshInterval` ပြောင်းတိုင်း arm/re-arm (timer မထပ်ဘူး၊ unmount မှာ `stopPortRefresh`)၊ `isTauri()` မဟုတ်ရင် arm မလုပ် (browser preview မှာ hotplug မရှိ)။ Tick တိုင်း **port operation တစ်ခုခု လုပ်နေရင် skip** — `portsBusy()` (live/scan/USSD/delete) **+ `liveStore.detectBusy` သီးသန့်** (`:92`၊ detect က `portsBusy()` ထဲ မပါ) |
| Clamp | `src/lib/utils/port-refresh.ts` `portRefreshPeriodMs()` | `MIN_PORT_REFRESH_SECONDS = 5` / `MAX_PORT_REFRESH_SECONDS = 3600`၊ `0` နဲ့ non-finite/negative/junk အားလုံး → `null` = **off**။ Ceiling က corrupt value က `setInterval` delay ကို 2³¹−1 ms ကျော်ခိုင်းပြီး near-zero ကို overflow ဖြစ်တာ (64 serial device ပေါ် tight loop) ကို ကာတာ။ Runes/Tauri import မပါတဲ့ plain `.ts` — `npm test` နဲ့ စမ်းနိုင်တယ် |
| Diff / UI | `port-refresh.ts` `diffPorts` / `summarizeNames` / `describePortChanges` | Diff က **device name** ပေါ် အခြေခံတယ် (index မဟုတ် — backend က port number အလိုက် sort တာမို့ အောက်က stick တစ်ခု ပေါ်လာရင် index တွေ ရွှေ့ကုန်တယ်; `path` လည်း မဟုတ် — replug မှာ tty node ပြောင်းတာကိုယ်တိုင် operator သိရမယ့် အချက်)။ Toast က ပေါ်လာ = Success၊ ပျောက် = Warning၊ ၂ ခုလုံး = Info၊ **မပြောင်းရင် တိတ်တိတ်**၊ နာမည် ၃ ခုကျော်ရင် `… and N more` |
| Merge | `src-tauri/src/commands/mod.rs:143` `merge_ports(enumerated, old, sim_dir, live_session)` | `refresh_ports` (`:194`) က pure function ခေါ်တာ ဖြစ်သွားပြီ — `/dev/serial/by-path` မလိုဘဲ rule တွေကို unit test လုပ်နိုင်တယ် (`:1675` ကနေ test ၆ ခု) |

**`live_ready` trap ကို တကယ် ဘယ်လို ပိတ်ခဲ့လဲ:** badge ဟာ "အခုချိန်မှာ ဒီ port ပေါ် worker
တစ်ခု ထိုင်နေတယ်" ဆိုတဲ့ အဓိပ္ပာယ်၊ ဒါကြောင့် refresh ကို ဖြတ်ကျန်တာ အောက်ပါ **၃ ချက်
အားလုံး** မှန်တဲ့အခါမှသာ (`mod.rs:177`):

1. **live session ကျန်နေရမယ်** — `st.live_on || st.live_stop.is_some()` (`:203`)၊
   `port_busy()` က ports-held လို့ သတ်မှတ်တဲ့ window တူတူ။ session မရှိရင် worker မရှိ
2. **port က enumeration ထဲ ကျန်နေရမယ်** — ပျောက်သွားတဲ့ port က ဒီ list ထဲ entry မရှိတာမို့
   ပြန်လာချိန်မှာ fresh `false` ကနေ စတယ်
3. **stable path အောက်က tty name မပြောင်းရဘူး** (`p.live_ready && p.name == name`) —
   live worker က **spawn ချိန် name ကို တစ်သက်လုံး** ကိုင်ထားပြီး outage ပြီးရင်လည်း အဲ့ name
   ကိုပဲ ပြန်ဖွင့်တာမို့ renumber ဖြစ်သွားတဲ့ stick က path ကျန်ပေမဲ့ worker မရှိတော့ဘူး —
   carry over လုပ်ရင် badge က လိမ်တာ ဖြစ်မယ်

State carry over က **stable `path` ပေါ်ပဲ** (name ပေါ် ဘယ်တော့မှ မဟုတ်) — name က replug မှာ
ရွှေ့တဲ့ ဟာ ဖြစ်လို့ name နဲ့ တွဲရင် stick တစ်ခုရဲ့ liveness/card ကို အခြား stick ပေါ် တင်မိမယ်။

**ICCID တူတာကို condition အဖြစ် တမင် မထည့်ဘူး:** `refresh_ports` က **port ကို ဘယ်တော့မှ
မဖွင့်ဘူး** — ဒါကြောင့် သူ report တဲ့ ICCID က old entry ကနေ ကူးလာတာ ဒါမှမဟုတ် SIM directory
ရဲ့ "ဒီ path မှာ နောက်ဆုံး တွေ့ခဲ့တာ" hint ပဲ။ ဆိုတော့ သူ ပြောင်းနိုင်တာက `None` → hint
တစ်မျိုးပဲ ဖြစ်ပြီး၊ အဲ့ဒါက **card ပြောင်းသွားတယ် ဆိုတဲ့ သက်သေ မဟုတ်ဘူး** (session အရင်က
ဟာ ဖြစ်နိုင်တယ်)။ တကယ့် swap ကို ဖမ်းတာက tty-name check ပဲ။ Test:
`mod.rs` `a_slot_hint_filling_in_an_unknown_iccid_leaves_the_badge_alone`။

`live_error` ကတော့ refresh တိုင်း `None` ဖြစ်တယ် (`mod.rs:187`) — ဒါက L1 ရဲ့ ဇစ်မြစ်၊ §C.4 ကြည့်။

### C.3 `developer.autoScroll` — inert အတိုင်း၊ **မဆုံးဖြတ်ရသေး**

Logs page မှာ session-local toggle ကိုယ်ပိုင် ရှိပြီး အလုပ် လုပ်နေတယ် (`logsStore.autoScroll`၊
default `true`၊ `src/lib/stores/logs.svelte.ts`)၊ setting ကနေ မဖတ်ဘူး။ Wire လုပ်တာ ခက်တာ မဟုတ် —
store ကို settings ကနေ **seed** လုပ်ရမယ်၊ ပြီးတော့ toggle ကို session-only ထားမလား setting ထဲ
ပြန်ရေးမလား ဆိုတာ ဆုံးဖြတ်ရမယ် (owner မဆုံးဖြတ်ရသေး)။

---

**C.4–C.7: v1.4.0 ကုဒ်ပေါ် စစ်ပြီး ရွှေ့ထားတဲ့ limitation ၄ ခု (L1–L4)** — bug မဟုတ်တဲ့
"မလုပ်ရသေးတာ" မဟုတ်ဘူး၊ **ရှိပြီးသား behaviour ရဲ့ အပေါက်** တွေ။ တစ်ခုချင်းစီမှာ file:line
သက်သေ နဲ့ operator မြင်ရမယ့် symptom ပါတယ်။

### C.4 (L1) Live mode က **နာမည် ပြောင်းပြီး ပြန်လာတဲ့ stick** ကို ဘယ်တော့မှ ပြန်မကောက်ဘူး

- **သက်သေ:** worker တစ်ခုက spawn ချိန် tty name ကို **တစ်သက်လုံး** ကိုင်ထားတယ်
  (`src-tauri/src/core/live.rs:167` — reconnect loop က `AtChannel::open(port_name)` အဲ့ name
  ကိုပဲ ပြန်ဖွင့်တာ)။ `merge_ports` က name ပြောင်းသွားရင် `live_ready` ကို carry over
  မလုပ်ဘူး (`src-tauri/src/commands/mod.rs:177`) ပြီးတော့ `live_error: None` ရေးတယ်
  (`:187`) — ဆိုတော့ `portStatus` က **CONNECTING ကို အမြဲ ကျသွားတယ်**
  (`src/lib/utils/port.ts:63`)
- **ပိုဆိုးတာ:** မိဘမရှိ ဖြစ်သွားတဲ့ worker ရဲ့ `Reconnecting` event ကို **name နဲ့ ရှာတယ်**
  (`src-tauri/src/commands/mod.rs:902` arm၊ lookup က `:907`
  `find(|p| p.name == port)`) — name အဲ့ဒါနဲ့ row မရှိတော့တာမို့ **ERROR တောင် ရေးမပြနိုင်ဘူး**
- **Symptom:** operator က stick ကို ပြန်ထိုးတယ်၊ card က `CONNECTING…` နဲ့ **ထာဝရ** ကျန်နေတယ် —
  error မရှိ၊ ERROR badge မရှိ၊ message မရှိ
- **Workaround:** live ကို **Stop → Start** (start_live က ယခု checked name တွေအလိုက်
  worker အသစ် ပြန် spawn တာမို့)
- **v1.4.0 မှာ ပိုမြင်လာတယ်:** auto-refresh က timer နဲ့ re-enumerate လုပ်တာမို့ အရင်က
  Refresh နှိပ်မှ ပေါ်တဲ့ ဒီ အခြေအနေက အခု အလိုအလျောက် ရောက်လာတယ်

### C.5 (L2) `start_live` က thread pool မရှိ၊ stagger မရှိ — checked port အရေအတွက် အတိုင်း spawn

- **သက်သေ:** `src-tauri/src/commands/mod.rs:838` `for port in ports` loop က port တစ်ခုစီအတွက်
  `thread::spawn` တစ်ခု (`:842`)၊ ports က `p.checked` filter (`:803`) ကနေ လာတာ။ semaphore /
  worker cap **မရှိ**
- **နှိုင်းယှဉ်:** တခြား port-heavy path အားလုံးက cap ကို လိုက်နာတယ် — `detect_ports`
  (`:277`၊ `MAX_CONCURRENT_PROBES = 32`)၊ `start_scan` (`:475`)၊ `get_sim_numbers` = USSD
  (`:615`)၊ `cleanup_sim_storage` (`:1351`) ၃ ခုက `MAX_CONCURRENT_PORTS = 16`
  (constant တွေ `:84` / `:90`)
- **Symptom:** 64-slot bank မှာ live start လုပ်လိုက်ရင် USB bridge တစ်ခုပေါ်
  **AT conversation ၆၄ ခု တစ်ပြိုင်နက်** ဖြစ်တယ်
- **⚠️ Benchmark မလုပ်ထားဘူး** — verify လုပ်ခဲ့တာက **concurrency shape** ပဲ (cap မရှိတာ)၊
  တကယ့် throughput/ပျက်ကွက်မှု အပေါ် သက်ရောက်မှုကို မတိုင်းထားဘူး

### C.6 (L3) `LiveEvent::Closed` က ready list ကို မရှင်းဘူး — status line က over-count ဖြစ်တယ်

- **သက်သေ:** `Closed` arm (`src-tauri/src/commands/mod.rs:1019`) က `p.live_ready = false`
  ရေးတယ်၊ `p.live_error` ကို set တယ်၊ `st.live_failed.push(...)` လုပ်တယ်၊ `status_text` ကို
  `"{port} FAILED: {e}"` လို့ ရေးတယ် — ဒါပေမဲ့ **`st.live_ports_ready.retain(...)` ကို
  မလုပ်ဘူး**၊ `live_status()` ကိုလည်း ပြန်မခေါ်ဘူး
- **နှိုင်းယှဉ်:** `Offline` arm က `st.live_ports_ready.retain(|p| p != &port)` လုပ်ပြီး
  (`:886`) `live_status(&st, port_count)` ကို ပြန်တွက်တယ် (`:890`)
- **Symptom:** worker တစ်ခု crash ပြီးရင် card က ERROR ပြပေမဲ့ နောက်တစ်ခါ status line
  ပြန်တွက်ချိန်မှာ (နောက် `Ready`/`Offline` event တစ်ခုခု) `live_ports_ready.len()` က
  ကျွတ်သွားတဲ့ port ကို ထည့်ရေတွက်နေတာမို့ **"Live x/y ready" က အများပြပြီး
  "connecting…" ရေတွက်မှုပါ လိုက်လွဲတယ်** (`live_status` က `:395`)

### C.7 (L4) Live monitoring loop အတွင်းမှာ **liveness re-probe မရှိဘူး**

- **သက်သေ:** `probe_channel` က (re)connect တစ်ခါစီမှာ **တစ်ခါပဲ** ပြေးတယ်
  (`src-tauri/src/core/live.rs:201`)။ Inner loop (`:327`–`:370`) က လုပ်တာ ၄ ခုပဲ:
  `+CMTI` queue ကို drain / `handle_cmgr`၊ ရပ်နေတဲ့ concat group ကို `flush_stale`၊
  `ch.is_dead()` စစ်တာ၊ ပြီးတော့ `SIM_SWEEP_EVERY` (600 s) retention sweep — **AT ပြန်ထုတ်ပြီး
  modem ကို ပြန်စစ်တာ မရှိဘူး**
- **Symptom:** tty node က ပွင့်နေပေမဲ့ modem က AT ကို ပြန်မဖြေတော့တဲ့ အခြေအနေမှာ
  badge က **အစိမ်း LIVE ကျန်နေပြီး message တစ်စောင်မှ မလာဘူး** — `is_dead()` က
  channel-level error ကိုပဲ ဖမ်းတာ ဖြစ်လို့ တိတ်တဆိတ် ငြိမ်သွားတာကို မဖမ်းဘူး
- **60 s `OFFLINE_RETRY` re-probe က ကူညီမပေးဘူး** (`live.rs:53`၊ `:210`) — သူက
  **probe က ကျရှုံးပြီးသား branch** အတွက်ပဲ (Offline latch ဝင်ပြီးသား port)၊ Ready
  ဖြစ်သွားပြီးသား port အတွက် မဟုတ်ဘူး

### C.8 ✅ **DONE (v1.5.0):** Backend outcome တွေ operator ဆီ ရောက်အောင် — event contract ရဲ့ ကျိုးနေတဲ့ အပိုင်း

**ပြဿနာ:** Rust က event ၁၈ မျိုး emit တယ်၊ frontend က ၁၆ ခုပဲ နားစွင့်တယ် —
`export:saved` နဲ့ `sim_cleanup:done` က listener **လုံးဝ မရှိ**။ ပြီးတော့ နားစွင့်ပေမယ့်
**အဖြေကို ဖုံးထားတဲ့** ကိစ္စ ၂ ခု ရှိတယ်: `delete:done` က payload ဗလာ ဖြစ်ခဲ့တယ်၊
`live:reconnecting` က `console.warn` ပဲ ဖြစ်ခဲ့တယ်။ ဆိုတော့:

| ဖြစ်ရပ် | Operator မြင်တာ (အရင်) |
|---|---|
| Export အောင်မြင် | **ဘာမှ မမြင်** — "Choose a location…" toast ပဲ။ cancel နဲ့ ခွဲမရ |
| Delete: slot ၁၀ ခုမှာ ၂ ခုပဲ ရ | **clean success နဲ့ ထပ်တူ**။ row ၈ ခု ပြန်ကျန်တာကိုပဲ ကိုယ်တိုင် သတိထားရ |
| Port ၇ တိတ်သွား | `console.warn` — packaged build မှာ devtools ပိတ်ထားတာမို့ **မမြင်** |
| SIM cleanup က port ၃ ခုမှာ fail | Ports page footer မှာပဲ။ Inbox မှာ ရှိရင် မမြင် |

**အဓိက ဇစ်မြစ်:** `liveStore.statusText` က backend ရဲ့ long-running outcome အားလုံးကို
ကိုင်ထားပေမယ့် render ဖြစ်တာ **`Ports.svelte:547` တစ်နေရာတည်း** — operator က ဒီ event
အားလုံးအတွက် Inbox ပေါ် ရှိနေတယ်။

**Fix:**
- `delete:done` ကို payload ပါစေ: `{ requested, freed, removed, kept, failed_ports }`။
  **display string ကို frontend မှာ ပြန် parse မလုပ်ဘူး** — contract ကို explicit လုပ်တာ။
  `kept > 0 || failed_ports > 0` ဆိုရင် `Warning` toast, မဟုတ်ရင် `Success`
- `export:saved` + `sim_cleanup:done` listener ထည့်
- `live:reconnecting` ကို `Warning` toast (`detect:done` ပုံစံ)။ **`live:offline` က
  console-only အတိုင်း** — SIM မရှိတဲ့ slot က incident မဟုတ်ဘူး၊ code comment နဲ့လည် ကိုက်တယ်
- `Inbox.svelte` မှာ `page-footer` ထည့် (`Ports.svelte` markup ပြန်သုံး၊ token အသစ် မထည့်) —
  message count + `statusText`
- Preview parity: `deleteSelected` ရဲ့ non-Tauri branch လည် toast ပြရမယ်

**Toast တိုင်းကို `api.ts` ရဲ့ ရှိပြီးသား `toast` wrapper ကနေ ပဲ ပို့ပါ** — id counter
အသစ် မထည့်နဲ့။ `ToastContainer.svelte:21` က `{#each toasts as t (t.id)}` နဲ့ id ကို key
လုပ်တာမို့ counter နှစ်ခု ထပ်ရင် duplicate key က throw ဖြစ်တယ် (အခု counter ၃ ခု ရှိပြီးသား:
`api.ts:22` က 1 ကနေ, `logs.svelte.ts:6` က 1000 ကနေ, `updater.ts:45`)။

**ကျန်နေခဲ့တဲ့ အလုပ် — အခု ပြင်ပြီး (C.9 ကြည့်):** toast array က cap မရှိခဲ့ဘူး
(`live.svelte.ts:13`) ပြီးတော့ coalesce မလုပ်ခဲ့ဘူး။ `live:reconnecting` toast ထည့်လိုက်တာက
ဒီ ချောင်းကို **ပိုနီးစပ်စေခဲ့တယ်**။

### C.9 ✅ **DONE (v1.5.0):** Toast column ကို ဘောင်ခတ်တာ + တူတဲ့ notice တွေ ပေါင်းတာ

**ပြဿနာ:** `addToast` က `toasts = [...toasts, t]` — cap မရှိ၊ dedupe မရှိ။ Toast တစ်ခုက
၄ စက္ကန့် ရှင်ပြီး `.toast-container` က `max-height` မရှိတဲ့ fixed bottom-right column
(`app.css:258`)။ ဆိုတော့ **port ၁၆ ခု တစ်ပြိုင်နက် ကျရင် card ၁၆ ခု viewport အပေါ်ကို
ကျော်တက်ပြီး၊ အဲ့ failure ကို report နေတဲ့ UI ကိုယ်တိုင် ဖုံးတယ်**

**Fix — `src/lib/utils/toast-queue.ts`** (rune-free, `$lib`-free မို့ Node runner က
တိုက်ရိုက် import လုပ်နိုင်တယ် — `csv.ts`/`port-refresh.ts` precedent):
- `MAX_TOASTS = 5`၊ `pushToast` က newest ကို ထားပြီး `slice(-MAX_TOASTS)`
- `kind` + `title` တူရင် card တစ်ခုပေါ် ပေါင်းပြီး `count` တိုးတယ်၊ title က `Port lost (16)`
  ဖြစ်လာတယ် (`countSuffix`)
- **`otp` ပါတဲ့ toast ကို ဘယ်တော့မှ မပေါင်းဘူး** — code တစ်ခုချင်းစီက operator ဖတ်ရမယ့်
  သီးသန့် အရာ ဖြစ်တာမို့၊ ပေါင်းလိုက်ရင် တစ်ခု တိတ်တဆိတ် ပျောက်မယ်
- Body က **newest** ကို ယူတယ်၊ merge မလုပ်ဘူး — port ၁၆ ခုရဲ့ နာမည် ပေါင်းထားတဲ့ body ဟာ
  ၄ စက္ကန့် card ထဲ ဖတ်လို့ မရဘူး၊ count က scale ကို ပြပြီးသား
- Coalesced card ကို array အဆုံးကို ရွှေ့တယ် — repeat က မျက်လုံး ခွာသွားပြီးတဲ့ card ကို
  update လုပ်တာထက် အောက်ဘက်မှာ ပြန်ကြေငြာတာ ပိုကောင်းတယ်

**4 စက္ကန့် timer:** `setTimeout` က schedule လုပ်တဲ့ id ပေါ်မှာပဲ key လုပ်တယ်။ Coalesced card
က id အသစ် ဖြစ်တာမို့ ကျော်သွားတဲ့ timer က ဖျက်စရာ မတွေ့ဘူး၊ merged card က သူ့ကိုယ်ပိုင်
၄ စက္ကန့် အပြည့် ရတယ် — **flap ဖြစ်နေသေးတဲ့ port ရဲ့ notice က ဆက်ရှိနေတယ်**၊ ဒါ ရည်ရွယ်ချက်

**Test ၁၃ ခု** (`toast-queue.test.ts`) — cap, coalesce, OTP မပေါင်းတာ, title/kind ကွာရင်
မပေါင်းတာ, immutability, cap အောက်မှာ coalesce လုပ်တာက တခြား card မပျောက်တာ

**Preview မှာ အတည်ပြုပြီး:** Refresh ၁၀ ခါ ဆက်တိုက် → card **၁** ခု `Refreshed (10)`,
container height 127px မှာ ရပ်တယ် (အရင်က card ၁၀ ခု စီမယ်)။ Cap ကိုယ်တိုင်ကို preview မှာ
မရောက်နိုင်ဘူး — synthetic app က distinct title ၂ ခုပဲ ထုတ်တာမို့၊ ဒါကို unit test က ဖမ်းတယ်

### C.10 Supervisor ၄ ခုကို `run_port_pool` အဖြစ် ပေါင်းတာ — **တမင် ရွှေ့ထားတာ** (bug မဟုတ်ဘူး)

v1.5.0 မှာ လုပ်မလုပ် စဉ်းစားပြီး **သဘောတူ ချန်ထားလိုက်တဲ့ code refactor တစ်ခုတည်း**။ အရင်က
doc 03 §T5 Rule ၃ မှာ passing note အဖြစ်ပဲ ရှိခဲ့တာကို ဒီမှာ entry အဖြစ် တင်လိုက်တယ်။

**ပုံစံ ထပ်တူ ဖြစ်နေတာ (= duplication debt):** port-heavy command ၄ ခုက structure တူတူ —
`Arc<Mutex<Vec<String>>>` work queue + `take_port` + worker cap + per-port `catch_unwind` +
supervisor က join ပြီး busy flag ရှင်း၊ status line တည်၊ `*:done` emit:

| Command | supervisor | worker | per-port `catch_unwind` | cap |
|---|---|---|---|---|
| `detect_ports` | `src-tauri/src/commands/mod.rs:376` | `:391` | `:393` | `MAX_CONCURRENT_PROBES` = 32 (`:134`) |
| `start_scan` | `:611` | `:620` | `:627` | `MAX_CONCURRENT_PORTS` = 16 (`:128`) |
| `get_sim_numbers` (USSD) | `:750` | `:764` | `:766` | `MAX_CONCURRENT_PORTS` |
| `cleanup_sim_storage` | `:1568` | `:1579` | `:1581` | `MAX_CONCURRENT_PORTS` |

Live supervisor (`:997`၊ per-port spawn `:1007`၊ `run_live` ရဲ့ `catch_unwind` `:1214`) က
cap **မရှိဘူး** (§C.5) ဆိုတော့ ပေါင်းရင် သူပါ ဒီ pool ထဲ ဝင်လာမယ် — အဲ့ဒါက C.5 ကိုပါ
တစ်ပြိုင်နက် ဖြေမယ့် ဆွဲအား။

**Blocker — panic-accounting policy ၄ မျိုး တကယ် မတူတာ** (boilerplate မဟုတ်ဘူး၊ behaviour):

| Command | port တစ်ခု panic ဖြစ်ရင် ဘာ လုပ်လဲ |
|---|---|
| scan | `failed_notes.push("{port} (worker panicked)")` **+** `scan_done += 1` (`:628`–`:633`) — status line မှာ ပေါ်တယ် |
| ussd | `done` ကိုပဲ တိုးတယ် (`:769`–`:772`) — ဆိုတော့ panic ဖြစ်တဲ့ port က "number မတွေ့ဘူး" လို့ ဖတ်တယ် |
| cleanup | failure counter တိုးတယ် (`:1599`–`:1602`) |
| detect | #19 ကတည်းက `ProbeVerdict::of(&port, probed)` (`:396`၊ enum `:306`၊ `of` `:319`) — panic က **`Inconclusive`**၊ ဒါကြောင့် `alive`/`checked`/`iccid`/`sim_dir` **လေးခုလုံး မထိရ** (`:429`၊ `:456`) |

**ဆိုတော့ naive merge က behaviour-normalising refactor ဖြစ်တယ်:** policy တစ်ခု ရွေးလိုက်ရင်
ကျန် ၃ ခုရဲ့ semantic ပြောင်းတယ်။ အဆိုးဆုံးက detect — crashed probe ကို "dead" အဖြစ်
ချုံ့ပစ်လိုက်ရင် **doc 03 case §16 ကို ပြန်မွေးတာ** ဖြစ်မယ် (momentary EBUSY တစ်ခုနဲ့ bank ရဲ့
slot→ICCID hint ပျောက်တာ)။ တကယ် လုပ်ရင် per-port `on_panic` callback နဲ့ policy ကို caller
ဆီ ချန်ရမယ် — pool ထဲ hardcode မလုပ်ရ။

**ဘာလို့ အစဉ်လိုက် အနောက်ဆုံးလဲ:** risk အမြင့်ဆုံး (diff က "ကုဒ် သိမ်းတာ" ပုံပေါက်ပြီး
semantic ပြောင်းနိုင်တာ)၊ ပြီးတော့ **operator မြင်ရမယ့် value က သုည**။ ဒါကြောင့် C.3
(ဆုံးဖြတ်ချက် လိုတာ) နဲ့ C.4–C.7 (operator တကယ် မြင်ရတဲ့ အပေါက်) အားလုံး ဒီအရင်။

**bug မဟုတ်ဘူး:** supervisor ၄ ခုက အခု အလုပ်လုပ်တယ်၊ exit path ကို `BusyGuard` (§T5) က
ပိတ်ပြီးသား၊ per-port `catch_unwind` လည်း ၄ ခုလုံးမှာ ရှိပြီးသား။ ကျန်တာက duplication ပဲ။

## D. Dropped — feature အဖြစ်ပါ **ပြန်မမွေးရ**

| Item | ဆုံးဖြတ်ချက် |
|---|---|
| **Compact Mode** | မလုပ်ဘူး။ Value နဲ့ မကိုက်တဲ့ CSS surface ကြီးမား — spacing utility တွေ component တိုင်းမှာ ကွဲပြားနေတာကို density variant ၂ မျိုး ထိန်းရမယ် |
| **Auto-copy OTP** | Switch အဖြစ် မထည့်ဘူး။ Owner နောက်ပိုင်း ပြန်စဉ်းစားနိုင်တယ် — ဒါပေမဲ့ **feature နဲ့အတူ** ပြန်လာရမယ်၊ feature ကို စောင့်နေတဲ့ switch အဖြစ် မဟုတ်ဘူး (doc 04 §H) |

## E. Desktop Notifications — Feasibility (စစ်ပြီး၊ **ပြန်မစစ်ရ**)

**Owner လိုချင်ခဲ့တာ:** message ကို ဖတ်ပြီးတာနဲ့ (ဒါမှမဟုတ် Read & Scan ပြေးတာနဲ့) native
notification က **အလိုအလျောက် ပြန်ပျောက်** သွားတာ။ → **desktop မှာ မရနိုင်ဘူး။** သက်သေ:

| အလွှာ | တွေ့ရှိချက် |
|---|---|
| TS surface (`@tauri-apps/plugin-notification` **2.3.3**) | `removeActive()`၊ `active()`၊ `cancel()`/`cancelAll()`၊ `id?: number`၊ `group?: string` — အားလုံး declare ထားတယ်၊ **ရှိသလို ထင်စေတယ်** |
| Rust plugin (`tauri-plugin-notification` 2.3.3, `src/lib.rs` `generate_handler!`) | command **၃ ခုပဲ** register တယ်: `notify`၊ `request_permission`၊ `is_permission_granted` |
| ဆိုတော့ | `removeActive()`/`active()`/`cancel()` ကို ခေါ်ရင် no-op မဟုတ်ဘူး — **"command not found" throw တယ်** (upstream issue **#1898**၊ ဖွင့်ထားတုန်း) |
| Dismissal code ဘယ်မှာလဲ | `mobile.rs` မှာပဲ (`remove_active`/`active`/`cancel` → `run_mobile_plugin`)။ `desktop.rs` မှာ မရှိ |
| `desktop.rs` show path | `title`/`body`/`icon`/`sound` ကိုပဲ map တယ်၊ **`id` နဲ့ `group` ကို တိတ်တဆိတ် ချပစ်တယ်**၊ ပြီးတော့ `tauri::async_runtime::spawn` ထဲမှာ `let _ = notification.show()` — handle ကို ချက်ချင်း စွန့်လိုက်တာမို့ ပြန်ပိတ်ဖို့ ကိုင်စရာ မကျန် |
| အောက်ခြေ backend | `notify-rust` **4.18** က Linux မှာ D-Bus `CloseNotification` + `replaces_id` နဲ့ **ပိတ်/အစားထိုး လုပ်နိုင်တယ်**။ Windows ဘက် `tauri-winrt-notification` **0.7.3** မှာ removal ဒါမှမဟုတ် replace-by-tag path **လုံးဝ မရှိ** |

**ဆိုတော့ တကယ် လုပ်ရမယ်ဆိုရင်:** Linux-only dismissal ကို plugin ကို ကျော်ပြီး custom Rust
command (notification handle ကို state ထဲ သိမ်း → close) နဲ့ ရေးရမယ်။ Windows အတွက် WinRT ကို
လက်နဲ့ ရေး + **AUMID register** ဖြစ်ရမယ် — installed app မှာပဲ အလုပ်လုပ်တာမို့ `cargo tauri dev`
အောက်မှာ **စမ်းလို့ မရဘူး** (plugin ကိုယ်တိုင် `target/debug`|`target/release` ကနေ ပြေးရင်
`app_id` ကို မ set တာ ဒီအတွက်)။

**ACL:** `src-tauri/capabilities/default.json` မှာ `notification:` entry **ဘယ်တုန်းကမှ မရှိခဲ့ဘူး** —
ဆိုတော့ plugin ရှိစဉ်ကတောင် ရိုးရိုး notification တစ်ခုကို ACL က ပိတ်ထားခဲ့တယ် (အဲ့တုန်းက plugin က
`lib.rs` မှာ init ဖြစ်နေခဲ့တာ)။ ပြန်ထည့်ရင် လိုအပ်မယ့် **minimum set က ၃ ခု**:

```jsonc
"notification:allow-notify",
"notification:allow-is-permission-granted",
"notification:allow-request-permission"
```

`notification:default` ကို မသုံးပါနဲ့ — သူက ဒီ ၃ ခုအပြင် desktop မှာ **မရှိတဲ့** command
၁၃ ခုအတွက် identifier တွေပါ ထပ်ပေးတယ် (`allow-remove-active`၊ `allow-get-active`၊ `allow-cancel`၊
`allow-batch`၊ channel API စသည်) — ခေါ်လိုက်ရင် throw ဖြစ်မယ့် ဟာတွေကို permit ထားတာ။

**⚠️ ဆုံးဖြတ်ရမယ့် security tension (ပြန်စရင် ဒါကို အရင် ဖြေရမယ်):** OTP code ကို notification
body ထဲ ထည့်လိုက်ရင် သူ OS notification center (Linux မှာ ရှိတဲ့ history daemon) ထဲ ရေးဝင်တယ်၊
lock screen ပေါ် ပေါ်နိုင်တယ် — အဲ့ဒါက log masking work (§B.2၊ `mask_number`/`otp_summary`) ရဲ့
ဆန့်ကျင်ဘက်။ အနည်းဆုံး "OTP ရောက်ပြီ (port X)" လို့ပဲ ပြပြီး code ကို app ထဲမှာသာ ပြသင့်တယ်။

**✅ Cleanup ပြီးသွားပြီ (v1.4.0) — plugin ကို ဖျက်လိုက်ပြီ:** `@tauri-apps/plugin-notification`
က ဘယ်နေရာကမှ import မဖြစ်တဲ့ **unused dependency** ဖြစ်နေခဲ့တာမို့ အထက်က feasibility
ဆုံးဖြတ်ချက် (desktop မှာ dismissal မရနိုင်) အပေါ် အခြေခံပြီး ၃ နေရာလုံး တစ်ပြိုင်နက်
ဖျက်လိုက်ပြီ — `package.json`၊ `src-tauri/Cargo.toml` (`tauri-plugin-notification = "2"`)၊
`src-tauri/src/lib.rs` (`.plugin(tauri_plugin_notification::init())`)။ (commit `d5d53e5` ရဲ့
`chore: drop the unused notification plugin`)

- **ACL ကနေ ဘာမှ မပြုတ်ဘူး:** `src-tauri/capabilities/default.json` မှာ `notification:`
  permission **ဘယ်တုန်းကမှ မရှိခဲ့ဘူး** (ဖိုင်ရဲ့ git history တစ်ခုလုံး စစ်ပြီး — `notification`
  ဆိုတဲ့ စာလုံး ဘယ် revision ထဲမှ မရှိ)။ ဆိုတော့ ဖျက်လိုက်တာက capability တစ်ခုကို
  **revoke လုပ်တာ မဟုတ်ဘူး** — plugin က `lib.rs` မှာ init ဖြစ်နေခဲ့ပေမဲ့ ACL က ပိတ်ထားခဲ့တာမို့
  runtime behaviour ကို **လုံးဝ မထိဘူး**
- **တကယ့် အကျိုးဆက် ၂ ချက်ပဲ:** (၁) bundle ငယ်သွားတာ၊ (၂) Linux မှာ **D-Bus dependency
  တစ်ခု လျော့သွားတာ**။ `notify-rust`၊ `tauri-winrt-notification`၊ `mac-notification-sys`၊
  `zbus` တွေ ကျွတ်သွားပြီး **`Cargo.lock` က ၅၂၁ လိုင်း ဆုတ်သွားတယ်** (`package-lock.json` ၁၀ လိုင်း)
- **ပြန်ထည့်ရင် သတိ:** အထက်က table/ACL အပိုင်းက **ပြန်ထည့်တဲ့အခါ လိုအပ်တဲ့ လက်စွဲ** အဖြစ်
  ကျန်ထားတာ — dismissal dead end ကို **ပြန်မစစ်ရ**၊ ပြီးတော့ ပြန်ထည့်ရင် permission ၃ ခုကို
  ကိုယ်တိုင် ရေးထည့်ရမယ် (`notification:default` ကို မသုံးရ)




