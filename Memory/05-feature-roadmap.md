# 🚀 Planned Features & Roadmap (အနာဂတ် လုပ်ဆောင်ချက်များ)

> **Project:** SIM Bank SMS Reader (`sms-tauri`) · Repo: `RyanWez/sms-deliverer`  
> **Status:** Backlog / Future Enhancements (အနာဂတ်တွင် ဆက်လက် အကောင်အထည်ဖော်မည့် Features စာရင်း)

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
