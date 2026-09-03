# 📨 Telegram Forwarding — Stage 1 + Stage 2 (implementation record)

> **ရေးတဲ့ရက်:** 2026-09-03 · **အခြေအနေ:** Stage 1 **ပြီး၊ hardware အတည်ပြီး** ·
> Stage 2 **ပြီး — OTP အစစ် group ထဲ ရောက်တာ အတည်ပြီး**၊ ကျန် test ၃ ခု (§F)
> Roadmap entry က `05 §၁.၁`၊ dependency trap က `03 §18`။
>
> **⚠️ ဒီ doc က plan ကနေ record ဖြစ်သွားပြီ။** §B/§C က ဘာလို့ ဒီလို ရေးလိုက်တာလဲ
> ဆိုတဲ့ အကြောင်းရင်းတွေ ဖြစ်တယ် — implementation က အဲဒီအတိုင်း ပြီးသွားပြီ။
> §E (မလုပ်ရတာ) က **အသက်ဝင်နေတုန်း** ဖြစ်တယ်။

## Stage 2 — ရေးပြီးသွားတဲ့ file တွေ

| File | ပါဝင်တာ |
|---|---|
| `src-tauri/src/forwarder.rs` **(အသစ်)** | `ForwardItem` · `ForwarderHandle::deliver` (non-blocking, filter, `MAX_QUEUE`=500 oldest-drop) · `Forwarder::shutdown` (final flush) · `take_work` / `requeue` (coalescing + edit boundary, pure) · `format_one` / `format_batch` / `clip` / `origin` · sender thread (`MIN_INTERVAL`=3.5s, `MAX_BATCH`=10, 429 honour, migration heal, `catch_unwind`) · test ၁၅ |
| `src-tauri/src/telegram.rs` | `edit_message` ထပ်ထည့် (`message is not modified` = success) |
| `src-tauri/src/commands/telegram.rs` | `ForwardingConfigDto` + `split()` |
| `src-tauri/src/commands/mod.rs` | `start_live(retention_hours, forwarding)` · supervisor thread မှာ forwarder start/shutdown · `Sms` arm **၂ ခုလုံး** `deliver` |
| `src/lib/types.ts` | `forwarding.{enabled, forwardOtp, forwardNonOtp}` ထပ်ထည့် |
| `src/lib/services/api.ts` | `forwardingArgs()` · `startLive` က ပို့တာ · `forward:failed` / `forward:migrated` listener |
| `src/lib/pages/Settings.svelte` | switch ၃ ခု (§D အတိုင်း) |

**Validation:** Rust test **202** · frontend test 89 · clippy `-D warnings` သန့် ·
`--locked` release check မှန် · `npm run check` 0/0。

---

## A. Stage 1 — ရှိပြီးသား (ဒါတွေကို ပြန်မရေးရ)

| File | ပါဝင်တာ |
|---|---|
| `src-tauri/src/telegram.rs` | `TelegramConfig` · `build_client` (SOCKS5 + `ensure_crypto_provider`) · `send_message` (→ `message_id` ပြန်တယ်) · `get_me` · `detect_group` / `pick_group` · `SendError::{Migrated, RateLimited, Other}` · `interpret` · `redact` · `escape_html` · `test_message_html` · test ၂၀ |
| `src-tauri/src/commands/telegram.rs` | command ၃ ခု: `verify_telegram_token` · `detect_telegram_group` · `send_telegram_test` (migration auto-heal) · `require_token` · `host_label` · test ၅ |
| `src/lib/types.ts` | `settings.forwarding = { botToken, chatId, proxyUrl }` |
| `src/lib/stores/settings.svelte.ts` | `forwarding` getter + `setForwarding` |
| `src/lib/pages/Settings.svelte` | `forwarding` group · `type: "text"` / `"secret"` branch · `pendingAction` loading state |
| `src/lib/services/api.ts` | `verifyTelegramToken` · `detectTelegramGroup` · `sendTelegramTest` |
| `src/lib/utils/telegram-preview.ts` | browser-preview parity + test ၁၀ |

**အရေးကြီးတဲ့ အချက်:** `send_message` က Telegram ရဲ့ **`message_id` ကို ပြန်ပေးတယ်**။
Stage 1 မှာ အဲဒါ ကို မသုံးဘူး — Stage 2 ရဲ့ `editMessageText` အတွက် တမင် ထားခဲ့တာ (§B.1)။

---

## B. Stage 2 ရဲ့ အဓိက အခက်အခဲ ၄ ခု

### B.1 Hook point — `sms:new` **တစ်ခုတည်း မရဘူး** (အရေးကြီးဆုံး)

`commands/mod.rs` ရဲ့ live event handler မှာ arm ၂ ခု ရှိတယ်၊ နှစ်ခုလုံး message ထုတ်တယ်:

| Arm | file:line | ဘယ် event ထွက်လဲ | forward လုပ်ရမလား |
|---|---|---|---|
| `LiveEvent::Batch` | `mod.rs:1087` | `messages:added` (`mod.rs:1104`) | **မလုပ်ရ** — live စတဲ့အခါ SIM ထဲ ရှိပြီးသား inbox တစ်ခုလုံး ဖြစ်တယ်။ Forward လုပ်ရင် group ထဲ အရင် message အားလုံး ပုံချမယ် |
| `LiveEvent::Sms` → `None` arm | `mod.rs:1171` | `sms:new` | **လုပ်ရမယ်** |
| `LiveEvent::Sms` → `Some` arm | `mod.rs:1163` | `messages:updated` | **လုပ်ရမယ် — `editMessageText`** |

`Some` arm ဆိုတာ ဘာလဲ: `mod.rs:1137-1146` မှာ prefix match ရှာတယ် —
`port` + `from` + `received` တူပြီး `message.text.starts_with(&it.message.text)` ဆိုရင်
**row အသစ် မထည့်ဘူး၊ ရှိတာကို အစားထိုးတယ်**။ ဆိုတော့ `sms:new` **မထွက်ဘူး**။

**ဘယ်တော့ ဖြစ်လဲ:** concat SMS။ `live.rs:268` က `asm.peek_partials()` နဲ့ fragment ကို
အရင် ပြတယ်၊ `live.rs:344` `handle_cmgr` က ကျန် part တွေ ရောက်လာတဲ့အခါ ပြည့်စုံတာ ထုတ်တယ်။
**မြန်မာစာက UCS-2 မှာ part တစ်ခု ၇၀ လုံးပဲ ဆံ့တာမို့ ဒါ ရှားတဲ့ case မဟုတ်ဘူး။**

`sms:new` တစ်ခုတည်း hook လုပ်ရင် ဒီ case မှာ **ဘာမှ မပို့ဖြစ်ဘူး** — ဒါ ဒီ feature ရဲ့
အဓိက silent-failure mode ပါ။

**ဖြေရှင်းချက်:**
1. `Sms` arm ရဲ့ **branch ၂ ခုလုံး** မှာ forwarder ကို ပို့ပါ။ `None` → `sendMessage`၊
   `Some` → **`editMessageText`** (`message_id` ကို ပြန်သုံး)
2. `SmsItem.id` → Telegram `message_id` map တစ်ခု ထား (`HashMap<u64, i64>`)။ `start_live`
   တစ်ခါစီ ရှင်း (session-scoped)၊ retention purge (`mod.rs:1520`) မှာလည်း လိုက်ရှင်း
3. `Some` arm မှာ map ထဲ id မရှိရင် (queue ထဲ စောင့်နေတုန်း၊ ဒါမှမဟုတ် forward မဖြစ်ခဲ့တာ)
   → **`sendMessage` အသစ် ပို့**၊ error မထုတ်ရ
4. Hook ကို `LiveEvent` layer မှာ **မထားရ** — `extract_otp` (`mod.rs:1095`, `:1118`) နဲ့
   id allocation က command layer မှာ ဖြစ်တယ်။ `live.rs` က OTP ကို မသိဘူး

**Dedup:** `live.rs:579` `fingerprint()` = `from` + `received` millis + `text` hash။
`live.rs:565` `dedup()` က reconnect re-read ကို ဖမ်းတယ်။ ဒါပေမဲ့ **partial ကြီးလာရင်
fingerprint အသစ် ဖြစ်တယ်** (`live.rs:576` comment) — ဒါက တမင်၊ completion ကို ဖြတ်ခွင့်ပေးတာ။
ဆိုတော့ forwarder က fingerprint ကို dedup key အဖြစ် **သုံးလို့ မရဘူး**၊ `SmsItem.id` ကို သုံးပါ။

### B.2 Rate limit — group တစ်ခုကို ၁ မိနစ် message ၂၀

Telegram FAQ: *"In a group, bots are not be able to send more than 20 messages per minute."*

"၃ စက္ကန့်တစ်ခါ ပို့မယ်" = ၂၀/min = limit ကို **အတိအကျ ထိနေတာ၊ အောက် မဟုတ်ဘူး**။
Port ၆၄ ခုက burst တစ်ခါ လာရင် queue ရှည်ပြီး **OTP သက်တမ်းကုန်မှ ရောက်တယ်** —
အဲဒီအခြေအနေမှာ feature တစ်ခုလုံး အလုပ်မလုပ်တာနဲ့ တူတယ်။

**ဖြေရှင်းချက် — coalescing:** queue depth တိုးလာရင် OTP အများကို **message တစ်ခုထဲ ပေါင်း**။
Limit က *message* ကို ရေတာမို့ throughput က ချက်ချင်း အဆမြောက် တက်တယ်။
Principle က `src/lib/utils/toast-queue.ts` နဲ့ တူတယ် (port ၁၆ ခု တစ်ခါတည်း fail တဲ့အခါ
card ၁၆ ခု မထပ်ဘဲ ပေါင်းတာ) — ဒါက ဒီ repo ရဲ့ ရှိပြီးသား pattern။

`SendError::RateLimited(secs)` ရှိပြီးသား (`telegram.rs`) — `retry_after` ကို honor လုပ်ပါ။

### B.3 Thread model — live worker thread ထဲကနေ HTTP **မခေါ်ရ**

Live worker က port ကို exclusive ကိုင်ထားတယ် (`live.rs:342` monitoring loop)။
HTTP timeout က `TIMEOUT = 15s` — worker ကို ၁၅ စက္ကန့် block လုပ်ရင် `+CMTI` notification
တွေ လွတ်မယ်၊ ပြီးတော့ `SIM_SWEEP_EVERY` (`live.rs:67`, ၆၀၀ စက္ကန့်) cadence ပါ ရွေ့မယ်။

**ဖြေရှင်းချက်:** forwarder thread **သီးသန့် ၁ ခု** + `std::sync::mpsc` channel။
Command layer က `tx.send(...)` (non-blocking) လုပ်ရုံ။ Thread ထဲမှာ pacing + coalescing +
retry။ `catch_unwind` နဲ့ ဝိုင်းပါ (ဒီ repo ရဲ့ per-worker pattern)၊ ပြီးတော့ panic ဖြစ်ရင်
`live_error` ထဲ **မထည့်ရ** — အဲဒီ field က "ဒီ port က monitoring မလုပ်တော့ဘူး" လို့
အဓိပ္ပာယ်ရတယ် (AGENTS.md backend invariants)။ Global forwarder status + toast သုံးပါ။

### B.4 Config lifetime

`retentionHours` နဲ့ **တူတဲ့ လမ်း**: `start_live` ရဲ့ argument အဖြစ် ပို့ (`api.ts:469` နဲ့
`mod.rs:934-940` က precedent)။ Rust ဘက် config file အသစ် **မလုပ်ရ** —
`core/sim_directory.rs:3-5` က invariant ရေးထားတယ်: *"User preferences live in exactly one
place: the frontend settings store."*

လက်ခံရမယ့် အချက်: live ဖွင့်ပြီးမှ token/chat_id ပြောင်းရင် Stop → Start လိုမယ်။
ဒါ ရိုးရှင်းတယ်၊ ပြီးတော့ `retentionHours` က ဒီအတိုင်းပဲ။ UI မှာ ရေးထားပါ။

---

## C. ရေးရမယ့် အစဉ်လိုက် (၆ ဆင့်)

> အဆင့်တိုင်းအဆုံးမှာ validation ၆ ခု ပြေးပါ (`AGENTS.md` "Validation" section)။

1. **`telegram.rs` မှာ `edit_message` ထည့်** — `send_message` ရဲ့ အတူတူ shape၊ payload မှာ
   `message_id` ပါတယ်။ Test: `interpret` ကို ပြန်သုံးတာမို့ error taxonomy test မလိုဘူး၊
   payload shape test တစ်ခုပဲ
2. **Message formatter** — `format_sms_html(item)` နဲ့ `format_batch_html(&[item])`။
   `escape_html` ကို body/sender/port အားလုံးမှာ သုံးရမယ်။ OTP ကို `<code>` (tap-to-copy)။
   Sender ကို `logging::mask_number` နဲ့ mask လုပ်မလား **ဆုံးဖြတ်ရမယ်** — group က
   trusted audience မို့ mask မလုပ်တာ သင့်တယ်၊ ဒါပေမဲ့ log နဲ့ မတူတာကို doc မှာ ရေးပါ။
   Test: rune-free pure function မို့ unit test လွယ်တယ် (Myanmar text + markdown metachar case)
3. **Forwarder thread + queue** (`src-tauri/src/forwarder.rs` အသစ်) —
   `mpsc::Receiver<ForwardJob>` · pacing · coalescing · `RateLimited` honor · `Migrated`
   auto-heal (chat_id အသစ်ကို event နဲ့ frontend ဆီ ပြန်ပို့ပြီး save ခိုင်း)。
   **Queue depth ကို ဘောင်ခတ်ပါ** — unbounded ဆိုရင် network ပြတ်နေချိန် RAM တိုးမယ်။
   Test: pacing/coalescing decision function ကို channel မလိုဘဲ pure function အဖြစ် ခွဲရေး
4. **`start_live` မှာ wire** — `telegram: Option<TelegramConfig>` argument၊ forwarder thread
   spawn၊ `Sms` arm branch ၂ ခုလုံးမှာ `tx.send`။ `stop_live` မှာ channel drop + thread join
5. **Switch ၃ ခု + UI** (§D)
6. **README + `03` case entry (bug တွေ့ရင်) + `05 §၁.၁` update**

---

## D. Switch ၃ ခု — Stage 2 နဲ့ **အတူတူပဲ** ထည့်ရမယ်

`04 §H` (inert-control rule): control ကို တကယ့် behaviour နဲ့ တူတဲ့ change ထဲ wire ရမယ်၊
မဟုတ်ရင် လုံးဝ မထည့်ရ။ Stage 1 မှာ ဒီ ၃ ခုကို **တမင် မထည့်ခဲ့တာ**။

| Field | Default | Wire ဖြစ်ရမယ့် နေရာ |
|---|---|---|
| `forwarding.enabled` | `false` | `start_live` က `None` ပို့မလား ဆုံးဖြတ်တာ |
| `forwarding.forwardOtp` | `true` | Forwarder ရဲ့ filter (`item.otp.is_some()`) |
| `forwarding.forwardNonOtp` | **`false`** | ကြော်ငြာ SMS နဲ့ ၂၀/min ဘောင် ချက်ချင်း ပြည့်မယ် |

`deepMerge` က stored key တွေကို iterate တာမို့ (`03 §T1`) field အသစ်က ရှိပြီးသား profile မှာ
default ကနေ ရမယ် — migration မလိုဘူး။

---

## E. မလုပ်ရတာ (ဆုံးဖြတ်ပြီးသား — ပြန်မဆွေးနွေးရ)

| မလုပ်ရတာ | ဘာလို့ |
|---|---|
| `sms:new` **တစ်ခုတည်း** hook | §B.1 — concat/မြန်မာစာ message တွေ တိတ်တဆိတ် ပျောက်တယ် |
| `parse_mode: Markdown` | Body က attacker-controlled။ `*`/`_`/backtick ပါရင် `400 can't parse entities` → OTP တိတ်တဆိတ် မရောက်ဘူး။ HTML + `escape_html` ကို သုံးပါ |
| Live worker thread ထဲ HTTP call | §B.3 — `+CMTI` လွတ်မယ် |
| Rust ဘက် config file / token persistence | `sim_directory.rs:3-5` invariant · `start_live` argument ကို သုံးပါ |
| Forwarder error ကို `live_error` ထဲ ထည့်တာ | အဲဒီ field က port monitoring အခြေအနေ ကိုယ်စားပြုတယ် |
| Log ထဲ raw HTTP error | Token က URL path ထဲ ပါတယ် (`03 §18`) — `redact` ကို အမြဲ ဖြတ်ပါ |
| `reqwest` ရဲ့ default features ပြန်ဖွင့်တာ | `03 §18` — crate ၂၂ ခု + cmake/C toolchain |
| Unbounded queue | Network ပြတ်ချိန် RAM တိုးမယ် |
| Tray မရှိဘဲ "PC ရှေ့ မရှိရင်တောင် ရမယ်" လို့ ကြေညာတာ | Window ပိတ်ရင် app ပြီးသွားတယ်။ Tray က ဒီ ကတိရဲ့ prerequisite (`05 §A` — `minimizeToTray` ကို tray code မရှိလို့ ဖျက်ခဲ့တာ) |

---

## F. Hardware test — ✅ ၄ ခုလုံး အတည်ပြီး (2026-09-03 ည)

| # | စမ်းရမယ့်အရာ | အခြေအနေ |
|---|---|---|
| 1 | concat OTP — group ထဲ bubble **၁ ခုပဲ** | ✅ **အတည်ပြီး** — 21:59:26→34 မှာ part **၄ ခု** (`idx 4,5,6,7 [concat]`)၊ `NEW SMS` **၁ ခေါက်ပဲ**၊ Telegram မှာ မြန်မာစာ အပြည့်အစုံ bubble **၁ ခု** |
| 2 | Live စတဲ့အခါ SIM ထဲ ရှိပြီးသား message **မ forward ဖြစ်ရ** (`Batch` arm) | ✅ **အတည်ပြီး** (20:48 run) |
| 3 | Burst — coalescing | ✅ **အတည်ပြီး** — 22:01:31 + 22:01:44 OTP ၂ ခု outage အတွင်း queue ဝင်ပြီး 22:02 မှာ `🔐 2 new messages` bubble **တစ်ခုထဲ** ရောက်တယ် |
| 4 | Network ပြတ်ပြီး ပြန်လာတဲ့အခါ queue ဆက်ပို့တာ | ✅ **အတည်ပြီး** — 21:59:02 (`retrying in 5s` → 21:59 မှာ `145299` ရောက်တယ်) နဲ့ 22:01:46→22:01:55→22:02:09 (`5s → 10s → 20s` exponential backoff၊ ပြီးမှ ရောက်တယ်) |

**Multi-SIM ပါ အတည်ပြီး:** coalesced bubble မှာ `09671973972` (ttyUSB14) ပြတယ် —
အရင် run တွေက `09671312573` (ttyUSB47)。

---

## G. Field test မှာ တွေ့တဲ့ OTP false positive ၂ ခု — ✅ **ပြင်ပြီး** (`03 §21`, `03 §22`)

**၁။ `2026` ကို OTP လို့ ဖတ်ခဲ့တာ။** 21:59 မှာ MyID ရဲ့ **login notification** (OTP message
မဟုတ်ဘူး) ကို forward လုပ်ပြီး OTP badge မှာ `2026` ပြခဲ့တယ် — message ထဲက
**ရက်စွဲ `2026/09/03` ရဲ့ ခုနှစ်** ပါ။

**Fix:** `decoder::in_date_or_time()` guard ထည့်လိုက်တယ် — separator (`/ : - .`) ရဲ့
တစ်ဖက်မှာ digit ၁-၂ လုံး field ရှိရင် အဲဒီ run က ရက်စွဲ/အချိန် field ဖြစ်တယ်၊ OTP
မဟုတ်ဘူး။ `extract_otp` က `captures_iter()` သုံးတာမို့ ရှေ့မှာ ရက်စွဲ ရှိရင် နောက်မှာ
ရှိတဲ့ တကယ့် code ကို ဖျောက်မပစ်ဘူး။ Gate ရော cascade ရော **မထိထားဘူး**
(`05 §B.1` hard refusal)。Test ၄ ခု။ အသေးစိတ် `03 §21`。

**၂။ `3211` ကို OTP လို့ ဖတ်ခဲ့တာ (v1.6.1)。** နောက်တစ်ည 01:12 မှာ KBZPay ရဲ့ **logout
notification** ကို forward လုပ်ပြီး OTP badge မှာ `3211` ပြခဲ့တယ် — အဲဒါ **KBZPay Call
Center နံပါတ်** ပါ။ Gate ကို ဖွင့်ခဲ့တာ message ကိုယ်တိုင်ရဲ့ "employees will never ask
for your OTP, PIN or NRC" သတိပေးစာ ဖြစ်တယ် — §21 နဲ့ **shape တူတူပဲ**。

**Fix:** `decoder::after_phone_label()` guard (guard #၂) — number ရဲ့ ရှေ့ text က phone
label (call center · hotline · helpline · customer service · contact · call · dial · tel ·
phone · **ဖုန်း**) နဲ့ **အဆုံးသတ်ရင်** အဲဒီ run က ဖုန်းနံပါတ် ဖြစ်တယ်။ Window မဟုတ်ဘဲ
**suffix** match ဖြစ်တာနဲ့ `.`/`,` ကို glue မလုပ်တာက false negative ကို ကာတယ်။ Cascade
ကို ဒီတစ်ခါလည်း မထိဘူး。Test ၄ ခု။ အသေးစိတ် `03 §22`。

