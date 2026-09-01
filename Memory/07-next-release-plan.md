# 🗺️ Next Release Plan — v1.5.0 Field Test ကနေ ဆွဲထုတ်ထားတဲ့ အစီအစဉ်

> **အခြေခံ:** v1.5.0 (tag `v1.5.0`၊ signed installer စစ်ပြီး၊ PR ၇ ခု merge) ကို
> **တကယ့် 64-port SIM bank ပေါ် operator ကိုယ်တိုင် run ခဲ့တဲ့ session** ရဲ့ log။
> **Purpose:** ဒီ doc က backlog မဟုတ်ဘူး — **နောက် release ၂ ခု (v1.5.1 / v1.6.0) ရဲ့
> အကြောင်းအရာ** ကို field သက်သေနဲ့ တွဲပြီး သတ်မှတ်ထားတာ။ Feature backlog နဲ့ deferred
> order က `05-feature-roadmap.md`၊ bug casebook က `03-troubleshooting.md`။
> **File:line သက်သေ အားလုံး v1.5.0 ကုဒ်ပေါ် စစ်ထားတာ** — item ကို နာမည်နဲ့ ရှာပါ၊
> နံပါတ်နဲ့ မရှာနဲ့ (`commands/mod.rs` က change တိုင်း ရှည်လာတယ်)။

---

## A. v1.5.0 Field Verification — ဘာ သက်သေပြီးလဲ၊ ဘာ မသက်သေရသေးလဲ

### A.1 ✅ PR #12 (`fix: give live-mode messages their real SIM slot index`) — **hardware ပေါ် အတည်ပြီး**

doc 03 §14 က bug ဖြစ်တယ်။ v1.4.0 မှာ `parse_cmgr` / `parse_pdu_cmgr` က `index: 0` ကို
hardcode ထားတာမို့ live path (`+CMTI` → `AT+CMGR`) ကနေ ရောက်လာတဲ့ message တိုင်း slot 0၊
delete လုပ်ရင် `AT+CMGD=0` ပို့ပြီး — SIM slot က **1 ကနေ** စတာမို့၊ ပြီးတော့ confirmation က
absence-based ဖြစ်တာမို့ — "ပျက်သွားပြီ" လို့ **false success** ဖတ်ခဲ့တယ်။

**သက်သေ ၁ — slot နံပါတ် တကယ် ပါလာတယ်** (v1.4.0 မှာ `idx 0` အမြဲ):

```
18:40:04.539  COM38: live SMS read (idx 4) [concat]
              COM38: live SMS read (idx 5)
              COM38: live SMS read (idx 6)
              COM38: live SMS read (idx 7)
              COM38: live SMS read (idx 8) [concat]
              COM38: live SMS read (idx 9) [concat]
```

Log site: `src-tauri/src/core/live.rs:517` (concat suffix `:520`) နဲ့ `:535`။

**သက်သေ ၂ — live ကနေ ရောက်လာတဲ့ single-slot message က SIM ပေါ်ကနေ တကယ် ထွက်သွားတယ်:**

```
18:46:47.485  COM38: initial batch 6 msg(s)
18:47:07.303  Live stop requested
18:47:15.819  COM38: deleted 1 msg(s)
18:47:15.825  Deleted 1 message(s) (1 SIM slot(s) freed)
18:47:20.214  COM38: initial batch 5 msg(s)
```

**ဒီမှာ အရေးကြီးတာက `initial batch` က UI ရဲ့ ကိုယ်ပိုင် count မဟုတ်ဘူး ဆိုတာ** —
live worker က port ဖွင့်ပြီး `AT+CMGL=4` ပို့လိုက်တဲ့ **card ကိုယ်တိုင်ရဲ့ အဖြေ**
(`live.rs:250` list → `:292` log)။ ဆိုတော့ `6 → 5` က "row ပျောက်တယ်" မဟုတ်ဘူး၊
**SMS က SIM ပေါ်ကနေ တကယ် ထွက်သွားတယ်** ဆိုတဲ့ သက်သေ။ v1.4.0 ဆိုရင် ဒီနေရာမှာ
`6` အတိုင်း ကျန်နေမယ် — အဲ့ဒါက §14 ရဲ့ တကယ့် symptom ပဲ။

**သက်သေ ၃ — live ကနေ ရောက်လာတဲ့ concat (part ၂ ခု၊ slot 8 + 9၊ KBZPay) ကို reassemble
လုပ်ပြီး OTP ၆ လုံး ထုတ်နိုင်တယ်:**

```
18:46:14.954  NEW SMS on COM38: from=***Pay otp=found (6 digits)
```

Log site `src-tauri/src/commands/mod.rs:1123` — sender က `mask_number`၊ OTP က
`otp_summary` ကနေ ဖြတ်လာတာ (AGENTS.md Logging rule)။ **ဒီ doc ထဲ masked မဟုတ်တဲ့
နံပါတ် / OTP တစ်ခုမှ မရေးရ** — အပေါ်က log line တွေက mask ပြီးသား ဖြစ်လို့ ဒီအတိုင်းပဲ ကူးထားတယ်။

**သက်သေ ၄ (session အစပိုင်းက) — scan path ကနေ လာတဲ့ concat row (slot ၂ ခု) ကို delete
လုပ်တာ ၂ ခုလုံး confirmed ဖြစ်ပြီး re-scan မှာ card ဗလာ ဖြစ်တယ်:**

```
COM17: pdu-mode read -> 0 msg(s)
Deleted 1 message(s) (2 SIM slot(s) freed)
```

### A.2 ⏳ မစမ်းရသေးတာ — **live ကနေ ရောက်လာတဲ့ concat row ကို delete လုပ်တာ**

Slot ၂ ခုကို **live path ကနေ** တစ်ပြိုင်နက် ဖြုတ်တာ hardware ပေါ် **မလုပ်ရသေးဘူး**။
အပိုင်း ၂ ခုစီ သက်သေ ရှိတယ် (A.1 သက်သေ ၂ = live + single slot၊ သက်သေ ၄ = concat ၂ slot
ဒါပေမဲ့ scan path) — ဒါပေမဲ့ **တစ်ခုတည်းအတွင်း မဟုတ်ဘူး**။ `part_indices` က တကယ့် slot
တွေ စုမိလားဆိုတာ ဒီ combination မှာပဲ ပြတ်သားစွာ မြင်ရမယ်။ §B ရဲ့ mailbox အလုပ်လုပ်ပြီးရင်
`04 §G` playbook ထဲ ဒီ case ကို ထည့်စမ်းရမယ်။

### A.3 ⚠️ Field run က **မ ဖိစီးလိုက်တဲ့** အပိုင်းများ (ship ပြီး၊ ဒါပေမဲ့ field-proven မဟုတ်)

| ပါလာတဲ့ ဟာ | field မှာ ဘာ ဖြစ်ခဲ့လဲ |
|---|---|
| `BusyGuard` (#18၊ `commands/mod.rs:103`၊ `Drop` `:117`) | panic တစ်ခုမှ မဖြစ်ခဲ့ဘူး — ဆိုတော့ guard ရဲ့ `Drop` path က **တစ်ခါမှ မ fire ခဲ့ဘူး**။ "Busy" ကပ်နေတာ မဖြစ်ခဲ့တာက guard ကောင်းလို့လား၊ ဒါမှမဟုတ် panic မဖြစ်လို့လား **မခွဲနိုင်ဘူး** |
| Live worker ရဲ့ `catch_unwind` (`core/live.rs:137`၊ `WORKER_PANIC` `:56`) | `Worker crashed` line တစ်ခုမှ မရှိဘူး — unproven အတူတူ |
| `ProbeVerdict::Inconclusive` (#19၊ enum `commands/mod.rs:306`၊ `of` `:319`) | **တစ်ခါမှ မထွက်ခဲ့ဘူး**။ Dead port ၃၀ လုံး `NOT_RESPONDING` → `Empty` ကနေ သွားတယ်: `Detect done. Modems found: 34/64 \| 30 port(s) with no modem deselected` (status builder `:514`၊ `Inconclusive` ရှိရင် ထပ်ဆင့် clause `:520`–`:525` ပါလာမယ် — မပါခဲ့ဘူး)။ ဆိုတော့ **`Empty` လမ်းကြောင်းပဲ field-proven**၊ doc 03 §16 ကို ပြန်မမွေးဘူးဆိုတာ သက်သေ မရသေးဘူး |

**ဆိုလိုတာ:** ဒီ ၃ ခုကို "စမ်းပြီးသား" လို့ မမှတ်ရ။ EBUSY / ModemManager contention
တမင် ဖန်တီးတဲ့ step ကို `04 §G` playbook ထဲ ထည့်တာ လိုတယ်။

---

## B. ISSUE 1 — Live mode ဖွင့်ထားရင် Delete / Clear All / Get SIM Numbers လုပ်လို့ မရဘူး

**→ နောက် release ရဲ့ headline item (v1.6.0)။**

- **Symptom:** operator က live ဖွင့်ထားစဉ် row တစ်ခု ရွေးတယ် — `Delete Selected` က
  **disabled**။ `Clear All`၊ `Get SIM Numbers` လည်း အတူတူ။ Live ကို Stop လုပ်မှ ပြန်ရတယ်
- **v1.5.0 regression မဟုတ်ဘူး** — v1.4.0 မှာလည် ဒီအတိုင်းပဲ။ ဒါပေမဲ့ **#12 က ဒါကို
  အရေးပါလာစေတယ်**: အရင်က live message ကို delete လုပ်ရင် "အလုပ်လုပ်တယ်" ပုံပေါက်ပြီး
  တကယ် ဘာမှ မဖြစ်ခဲ့တာမို့ **"ဘယ်အချိန် delete လုပ်လို့ ရလဲ" ဆိုတာ ဘယ်သူမှ မမေးခဲ့ဘူး**။
  အခု တကယ် အလုပ်လုပ်လာတာနဲ့ အဲ့ဒီ မေးခွန်းက တကယ့် မေးခွန်း ဖြစ်လာတယ်

### B.1 Mechanism — အလွှာ ၃ ခု

| အလွှာ | သက်သေ | ဘာ လုပ်လဲ |
|---|---|---|
| Frontend | `src/lib/components/Toolbar.svelte:10`–`:16` — `busy` derivation ထဲ **`liveStore.on` (`:11`)** ပါတယ် | `Delete Selected` (`:109`)၊ `Get SIM Numbers` (`:92`)၊ `Clear All` (`:135`)၊ `Scan & Read All` (`:66`) အားလုံး `disabled={busy}`။ `Live Mode` button တစ်ခုပဲ ကိုယ်ပိုင် condition (`:80` — `liveStore.on` မပါ) |
| Backend gate | `AppStateInner::port_busy()` (`src-tauri/src/commands/mod.rs:53`၊ `live_on` `:55`) | Button ကို ဖြုတ်ပေးလိုက်တာနဲ့တောင် `delete_selected` (`:1331`) ရဲ့ ပထမ စစ်ချက် (`:1338`) က `Err("Busy")` ပြန်မယ် |
| **တကယ့် root cause — physical** | `modem::delete_messages` (`src-tauri/src/core/modem.rs:566`) က **port ကို ကိုယ်တိုင် ဖွင့်တယ်** (`at::AtChannel::open`၊ `:567`) | live worker က အဲ့ port ကို **ကိုင်ထားပြီးသား**။ Windows မှာ COM port ကို ၂ ခါ ဖွင့်လို့ မရဘူး — ဆိုတော့ UI gate ကို ဖြုတ်လိုက်ရင် "Access denied" ပဲ ရမယ်၊ delete မရဘူး |

ဆိုတော့ ဒါက UI bug မဟုတ်ဘူး — **architecture ရဲ့ ကျန်နေတဲ့ အပေါက်**။ `Get SIM Numbers`
(`modem::get_sim_number` `:642`၊ open `:643`) နဲ့ `Clear All` (`delete_messages` အတူတူ)
လည် တူတဲ့ အကြောင်းအရင်း တစ်ခုတည်းကြောင့် ပိတ်နေတာ။

### B.2 လက်ရှိ workaround (Stop Live → Delete → Start Live) ရဲ့ တန်ဖိုး — ၃ ချက်

1. **Live ပြန်စတာက port တိုင်းကို `AT+CMGL` နဲ့ ပြန်ဖတ်တယ်** (`core/live.rs:250`၊
   text fallback `:256`၊ timeout ၁၅ s) — log မှာ `initial batch` line တွေ အဖြစ် မြင်ရတယ်၊
   ဒီ bank မှာ **port ၃၄ ခုစာ**။ Delete တစ်ခုအတွက် bank တစ်လုံးလုံး ပြန် backfill
2. **Stop/Start ကြားထဲ ရောက်လာတဲ့ SMS က `+CMTI` ကနေ မလာဘူး** — initial batch ထဲ
   ပါလာတယ်၊ ပြီးတော့ ပထမ connect ဖြစ်တာမို့ `is_new: false` နဲ့ `LiveEvent::Batch`
   (`live.rs:310`၊ handler `commands/mod.rs:1087`) ကနေ ဝင်တယ် — ဆိုတော့ **live badge မရှိ၊
   OTP toast မရှိ**။ Operator က OTP ကို စောင့်နေရင်း အဲ့ဒါက တိတ်တဆိတ် ရောက်ပြီးသား ဖြစ်နေတယ်
3. **`stop_live` က ချက်ချင်း မပြီးဘူး** — `stop_live` (`commands/mod.rs:1261`) က
   `live_on` ကို ချက်ချင်း ရှင်းတယ် (`:1266`) ဒါပေမဲ့ worker တွေက supervisor join မလုပ်မချင်း
   port ကို ကိုင်ထားတယ်၊ `AT+CMGL=4` (15000 ms timeout) ထဲ ထိုင်နေတဲ့ worker ဆိုရင်
   **~15 s ထပ်** ကြာနိုင်တယ်။ `port_busy()` ရဲ့ `live_stop.is_some()` (`:56`) က
   အဲ့ window အတွက်။ Field log မှာလည် `18:47:07.303 Live stop requested` →
   `18:47:15.819` delete = **~8.5 s** ကြာခဲ့တယ်

### B.3 ဖြေရှင်းချက် အကြံ — live worker တစ်ခုစီကို **command mailbox** ပေးတာ

**သဘော:** delete request ကို port ဖွင့်ပြီး လုပ်တာ မဟုတ်ဘဲ **အဲ့ port ကို ပိုင်ထားတဲ့
worker ဆီ queue လုပ်ပေးလိုက်တာ**။ Worker က `+CMTI` poll တွေ ကြားထဲမှာ `AT+CMGD` +
ရှိပြီးသား `modem::delete_confirmed` ကို **သူ ကိုင်ထားတဲ့ `AtChannel` ပေါ်** run ပြီး
`OpResult` ပြန်ပို့တယ်။

**ဘယ်နေရာ ထည့်ရမလဲ:** monitoring loop `core/live.rs:342`–`:385` — အခု `queue`
(`:340`) ကနေ `+CMTI` index ဆွဲတာ (`:343`)၊ မရှိရင် `ch.wait_notification(500)`
(`:347`) နဲ့ ၅၀၀ ms စောင့်တာ။ Mailbox check က အဲ့ဒီ ၂ ခု ကြားထဲ ဝင်ရမယ်။

**ဒီ design ရဲ့ merit ၃ ချက်:**
- **lock အသစ် မလိုဘူး** — worker က သူ့ channel ကို ပိုင်ထားပြီးသား၊ mailbox က
  `mpsc::Receiver` တစ်ခုပဲ။ Channel ကို thread ၂ ခု မထိဘူး
- **confirmation path ကို ပြောင်းစရာ မလိုဘူး** — `modem::delete_confirmed`
  (`src-tauri/src/core/modem.rs:477`) က already-open channel ကို လက်ခံတယ်၊ ပြီးတော့
  `live::sweep_expired` (`core/live.rs:443`၊ ခေါ်တာ `:463`) က **ဒီ pattern ကို
  လုပ်ပြနေပြီးသား precedent** — live worker က သူ့ channel ပေါ် confirmed delete
  လုပ်တာ v1.5.0 ကတည်းက ရှိတယ်။ ကျန်တာက "operator ရဲ့ request ကို အဲ့ဒီ နေရာ ရောက်စေတာ" ပဲ
- **`Clear All` နဲ့ `Get SIM Numbers` ပါ တစ်ပြိုင်နက် ပြေတယ်** — mailbox က
  message type ၃ မျိုး ကိုင်ရုံပဲ

**Risk:** live loop ရဲ့ **timing ပြောင်းတယ်** — `AT+CMGD` + confirming `AT+CMGL`
(15 s timeout) က `+CMTI` poll ကို ဆွဲထားနိုင်တယ်၊ ဆိုတော့ mailbox item တစ်ခု ကိုင်နေစဉ်
notification queue မှာ backlog တင်နိုင်တယ်။ ဒါကြောင့်:
- ဒါက **ကိုယ်ပိုင် `Memory/03` case entry ရထိုက်တဲ့ change** (symptom → root cause → fix)
- `04 §G` **Hardware Live-Check Playbook ကို run ရမယ်** — bank ချိတ်မထားဘဲ merge မလုပ်ရ။
  A.2 က မစမ်းရသေးတဲ့ combination (live + concat ၂ slot) က ဒီ playbook run ရဲ့ step တစ်ခု
- Backend gate ကို ဖြုတ်တဲ့အခါ `port_busy()` က `live_on` ကို စစ်တာ **ဖျက်ပစ်လို့ မရဘူး** —
  mailbox route ရှိတဲ့ command တွေအတွက်ပဲ ကျော်ရမယ် (scan က port ကို ကိုယ်တိုင် ဖွင့်တာမို့
  ဆက် ပိတ်ထားရမယ်)

**Version framing:** capability အသစ် → `feat:` commit → **v1.6.0**။

---

## C. ISSUE 2 — `SIM cleanup done. Deleted 14 | FAILED: 30/64` က လွဲမှားစေတယ်

- **Field evidence:** Detect မလုပ်ခင် **port 64 လုံးအပေါ်** cleanup run ခဲ့တယ်။ အထဲက
  **၃၀ လုံးက modem လုံးဝ မရှိဘူး** (Detect က နောက်ပိုင်း `34/64` လို့ ပြတယ်)၊ တစ်ခုချင်းစီ
  အတွက် `Modem not responding` log တင်ခဲ့တယ်။ **တကယ့် failure က သုည** — ဒါပေမဲ့ status
  line က `FAILED: 30/64`။ Bank ရှေ့မှာ ရပ်ပြီး incident debug လုပ်နေတဲ့ operator အတွက်
  ဒါက **အလွန် ထိတ်လန့်စေတဲ့ line** — ဘာမှ မပျက်ပါဘဲ ၃၀ ခု ပျက်တယ် လို့ ပြောနေတာ
- **Root Cause:** cleanup worker ရဲ့ failure counter (`src-tauri/src/commands/mod.rs:1591`–
  `:1598`) က **`expire_old` ရဲ့ non-ok result တိုင်း** `failed` ကို တိုးတယ် (`:1592`)၊
  panic arm (`:1599`–`:1602`) နဲ့ **တူတူ** — ဆိုတော့ modem မရှိတဲ့ ဗလာ slot က
  genuine failure နဲ့ ခွဲမရဘူး။ `modem::expire_old` (`src-tauri/src/core/modem.rs:874`) က
  `read_port` မ ok ရင် error ကို အတိုင်း ပြန်ပေးတယ် (`:876`–`:883`)၊ probe silence က
  `NOT_RESPONDING` ဖြစ်တာမို့ ဒါက "no modem" ဆိုတာ **သိပြီးသား** — counter ကပဲ မခွဲတာ။
  ပြီးတော့ status line က အဲ့ single number ကနေ တည်တယ် (`commands/mod.rs:1618`–`:1622`၊
  event payload `:1627`–`:1630` လည် `deleted`/`failed` ၂ ခုပဲ)
- **နှိုင်းယှဉ်ဖို့:** `detect_ports` က #19 ကတည်းက `Empty` နဲ့ `Inconclusive` ကို ခွဲပြီး
  `30 port(s) with no modem deselected` လို့ **တိတိကျကျ** ပြောတယ်
  (`detect_done_status` `commands/mod.rs:514`၊ verdict enum `:306`)။ ဆိုတော့ ဒီ project မှာ
  **မှန်တဲ့ ပုံစံ ရှိပြီးသား** — cleanup က အဲ့ဒီ ပုံစံကို မလိုက်တာပဲ
- **ဒါက `05 §C.10` (supervisor ၄ ခု၊ panic/failure accounting policy ၄ မျိုး) ရဲ့
  field မှာ ပထမဆုံး မြင်လာတဲ့ symptom** — အဲ့ဒီ entry ရဲ့ table က cleanup ကို
  "failure counter တိုးတယ်" လို့ မှတ်ထားတာ တကယ် operator ဆီ ရောက်လာတာ ဒါ ပထမဆုံး။
  ဆိုတော့ ဒါကို ပြင်တာ **detour မဟုတ်ဘူး၊ C.10 အတွက် down payment** — policy ကို
  per-command ခွဲထားရမယ် ဆိုတဲ့ အချက်ကို ခိုင်စေတယ်
- **အကြံ (shape):** detect လုပ်သလို "no modem" ကို "failed" ကနေ ခွဲပါ —
  ဥပမာ `SIM cleanup done. Deleted 14  |  30 empty  |  FAILED: 0`။ Counter တစ်ခု ထပ်တိုးတာ
  (`empty`) + `probe_failure`/`NOT_RESPONDING` ကို စစ်တာ + status line wording —
  **behaviour ပြောင်းတာ မဟုတ်ဘူး** ဆိုတော့ `fix:` commit → **v1.5.1**။
  `sim_cleanup:done` payload ထဲလည် `empty` ထည့်ပေးရင် UI က ကိုက်ညီစွာ toast လုပ်နိုင်မယ်

---

## D. သေးငယ်တဲ့ item ၂ ခု — **v1.5.1 တစ်ခုတည်းထဲ**

### D.1 `msg(s)` ဆိုတဲ့ unit တစ်လုံးက အရာ ၂ မျိုးကို ကိုယ်စားပြုနေတယ်

Field log မှာ ဒီလို မြင်ရတယ် — ကိန်းဂဏန်း မကိုက်တဲ့ ပုံပေါက်တယ်:

```
COM39: pdu-mode read -> 2 msg(s)
COM39: deleted 5 msg(s)
```

- **Read side (`src-tauri/src/core/modem.rs:379`၊ text-mode `:400`):** `msgs.len()` က
  **reassemble ပြီးတဲ့ row အရေအတွက်** — concat fragment တွေကို `Reassembler` က
  တစ်ခုတည်း အဖြစ် ပေါင်းပြီးမှ count တာ (`:366`–`:378`)
- **Delete side (`src-tauri/src/core/modem.rs:543`၊ partial-failure form `:551`–`:557`၊
  confirm မရတဲ့ form `:524`–`:528`):** `gone.len()` က **SIM slot အရေအတွက်** — row ၂ ခုက
  fragment ၅ ခု ဖြစ်နိုင်တယ်
- ၂ ခုလုံး `msg(s)` လို့ print တာမို့ `2` ကနေ `5` ဖြစ်တာ error လို ဖတ်ရတယ် —
  တကယ်က မှန်တယ်။ Status line ကတော့ ခွဲထားပြီးသား ဖြစ်တာ သတိထားပါ:
  `Deleted 1 message(s) (1 SIM slot(s) freed)` (`commands/mod.rs:1436`) က unit ၂ ခု
  ၂ မျိုး ရေးထားတယ် — **per-port log line ကပဲ ကျန်နေတာ**
- **အကြံ:** delete line တွေကို `slot(s)` လို့ ပြောင်း (`deleted {} slot(s)`)။
  Wording ပဲ ဖြစ်လို့ `fix:`။ **README sync:** ဒီ log format တွေ (`… read -> N msg(s)`၊
  `deleted N msg(s)`၊ `SIM cleanup done…`) က README.md ထဲ **မပါဘူး** (grep ပြီး —
  README က probe timeout / timeout-chain table / AT flow ကိုပဲ ကိုင်ထားတယ်) ဆိုတော့
  §C နဲ့ §D ၂ ခုလုံးအတွက် README ပြောင်းစရာ မရှိဘူး — ဒါပေမဲ့ ပြောင်းမယ့်အချိန်မှာ
  ပြန်စစ်ပါ (AGENTS.md Documentation duty)

### D.2 USSD rejection warning က **code** ကိုပဲ ပြတယ် — **command** ကို မပြဘူး

```
18:39:31.846  COM38: USSD *88# rejected (+CME ERROR: 4)
18:39:31.861  COM38: USSD *88# rejected (+CME ERROR: 4)
18:39:36.585  (success on the *124# fallback)
```

- 15 ms အကွာမှာ **တစ်လုံးမကွာ တူတဲ့ line ၂ ခု** — duplicate log လို ဖတ်ရတယ်။
  တကယ်က **တကယ် မတူတဲ့ attempt ၂ ခု**: `AT+CUSD=1,"*88#",15` ပထမ၊ ပြီးရင်
  `AT+CUSD=2` နဲ့ session ဖျက်ပြီး **bare form `AT+CUSD=1,"*88#"`** retry
  (`src-tauri/src/core/modem.rs:804` ပထမ attempt၊ `Rejected` arm `:807`၊
  `AT+CUSD=2` `:808`၊ bare retry `:809`)
- **Root cause:** `ussd_attempt` (`:817`) က signature မှာ `command` နဲ့ `code`
  ၂ ခုလုံး လက်ခံပေမဲ့ log မှာ **`code` ကိုပဲ** ရေးတယ် (`:824`)၊ `command` ကို မရေးဘူး။
  `no reply within {}s` line (`:833`–`:838`) နဲ့ `replied without a number` (`:853`)
  လည် အတူတူ
- **ဘာလို့ အရေးကြီးလဲ:** `,15` (DCS argument) ကို ငြင်းတဲ့ firmware ကို ရှာဖွေတာက
  ဒီ retry ရဲ့ တစ်ခုတည်းသော ရည်ရွယ်ချက် (comment `:800`–`:803`) — ဒါပေမဲ့ log ကနေ
  **ဘယ် form ငြင်းခဲ့လဲ ခွဲမရဘူး** ဆိုတော့ field မှာ firmware pattern မှတ်လို့ မရဘူး
- **အကြံ:** `,15` ပါလား မပါလား ပါဝင်စေပါ — ဥပမာ `USSD *88# (with ,15) rejected …` /
  `USSD *88# (bare) rejected …`၊ ဒါမှမဟုတ် `command` ကို တိုက်ရိုက် ရေး။ **OTP/subscriber
  number မပါတဲ့ string ပဲ** ဖြစ်တာမို့ Info level မှာ ဘေးမရှိဘူး (`AT+CUSD` ရဲ့ တကယ့်
  reply body က debug မှာပဲ ကျန်ရမယ် — AGENTS.md Logging refusal)။ `fix:` → **v1.5.1**

---

## E. ဒီ plan ထဲ **မပါတာ** (ဆုံးဖြတ်ပြီးသား deferral တွေ — ဒီမှာ ပြန်မရေးဘူး)

| Item | ဘယ်မှာ ရှိလဲ |
|---|---|
| `developer.autoScroll` — ဆုံးဖြတ်ရသေးတဲ့ setting တစ်ခုတည်း | `05 §C.3` |
| L1–L4 limitation ၄ ခု (renamed stick၊ live thread pool မရှိတာ၊ `Closed` over-count၊ liveness re-probe မရှိတာ) | `05 §C.4`–`§C.7` |
| Supervisor ၄ ခုကို `run_port_pool` အဖြစ် ပေါင်းတာ — **အစဉ်လိုက် အနောက်ဆုံး** (§C က ဒီ entry ရဲ့ down payment) | `05 §C.10` |
| `main` ruleset မရှိတာ + updater release-draft flip | `02 §6` |

---

## 🎯 Release Shape — အနှစ်ချုပ်

| Release | ပါဝင်တာ | Commit type | Risk | Hardware လိုလား |
|---|---|---|---|---|
| **v1.5.1** | §C (cleanup status line က "no modem" ကို failure အဖြစ် ရေတာ) + §D.1 (`msg(s)` unit ၂ မျိုး) + §D.2 (USSD rejection line) | အားလုံး `fix:` | **နိမ့်** — counter/wording ပဲ၊ **behaviour ပြောင်းတာ မရှိဘူး** | မလို (unit test နဲ့ လုံလောက်) |
| **v1.6.0** | §B (live worker command mailbox — Delete / Clear All / Get SIM Numbers ကို live ဖွင့်ထားစဉ် လုပ်နိုင်တာ) | `feat:` | **မြင့်** — live loop timing ပြောင်းတယ် | **လိုတယ် — `04 §G` playbook**၊ ပြီးတော့ `Memory/03` case entry အသစ် |

> **အစဉ်လိုက်:** v1.5.1 ကို အရင် ထုတ်ပါ။ ဒါက risk နိမ့်ပြီး operator ရဲ့ status line ကို
> ချက်ချင်း ယုံကြည်လို့ ရစေတယ် — §B ရဲ့ ကြီးမားတဲ့ change ကို debug လုပ်တဲ့အခါ
> **status line ကို ယုံနိုင်တာက ကိရိယာ** ဖြစ်လာမယ်။
