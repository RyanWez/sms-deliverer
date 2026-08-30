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
**တမင် ချန်ထားတာ** — §C ကြည့်။

## B. Hard Refusals — ဒီ ၂ ခုကို editable field အဖြစ် **ပြန်မထည့်ရ**

ဒါတွေက "အချိန်မရလို့ ရွှေ့ထားတာ" မဟုတ်ဘူး၊ **ငြင်းပယ်ထားတာ**။ ဖျက်ရတဲ့ အကြောင်းရင်းက
implementation cost မဟုတ်ဘူး — control ကိုယ်တိုင်က တကယ် အလုပ်လုပ်လိုက်ရင် ပိုဆိုးတာ။

### B.1 `otp.otpPattern` — operator-editable OTP regex

OTP detection က `src-tauri/src/core/decoder.rs::extract_otp` — regex တစ်ခုတည်း မဟုတ်ဘူး၊
**keyword gate + ordered cascade ၄ ဆင့်** (အားလုံး `LazyLock<Regex>` statics):

1. `normalize_myanmar_digits()` — မြန်မာ digit (`U+1040`–`U+1049`) → ASCII
2. `KEYWORD_RE` **gate** — `otp|one.?time|code|pin|verification|verify|confirm` + မြန်မာ keyword
   constants (`KW_KODE` = ကုဒ်၊ `KW_CONFIRM`၊ `KW_SECURE` = လုံခြုံ)။ **မ match ရင် ချက်ချင်း `None`**
   *(`KW_CONFIRM` စာလုံးပေါင်း အမှား ရှိတယ် — doc 03 §T3)*
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

### C.1 ⬅️ **NEXT:** Theme Dark/Light တကယ် အလုပ်လုပ်အောင် လုပ်

Control က Settings ပေါ် ရှိတယ် (System / Dark / Light)၊ **Light က no-op ဖြစ်နိုင်တယ်**။
တစ်ခုတည်း မဟုတ်ဘူး — **သီးသန့် blocker ၃ ခု** ရှိတယ်၊ ၃ ခုလုံး မဖြေရင် theme မလာဘူး:

| # | Blocker | တကယ့် အခြေအနေ |
|---|---|---|
| 1 | **Class strategy မပြည့်စုံ** | `applyTheme()` (`src/lib/stores/settings.svelte.ts`) က `dark` class ကို add/remove ပဲ လုပ်တယ် — `light` class ဆိုတာ မရှိ၊ ပြီးတော့ `src/` တစ်ခုလုံးမှာ **`dark:` Tailwind variant တစ်ခုမှ မသုံးထားဘူး** (grep → zero)။ Component တွေက `bg-surface` စတဲ့ CSS-variable token တွေပဲ သုံးတယ် |
| 2 | **Light token တွေ OS ကို ချုပ်ခံထား** | `src/app.css` ရဲ့ light palette က `@media (prefers-color-scheme: light) { :root:not(.dark) { … } }` အောက်။ OS က dark ဖြစ်တဲ့ machine မှာ Light ရွေးရင် — `dark` class ဖြုတ်ပေမဲ့ media query က မ match တာမို့ — `:root` ရဲ့ dark token တွေ ကျန်နေတယ်၊ **ဘာမှ မပြောင်း** |
| 3 | **Shell က dark ကို pin ထား** | `index.html`: `<html class="dark" style="…color-scheme: dark">` + inline `<style>` မှာ `html, body, #app { background-color: #171717 !important }` (+ `color: #e4e4e4`၊ `<body style>`၊ `meta theme-color`/`color-scheme`)။ `!important` ကြောင့် app background ကို token နဲ့ override မရဘူး၊ `color-scheme: dark` ကြောင့် **native scrollbar နဲ့ `<select>` dropdown တွေလည်း dark ကျန်နေမယ်** |

> ⚠️ **`index.html` block ကို ဒီအတိုင်း မဖျက်ပါနဲ့** — သူက **flash guard**။ Vite က CSS ကို
> inject မလုပ်ခင် webview က default white ကို ခဏ ပြတယ်၊ dark app မှာ အဲ့ဒါက အလင်းပြက်ခြင်း
> အဖြစ် မြင်ရတယ်။ Theme လုပ်တဲ့အခါ ဒီ guard ကို **persist ထားတဲ့ theme အလိုက် ရွေးပေးတဲ့
> inline script** (localStorage `sms-reader-settings` ကို `<head>` ထဲကတင် ဖတ်တာ) နဲ့ အစားထိုးရမယ် —
> ဖျက်ပစ်တာ မဟုတ်ဘူး။

**Bonus trap:** `src/lib/pages/Logs.svelte:223` က `bg-[#0d1117] text-[#e6edf3]` လို့ hardcode
ထားတယ် — `src/` တစ်ခုလုံးမှာ **hardcoded hex တစ်ခုတည်း**။ Console အတွင်းထဲက log line တွေက
token colour (`text-muted-foreground` စသည်) သုံးတာမို့ light theme အလုပ်လုပ်လာရင် Logs page က
"dark ကျန်နေတာ" မဟုတ်ဘဲ **အလင်းပေါ်အလင်း — လုံးဝ မဖတ်နိုင်တာ** ဖြစ်လာမယ်။ Console အတွက်
သီးသန့် token pair (`--console-bg` / `--console-fg`) ထည့်ရမယ်။

### C.2 `general.portRefreshInterval` — inert အတိုင်း **တမင် ချန်ထား**

Field က `types.ts` နဲ့ Settings page ၂ ခုလုံးမှာ ရှိတယ် (default `30`)၊ timer တစ်ခုမှ မဖတ်ဘူး။
Feature က တကယ် အသုံးဝင်တယ် (SIM bank ကို re-plug လုပ်ရင် အလိုအလျောက် သိစေတာ) — ဒါကြောင့်
ဖျက်မထားဘူး။ ဒါပေမဲ့ **trap:**

`refresh_ports` (`src-tauri/src/commands/mod.rs`) က port အားလုံးအတွက် **`live_ready: false`** နဲ့
`PortInfo` အသစ် ပြန်တည်တယ် (`checked`/`alive`/`iccid` ကိုပဲ stable `path` နဲ့ carry over တယ်)။
ဆိုတော့ live mode ဖွင့်ထားစဉ် background timer တစ်ခု ပြေးလိုက်ရင် **LIVE badge အားလုံး ပြုတ်သွားမယ်** —
modem တွေ ကောင်းနေတဲ့ အခိုက်မှာ။

Implementation က ဒီ ၂ ခုထဲ တစ်ခု လိုတယ်:
- Frontend timer ကို `App.svelte` ရဲ့ `portsBusy()` helper နဲ့ gate လုပ် (live / scan / USSD /
  delete busy ဖြစ်ရင် skip) — SIM cleanup timer တွေ ဒီ pattern ကို သုံးပြီးသား၊ ဒါမှမဟုတ်
- `refresh_ports` ကိုယ်တိုင် `live_ready`/`live_error` ကို old map ကနေ preserve လုပ်အောင် ပြင်

Owner လိုချင်တာ ထပ်တစ်ခု: auto-detect လုပ်လိုက်တဲ့ **re-plug ကို UI မှာ ပြပေးတာ**
(port အသစ် ပေါ်လာ/ပျောက်သွားတာကို status line ဒါမှမဟုတ် toast နဲ့) — refresh တိတ်တဆိတ် ဖြစ်နေတာ မဟုတ်ဘဲ။

### C.3 `developer.autoScroll` — inert အတိုင်း၊ **မဆုံးဖြတ်ရသေး**

Logs page မှာ session-local toggle ကိုယ်ပိုင် ရှိပြီး အလုပ် လုပ်နေတယ် (`logsStore.autoScroll`၊
default `true`၊ `src/lib/stores/logs.svelte.ts`)၊ setting ကနေ မဖတ်ဘူး။ Wire လုပ်တာ ခက်တာ မဟုတ် —
store ကို settings ကနေ **seed** လုပ်ရမယ်၊ ပြီးတော့ toggle ကို session-only ထားမလား setting ထဲ
ပြန်ရေးမလား ဆိုတာ ဆုံးဖြတ်ရမယ် (owner မဆုံးဖြတ်ရသေး)။

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

**ACL:** `src-tauri/capabilities/default.json` မှာ `notification:` entry **မရှိဘူး** — ဆိုတော့
ရိုးရိုး notification တစ်ခုတောင် ယခု ACL က ပိတ်ထားတယ် (plugin က `lib.rs` မှာ init ဖြစ်ပေမဲ့)။
လိုအပ်တဲ့ **minimum set က ၃ ခု**:

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

**Cleanup candidate (ယခု မဖျက်ပါနဲ့):** `@tauri-apps/plugin-notification` က ဘယ်နေရာကမှ
import မဖြစ်တဲ့ **unused dependency** ဖြစ်နေပြီ — `package.json`၊ `src-tauri/Cargo.toml`
(`tauri-plugin-notification = "2"`)၊ `src-tauri/src/lib.rs` (`.plugin(tauri_plugin_notification::init())`)
၃ နေရာ။ Notification ကို ပြီးပြီးပြတ်ပြတ် စွန့်လွှတ်ဆုံးဖြတ်မှ ၃ နေရာ တစ်ပြိုင်နက် ဖျက်ပါ။




