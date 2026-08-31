use crate::core::models::SmsMessage;
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use regex::Regex;
use std::sync::LazyLock;

const KW_KODE: &str = "\u{1000}\u{102F}\u{1012}\u{103A}";
const KW_CONFIRM: &str = "\u{1021}\u{1010}\u{100A}\u{103A}\u{1015}\u{103C}\u{102F}";
const KW_SECURE: &str = "\u{101C}\u{102F}\u{1036}\u{1001}\u{103C}\u{102F}\u{1036}";
const KW_IS: &str = "\u{1016}\u{103C}\u{1005}\u{103A}";

static KEYWORD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i)(otp|one.?time|code|pin|{}|verification|verify|confirm|{}|{})",
        KW_KODE, KW_CONFIRM, KW_SECURE
    ))
    .unwrap()
});
static P1: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i)(?:otp|one.?time|code|pin|{}|verification)[^0-9]{{0,24}}([0-9]{{4,8}})",
        KW_KODE
    ))
    .unwrap()
});
static P2: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i)([0-9]{{4,8}})[^0-9]{{0,8}}(?:is|as your|{})",
        KW_IS
    ))
    .unwrap()
});
static P3: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\b([0-9]{6})\b").unwrap());
static P4: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\b([0-9]{4,8})\b").unwrap());
static HEX_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[0-9A-Fa-f]+$").unwrap());
static CMGL_HEADER_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\+CMGL:\s*(.+)$").unwrap());
static CMGR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\+CMGR:\s*(.+)$").unwrap());
static CMTI_IDX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\+CMTI:\s*"[^"]*",\s*(\d+)"#).unwrap());
static CUSD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\+CUSD:\s*(\d+),"([^"]*)"(?:,(\d+))?"#).unwrap());
static NUMBER_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:959|09)\d{8,10}").unwrap());
static CNUM_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\+CNUM:\s*(.+)$").unwrap());
static ICCID_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d{18,20}[Ff]?").unwrap());
static DATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(\d{2,4})/(\d{1,2})/(\d{1,2})[ ,](\d{1,2}):(\d{1,2}):(\d{1,2})(?:\s*([+-])(\d{1,2}))?",
    )
    .unwrap()
});
static PDU_DATA_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[0-9A-Fa-f]{8,}$").unwrap());

// ── Concatenated SMS (UDH) ──

/// Concat info carried in the User Data Header of a SMS-DELIVER PDU
/// (IEI 0x00 = 8-bit reference, IEI 0x08 = 16-bit reference).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConcatInfo {
    pub ref_num: u16,
    pub total: u8,
    pub seq: u8,
}

/// Result of decoding a SMS-DELIVER PDU: the message itself plus the
/// concatenation header when this message is a fragment of a long SMS.
#[derive(Debug, Clone)]
pub struct DeliverInfo {
    pub message: SmsMessage,
    pub concat: Option<ConcatInfo>,
}

fn extract_concat_from_udh(udh: &[u8]) -> Option<ConcatInfo> {
    let mut pos = 0usize;
    while pos + 2 <= udh.len() {
        let iei = udh[pos];
        let len = udh[pos + 1] as usize;
        // Every length in here is attacker/firmware controlled, so the element
        // body is taken with `get`: an element that claims more bytes than the
        // header carries means the rest of the header is unparseable, and there
        // is nothing to scan past it.
        let Some(data) = udh.get(pos + 2..pos + 2 + len) else {
            break;
        };
        match (iei, len) {
            (0x00, l) if l >= 3 => {
                return Some(ConcatInfo {
                    ref_num: data[0] as u16,
                    total: data[1],
                    seq: data[2],
                });
            }
            (0x08, l) if l >= 4 => {
                return Some(ConcatInfo {
                    ref_num: ((data[0] as u16) << 8) | data[1] as u16,
                    total: data[2],
                    seq: data[3],
                });
            }
            // Anything else (including a zero-length element, which is legal
            // padding) is skipped: `pos` always advances by at least 2, so this
            // cannot loop. Bailing out here instead used to hide a concat header
            // that merely sat behind an element we do not understand.
            _ => {}
        }
        pos += 2 + len;
    }
    None
}

/// Split the User Data Header off the front of the user data at `i`.
///
/// Returns the concat header (when one is present), the declared UDHL and the
/// offset of the first payload byte — clamped, by construction, to inside
/// `bytes`.
///
/// `None` means the header itself does not fit in the PDU: the UDHL byte is
/// missing, or it claims more bytes than were received. That is the one case
/// where nothing can be salvaged — the payload start is unknown, so there is no
/// text to recover and no trustworthy fragment number to file it under — so the
/// caller rejects the whole PDU. Returning a half-decoded fragment would push an
/// empty part into the reassembler and let a long message "complete" with a hole
/// in it. Truncation *after* the header is a different story and is clamped, not
/// rejected: the surviving prefix often still carries the OTP.
///
/// Bounding this is not cosmetic. `i += 1 + udhl` used to run past the end of
/// the buffer and the following `bytes.len() - i` underflowed, panicking the
/// thread that decoded the PDU (debug) or silently yielding garbage (release).
fn split_udh(bytes: &[u8], i: usize) -> Option<(Option<ConcatInfo>, usize, usize)> {
    let udhl = *bytes.get(i)? as usize;
    let end = i + 1 + udhl;
    if end > bytes.len() {
        return None;
    }
    Some((extract_concat_from_udh(&bytes[i + 1..end]), udhl, end))
}


const GSM7_BASIC: [&str; 128] = [
    "@", "£", "$", "¥", "è", "é", "ù", "ì", "ò", "Ç", "\n", "Ø", "ø", "\r", "Å", "æ", "Æ", "É",
    "Δ", "Φ", "Γ", "Λ", "Ω", "Π", "Ψ", "Σ", "Θ", "Ξ", "Þ", "ß", "É", " ", " ", "!", "\"", "#", "¤",
    "%", "&", "'", "(", ")", "*", "+", ",", "-", ".", "/", "0", "1", "2", "3", "4", "5", "6", "7",
    "8", "9", ":", ";", "<", "=", ">", "?", "¡", "A", "B", "C", "D", "E", "F", "G", "H", "I", "J",
    "K", "L", "M", "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z", "Ä", "Ö", "Ñ",
    "Ü", "§", "¿", "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p",
    "q", "r", "s", "t", "u", "v", "w", "x", "y", "z", "ä", "ö", "ñ", "ü", "à",
];

static GSM7_EXT: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    let mut v = vec![""; 128];
    v[0x0A] = "\u{000C}";
    v[0x14] = "^";
    v[0x28] = "{";
    v[0x29] = "}";
    v[0x2F] = "\\";
    v[0x3C] = "[";
    v[0x3D] = "~";
    v[0x3E] = "]";
    v[0x3F] = "|";
    v[0x65] = "€";
    v
});

// ── OTP ──

pub fn extract_otp(text: &str) -> Option<String> {
    if text.is_empty() {
        return None;
    }
    let normalized = normalize_myanmar_digits(text);
    if !KEYWORD_RE.is_match(&normalized) {
        return None;
    }
    for re in [&*P1, &*P2, &*P3, &*P4] {
        if let Some(cap) = re.captures(&normalized) {
            return cap.get(1).map(|m| m.as_str().to_string());
        }
    }
    None
}

fn normalize_myanmar_digits(text: &str) -> String {
    text.chars()
        .map(|c| {
            if ('\u{1040}'..='\u{1049}').contains(&c) {
                char::from_u32('0' as u32 + (c as u32 - '\u{1040}' as u32)).unwrap_or(c)
            } else {
                c
            }
        })
        .collect()
}

// ── Hex / UCS2 ──

pub fn is_hex(s: &str) -> bool {
    !s.is_empty() && HEX_RE.is_match(s)
}

pub fn decode_hex_or_raw(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    let t = s.trim().trim_matches('"');
    if t.len() >= 4 && t.len().is_multiple_of(4) && is_hex(t) {
        let mut result = String::with_capacity(t.len() / 4);
        let mut chars = t.chars();
        loop {
            let c1 = chars.next();
            let c2 = chars.next();
            let c3 = chars.next();
            let c4 = chars.next();
            if let (Some(c1), Some(c2), Some(c3), Some(c4)) = (c1, c2, c3, c4) {
                let hex_str = format!("{}{}{}{}", c1, c2, c3, c4);
                if let Ok(code) = u32::from_str_radix(&hex_str, 16) {
                    if let Some(ch) = char::from_u32(code) {
                        result.push(ch);
                        continue;
                    }
                }
                result.push_str(&hex_str);
            } else {
                break;
            }
        }
        return result;
    }
    t.to_string()
}

// ── Number ──

pub fn normalize_number(raw: &str) -> String {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    // Only 95xxxxxxxxx needs rewriting; everything else passes through as-is.
    if digits.starts_with("95") && digits.len() >= 11 {
        format!("0{}", &digits[2..])
    } else {
        digits
    }
}

pub fn extract_number_from_ussd(resp: &str) -> Option<String> {
    if let Some(cap) = CUSD_RE.captures(resp) {
        let v = cap.get(2)?.as_str();
        let dcs = cap.get(3).map(|m| m.as_str()).unwrap_or("");
        let decode = match dcs {
            "72" | "48" => true,
            "15" | "0" => false,
            _ => v.len() >= 8 && v.len() % 4 == 0 && is_hex(v),
        };
        let text = if decode {
            decode_hex_or_raw(v)
        } else {
            v.to_string()
        };
        NUMBER_RE.find(&text).map(|m| m.as_str().to_string())
    } else {
        NUMBER_RE.find(resp).map(|m| m.as_str().to_string())
    }
}

/// Pull the subscriber number out of an `AT+CNUM` reply.
///
/// The reply is `+CNUM: "<alpha>","<number>",<type>[,<speed>,<service>]`, one
/// line per record, and EF_MSISDN is optional — a perfectly healthy prepaid SIM
/// often answers a bare `OK`. Only Myanmar-shaped numbers are accepted so that
/// service dialling numbers or a factory-blank field cannot be mistaken for the
/// subscriber's own MSISDN.
pub fn extract_number_from_cnum(resp: &str) -> Option<String> {
    resp.lines()
        .filter_map(|l| CNUM_RE.captures(l.trim()))
        .filter_map(|cap| {
            let fields = split_quoted(cap.get(1)?.as_str());
            // Field 0 is the alphanumeric label, field 1 the number. Some
            // firmware omits the label entirely, so scan both.
            fields
                .iter()
                .take(2)
                .find_map(|f| NUMBER_RE.find(f).map(|m| m.as_str().to_string()))
        })
        .next()
}

/// Pull the ICCID out of a `+CCID:` / `+ICCID:` / `^ICCID:` reply.
///
/// Vendors disagree on the prefix and some answer with the bare digits, so the
/// digit run is what we key on: 19 or 20 characters, occasionally padded with a
/// trailing `F` when the last nibble is unused. The padding is dropped so the
/// same card always produces the same key regardless of which command answered.
pub fn extract_iccid(resp: &str) -> Option<String> {
    for line in resp.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("AT") || is_result_code(line) {
            continue;
        }
        if let Some(m) = ICCID_RE.find(line) {
            let id = m.as_str().trim_end_matches(['F', 'f']).to_string();
            if id.len() >= 18 {
                return Some(id);
            }
        }
    }
    None
}

fn is_result_code(line: &str) -> bool {
    matches!(line, "OK" | "ERROR") || line.starts_with("+CME ERROR") || line.starts_with("+CMS ERROR")
}

// ── CMTI helpers ──

pub fn find_cmti(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| l.contains("+CMTI:"))
        .map(|l| l.trim().to_string())
        .collect()
}

pub fn parse_cmti_index(line: &str) -> Option<i32> {
    CMTI_IDX_RE
        .captures(line)
        .and_then(|cap| cap.get(1)?.as_str().parse().ok())
}

// ── CSV / Date ──

fn split_quoted(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut in_quote = false;
    let mut cur = String::new();
    for c in s.chars() {
        if c == '"' {
            in_quote = !in_quote;
        } else if c == ',' && !in_quote {
            parts.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    parts.push(cur);
    parts
}

fn parse_date(s: &str) -> DateTime<Utc> {
    if let Some(cap) = DATE_RE.captures(s) {
        let mut yy: i32 = cap[1].parse().unwrap_or(0);
        if yy < 100 {
            yy += 2000;
        }
        let mm: u32 = cap[2].parse().unwrap_or(1);
        let dd: u32 = cap[3].parse().unwrap_or(1);
        let hh: u32 = cap[4].parse().unwrap_or(0);
        let mi: u32 = cap[5].parse().unwrap_or(0);
        let ss: u32 = cap[6].parse().unwrap_or(0);
        if let Some(d) = NaiveDate::from_ymd_opt(yy, mm, dd) {
            if let Some(t) = d.and_hms_opt(hh, mi, ss) {
                let mut utc = Utc.from_utc_datetime(&t);
                if let (Some(sign), Some(q)) = (cap.get(7), cap.get(8)) {
                    let quarters: i64 = q.as_str().parse().unwrap_or(0);
                    let minutes = quarters * 15;
                    utc = if sign.as_str() == "-" {
                        utc + Duration::minutes(minutes)
                    } else {
                        utc - Duration::minutes(minutes)
                    };
                }
                return utc;
            }
        }
    }
    DateTime::UNIX_EPOCH
}

// ── Text-mode AT+CMGL ──

fn parse_cmgl_header(line: &str, decode_from: bool) -> (i32, String, String, DateTime<Utc>) {
    let body = line.trim_start_matches("+CMGL:").trim();
    let f = split_quoted(body);
    let idx = f.first().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
    let stat = f.get(1).map(|s| s.trim().to_string()).unwrap_or_default();
    let from = f
        .get(2)
        .map(|s| {
            let s = s.trim();
            if decode_from {
                decode_hex_or_raw(s)
            } else {
                s.to_string()
            }
        })
        .unwrap_or_default();
    let mut date_str = f.last().map(|s| s.trim().to_string()).unwrap_or_default();
    if f.len() > 3 && !date_str.contains('/') {
        if let Some(prev) = f.get(f.len() - 2) {
            date_str = format!("{},{}", prev.trim(), date_str);
        }
    }
    (idx, stat, from, parse_date(&date_str))
}

pub fn parse_text_mode_list(resp: &str, port: &str) -> Vec<SmsMessage> {
    let mut list = Vec::new();
    let mut cur: Option<(i32, String, String, DateTime<Utc>)> = None;
    let mut hex_buf = String::new();

    for raw in resp.replace("\r\n", "\n").lines() {
        let line = raw.trim();
        if let Some(cap) = CMGL_HEADER_RE.captures(line) {
            if let Some((idx, stat, from, received)) = cur.take() {
                list.push(SmsMessage {
                    port: port.to_string(),
                    index: idx,
                    from,
                    received,
                    status: stat,
                    text: decode_hex_or_raw(&hex_buf),
                    part_indices: Vec::new(),
                });
                hex_buf.clear();
            }
            cur = Some(parse_cmgl_header(&cap[1], true));
        } else if !line.is_empty() && !line.starts_with("OK") {
            hex_buf.push_str(line);
        }
    }
    if let Some((idx, stat, from, received)) = cur {
        list.push(SmsMessage {
            port: port.to_string(),
            index: idx,
            from,
            received,
            status: stat,
            text: decode_hex_or_raw(&hex_buf),
            part_indices: Vec::new(),
        });
    }
    list
}

pub fn parse_indices(resp: &str) -> Vec<i32> {
    let mut list = Vec::new();
    for raw in resp.replace("\r\n", "\n").lines() {
        let line = raw.trim();
        if let Some(cap) = CMGL_HEADER_RE.captures(line) {
            let head = cap[1].trim();
            let h = head.split(',').next().unwrap_or(head);
            if let Ok(idx) = h.trim().parse() {
                list.push(idx);
            }
        }
    }
    list
}

// ── Single AT+CMGR ──

/// Parse a text-mode `AT+CMGR=<idx>` response.
///
/// `idx` is the slot the read was issued against. A `+CMGR` header carries a
/// status but no index, so the caller is the only one that knows which SIM slot
/// the message occupies, and it has to be threaded in rather than defaulted:
/// slot 0 does not exist on a SIM, is therefore absent from every `AT+CMGL`
/// listing, and `confirm_delete` reads that absence as "already gone".
pub fn parse_cmgr(resp: &str, port: &str, idx: i32) -> Option<SmsMessage> {
    let mut found = false;
    let mut status = String::new();
    let mut from = String::new();
    let mut received = DateTime::UNIX_EPOCH;
    let mut hex_buf = String::new();

    for raw in resp.replace("\r\n", "\n").lines() {
        let line = raw.trim();
        if let Some(cap) = CMGR_RE.captures(line) {
            found = true;
            let body = cap[1].trim();
            let f = split_quoted(body);
            let mut date_str = f.last().map(|s| s.trim().to_string()).unwrap_or_default();
            if f.len() > 2 && !date_str.contains('/') {
                if let Some(prev) = f.get(f.len() - 2) {
                    date_str = format!("{},{}", prev.trim(), date_str);
                }
            }
            status = f.first().map(|s| s.trim().to_string()).unwrap_or_default();
            from = f
                .get(1)
                .map(|s| decode_hex_or_raw(s.trim()))
                .unwrap_or_default();
            received = parse_date(&date_str);
        } else if found && !line.is_empty() && !line.starts_with("OK") && !line.starts_with("+CMTI")
        {
            hex_buf.push_str(line);
        }
    }
    if !found {
        return None;
    }
    Some(SmsMessage {
        port: port.to_string(),
        index: idx,
        status,
        from,
        received,
        text: decode_hex_or_raw(&hex_buf),
        part_indices: Vec::new(),
    })
}

// ── PDU mode ──

pub fn parse_pdu_list(resp: &str, port: &str) -> Vec<DeliverInfo> {
    let mut list = Vec::new();
    let mut cur_index: i32 = 0;
    let mut cur_stat = String::new();

    for raw in resp.replace("\r\n", "\n").lines() {
        let line = raw.trim();
        if let Some(cap) = CMGL_HEADER_RE.captures(line) {
            let f: Vec<&str> = cap[1].split(',').collect();
            cur_index = f.first().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
            cur_stat = f.get(1).map(|s| s.trim().to_string()).unwrap_or_default();
        } else if cur_index > 0 && PDU_DATA_RE.is_match(line) && line.len() % 2 == 0 {
            if let Some(info) = decode_deliver(line, port) {
                list.push(DeliverInfo {
                    message: SmsMessage {
                        port: port.to_string(),
                        index: cur_index,
                        from: info.message.from,
                        received: info.message.received,
                        status: cur_stat.clone(),
                        text: info.message.text,
                        part_indices: Vec::new(),
                    },
                    concat: info.concat,
                });
            }
            cur_index = 0;
        }
    }
    list
}

/// Parse an `AT+CMGR` response while the modem is in PDU mode
/// (`AT+CMGF=0`). Response looks like:
/// `+CMGR: <stat>,...,<length>\n<pdu hex>\nOK`
///
/// `idx` is the slot the read was issued against — see `parse_cmgr` for why it
/// cannot be defaulted.
pub fn parse_pdu_cmgr(resp: &str, port: &str, idx: i32) -> Option<DeliverInfo> {
    let mut stat: Option<String> = None;
    for raw in resp.replace("\r\n", "\n").lines() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix("+CMGR:") {
            if stat.is_none() {
                let head = rest
                    .split(',')
                    .next()
                    .unwrap_or("")
                    .trim();
                let clean = head.trim_matches('"');
                if !clean.is_empty() {
                    stat = Some(clean.to_string());
                }
            }
        } else if stat.is_some() && PDU_DATA_RE.is_match(line) && line.len() % 2 == 0 {
            let info = decode_deliver(line, port)?;
            return Some(DeliverInfo {
                message: SmsMessage {
                    index: idx,
                    status: normalize_pdu_stat(stat.as_deref().unwrap_or("")),
                    ..info.message
                },
                concat: info.concat,
            });
        }
    }
    None
}

fn normalize_pdu_stat(raw: &str) -> String {
    match raw {
        "0" => "REC UNREAD".into(),
        "1" => "REC READ".into(),
        "2" => "STO UNSENT".into(),
        "3" => "STO SENT".into(),
        other => other.to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmsAlphabet {
    Gsm7,
    EightBit,
    Ucs2,
}

pub fn parse_dcs(dcs: u8) -> SmsAlphabet {
    let hi = (dcs >> 4) & 0x0F;
    match hi {
        // Group 00xx (0x00..0x3F): General Data Coding
        // Group 01xx (0x40..0x7F): Automatic Deletion Group
        0x0..=0x7 => match (dcs >> 2) & 0x03 {
            0x00 => SmsAlphabet::Gsm7,
            0x01 => SmsAlphabet::EightBit,
            0x02 => SmsAlphabet::Ucs2,
            _ => SmsAlphabet::Gsm7,
        },
        // Group 10xx (0x80..0xBF): Operator-specific / Reserved
        0x8..=0xB => {
            if (dcs & 0x0C) == 0x08 {
                SmsAlphabet::Ucs2
            } else if (dcs & 0x0C) == 0x04 {
                SmsAlphabet::EightBit
            } else {
                SmsAlphabet::Gsm7
            }
        }
        // Group 1100 (0xC0..0xCF): Discard message (GSM 7-bit)
        // Group 1101 (0xD0..0xDF): Store message (GSM 7-bit)
        0xC | 0xD => SmsAlphabet::Gsm7,
        // Group 1110 (0xE0..0xEF): Store message (UCS-2) (common on Myanmar carrier gateways)
        0xE => SmsAlphabet::Ucs2,
        // Group 1111 (0xF0..0xFF): Data coding / message class
        0xF => {
            if (dcs & 0x04) != 0 {
                SmsAlphabet::EightBit
            } else {
                SmsAlphabet::Gsm7
            }
        }
        _ => SmsAlphabet::Gsm7,
    }
}

fn decode_deliver(pdu_hex: &str, port: &str) -> Option<DeliverInfo> {
    let bytes = hex::decode(pdu_hex).ok()?;
    let mut i = 0;
    let sca_len = *bytes.get(i)? as usize;
    i += 1 + sca_len;
    if i >= bytes.len() {
        return None;
    }
    let first = bytes[i];
    i += 1;
    let addr_len = *bytes.get(i)? as usize;
    i += 1;
    let toa = *bytes.get(i)?;
    i += 1;
    let addr_bytes = addr_len.div_ceil(2);
    if i + addr_len > bytes.len() {
        return None;
    }
    let from = if (toa & 0x70) == 0x50 {
        decode_gsm7(&bytes, i, (addr_len * 4) / 7, 0)
    } else {
        semi_octet_address(&bytes, i, addr_bytes, addr_len)
    };
    i += addr_bytes;
    if i + 2 > bytes.len() {
        return None;
    }
    let dcs = bytes[i + 1];
    i += 2;
    if i + 7 > bytes.len() {
        return None;
    }
    let ts = parse_scts(&bytes, i);
    i += 7;
    if i >= bytes.len() {
        return None;
    }
    let udl = *bytes.get(i)? as usize;
    i += 1;
    let has_udh = (first & 0x40) != 0;
    let mut concat: Option<ConcatInfo> = None;
    let alphabet = parse_dcs(dcs);
    let text = match alphabet {
        SmsAlphabet::Ucs2 => {
            let mut udhl = 0;
            if has_udh {
                // A header that does not fit rejects the PDU (see `split_udh`).
                let (c, h, next) = split_udh(&bytes, i)?;
                concat = c;
                udhl = h;
                i = next;
            }
            // UDL is declared by the sender and routinely overshoots what the
            // serial read actually delivered, so it is clamped to the bytes on
            // hand rather than trusted.
            let len = udl.saturating_sub(udhl).min(bytes.len().saturating_sub(i));
            utf16be_to_string(&bytes, i, len)
        }
        SmsAlphabet::EightBit => {
            let mut udhl = 0;
            if has_udh {
                let (c, h, next) = split_udh(&bytes, i)?;
                concat = c;
                udhl = h;
                i = next;
            }
            let len = udl.saturating_sub(udhl).min(bytes.len().saturating_sub(i));
            let slice = bytes.get(i..i + len).unwrap_or(&[]);
            if slice.len() >= 2 && slice.len() % 2 == 0 && (slice[0] == 0x10 || slice[0] == 0x00) {
                utf16be_to_string(&bytes, i, len)
            } else if let Ok(s) = std::str::from_utf8(slice) {
                s.to_string()
            } else {
                String::from_utf8_lossy(slice).into_owned()
            }
        }
        SmsAlphabet::Gsm7 => {
            let mut septets = udl;
            let mut skip = 0;
            // GSM-7 septets are counted from the start of the UDH, not from the
            // first payload byte: the header occupies whole septets plus fill
            // bits. `skip` walks over both, so the bit cursor must start at the
            // UDHL byte even though `i` moves past the header for the UCS-2
            // recovery probe below.
            let ud_start = i;
            if has_udh {
                let (c, udhl, next) = split_udh(&bytes, i)?;
                concat = c;
                i = next;
                skip = ((udhl + 1) * 8).div_ceil(7);
                septets = udl.saturating_sub(skip);
            }
            let decoded = decode_gsm7(&bytes, ud_start, septets, skip);
            // Automatic recovery: If GSM-7 output contains corrupt symbols and raw payload is valid UTF-16BE
            let raw_len = bytes.len().saturating_sub(i);
            if raw_len >= 2 && decoded.chars().any(|c| "¿ΩÉÑ¡ΔΦΓΛΠΨΣΘΞÞßῪῤò".contains(c)) {
                let ucs2_candidate = utf16be_to_string(&bytes, i, raw_len);
                let myanmar_or_ascii = ucs2_candidate
                    .chars()
                    .filter(|c| {
                        ('\u{1000}'..='\u{109F}').contains(c)
                            || c.is_ascii_alphanumeric()
                            || c.is_ascii_whitespace()
                    })
                    .count();
                if myanmar_or_ascii > 0 && myanmar_or_ascii >= ucs2_candidate.chars().count() / 2 {
                    ucs2_candidate
                } else {
                    decoded
                }
            } else {
                decoded
            }
        }
    };
    Some(DeliverInfo {
        message: SmsMessage {
            port: port.to_string(),
            // A raw PDU carries no SIM slot: it is `+CMGL`/`+CMGR` framing that
            // names the slot, so both callers overwrite this. Never read it.
            index: 0,
            from,
            received: ts,
            status: String::new(),
            text,
            part_indices: Vec::new(),
        },
        concat,
    })
}

fn semi_octet_address(bytes: &[u8], offset: usize, bytes_len: usize, digits: usize) -> String {
    let mut sb = String::new();
    for i in 0..bytes_len.min(digits) {
        if offset + i >= bytes.len() {
            break;
        }
        let b = bytes[offset + i];
        sb.push(char::from_digit((b & 0x0F) as u32, 10).unwrap_or('?'));
        if sb.len() < digits {
            let hi = (b >> 4) & 0x0F;
            sb.push(if hi < 10 {
                char::from_digit(hi as u32, 10).unwrap()
            } else {
                '?'
            });
        }
    }
    sb
}

fn parse_scts(bytes: &[u8], offset: usize) -> DateTime<Utc> {
    let bcd = |b: u8| ((b & 0x0F) as u32) * 10 + ((b >> 4) as u32);
    let mut v = [0u32; 6];
    for i in 0..6 {
        if offset + i >= bytes.len() {
            break;
        }
        v[i] = bcd(bytes[offset + i]);
    }
    let tz_raw = if offset + 6 < bytes.len() {
        bytes[offset + 6]
    } else {
        0
    };
    let negative = tz_raw & 0x08 != 0;
    let quarters = ((tz_raw & 0x07) as i64) * 10 + ((tz_raw >> 4) & 0x0F) as i64;
    let minutes = quarters * 15;
    NaiveDate::from_ymd_opt(2000 + v[0] as i32, v[1], v[2])
        .and_then(|d| d.and_hms_opt(v[3], v[4], v[5]))
        .map(|dt| {
            let utc = Utc.from_utc_datetime(&dt);
            if negative {
                utc + Duration::minutes(minutes)
            } else {
                utc - Duration::minutes(minutes)
            }
        })
        .unwrap_or(DateTime::UNIX_EPOCH)
}

fn utf16be_to_string(bytes: &[u8], offset: usize, byte_len: usize) -> String {
    let mut s = String::new();
    let mut i = 0;
    while i + 1 < byte_len && offset + i + 1 < bytes.len() {
        let code = ((bytes[offset + i] as u32) << 8) | (bytes[offset + i + 1] as u32);
        if let Some(c) = char::from_u32(code) {
            s.push(c);
        }
        i += 2;
    }
    s
}

fn decode_gsm7(data: &[u8], start: usize, count: usize, skip_septets: usize) -> String {
    let mut sb = String::new();
    let mut bit = skip_septets * 7;
    let mut esc = false;
    for _ in 0..count {
        let byte_idx = start + (bit >> 3);
        if byte_idx >= data.len() {
            break;
        }
        let shift = bit & 7;
        let mut v = (data[byte_idx] >> shift) as u32;
        if shift > 0 && byte_idx + 1 < data.len() {
            v |= (data[byte_idx + 1] as u32) << (8 - shift);
        }
        let c = (v & 0x7F) as usize;
        bit += 7;
        if esc {
            let e = GSM7_EXT[c];
            sb.push_str(if e.is_empty() { GSM7_BASIC[c] } else { e });
            esc = false;
        } else if c == 0x1B {
            esc = true;
        } else {
            sb.push_str(GSM7_BASIC.get(c).copied().unwrap_or(""));
        }
    }
    sb
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    // ── PDU test helpers ──

    const GSM7_ALPHABET: &str = "@£$¥èéùìòÇ\nØø\rÅæÆÉΔΦΓΛΩΠΨΣΘΞÞßÉ  !\"#¤%&'()*+,-./0123456789:;<=>?¡ABCDEFGHIJKLMNOPQRSTUVWXYZÄÖÑÜ§¿abcdefghijklmnopqrstuvwxyzäöñüà";

    fn gsm_index(ch: char) -> u8 {
        GSM7_ALPHABET
            .chars()
            .position(|c| c == ch)
            .map(|i| i as u8)
            .unwrap_or(0)
    }

    fn utf16be_bytes(s: &str) -> Vec<u8> {
        let mut out = Vec::new();
        for c in s.encode_utf16() {
            out.push((c >> 8) as u8);
            out.push((c & 0xFF) as u8);
        }
        out
    }

    fn pack_gsm7(text: &str) -> Vec<u8> {
        let septets: Vec<u8> = text.chars().map(gsm_index).collect();
        let mut out = Vec::new();
        let mut carry: u32 = 0;
        let mut bits: u32 = 0;
        for s in septets {
            carry |= (s as u32) << bits;
            bits += 7;
            while bits >= 8 {
                out.push((carry & 0xFF) as u8);
                carry >>= 8;
                bits -= 8;
            }
        }
        if bits > 0 {
            out.push((carry & 0xFF) as u8);
        }
        out
    }

    fn build_deliver_pdu(
        sender: &str,
        dcs: u8,
        payload: &[u8],
        udl_override: Option<u8>,
    ) -> String {
        let mut b = vec![0x00u8, 0x04]; // SCA length 0; MTI=deliver, no UDHI
        b.push(sender.len() as u8); // address length (digits)
        b.push(0x91); // TOA: international
        for i in (0..sender.len()).step_by(2) {
            let lo = sender.as_bytes()[i] - b'0';
            let hi = if i + 1 < sender.len() {
                sender.as_bytes()[i + 1] - b'0'
            } else {
                0x0F
            };
            b.push((hi << 4) | lo);
        }
        b.push(0x00); // PID
        b.push(dcs); // DCS
        b.extend_from_slice(&[0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x00]); // SCTS
        b.push(udl_override.unwrap_or(payload.len() as u8));
        b.extend_from_slice(payload);
        b.iter().map(|x| format!("{:02x}", x)).collect()
    }

    // ── OTP extraction ──

    #[test]
    fn otp_english_keywords() {
        assert_eq!(
            extract_otp("Your KBZPay OTP code is 483920. Do not share."),
            Some("483920".into())
        );
        assert_eq!(
            extract_otp("WavePay verification code: 774213"),
            Some("774213".into())
        );
    }

    #[test]
    fn otp_myanmar_keywords() {
        assert_eq!(extract_otp("ထောက်ပံ့ငွေ ကုဒ် ၅၅၂၂၁၁"), Some("552211".into()));
        let msg = "\u{101E}\u{102D}\u{1015}\u{103A}\u{1001}\u{102F}\u{1016}\u{103A}\u{1010}\u{1032}\u{1037} \u{1000}\u{102F}\u{1012}\u{103A} \u{1045}\u{1045}\u{1042}\u{1042}\u{1041}\u{1041}";
        assert_eq!(extract_otp(msg), Some("552211".into()));
    }

    /// အတည်ပြု ("confirm") used to be misspelled အတန်ပြု in the keyword gate, so
    /// a Myanmar OTP whose only trigger word is အတည်ပြု never reached the
    /// patterns and silently came back as None. The body below carries no other
    /// keyword — no ကုဒ်, no လုံခြုံ, no ဖြစ်, no English trigger — so it only
    /// passes once the constant is spelled correctly.
    #[test]
    fn otp_myanmar_confirm_keyword() {
        let msg = "\u{1021}\u{1010}\u{100A}\u{103A}\u{1015}\u{103C}\u{102F}\u{1014}\u{1036}\u{1015}\u{102B}\u{1010}\u{103A} \u{1044}\u{1048}\u{1042}\u{1049}\u{1043}\u{1041} \u{1000}\u{102D}\u{102F} \u{1019}\u{100A}\u{103A}\u{101E}\u{1030}\u{1037}\u{1000}\u{102D}\u{102F}\u{1019}\u{103E} \u{1019}\u{1015}\u{103C}\u{1031}\u{102C}\u{1015}\u{102B}\u{1014}\u{103E}\u{1004}\u{1037}\u{103A}\u{104B}";
        assert_eq!(extract_otp(msg), Some("482931".into()));
        // Same body minus the keyword: proves အတည်ပြု is what opened the gate.
        let no_keyword = "\u{1014}\u{1036}\u{1015}\u{102B}\u{1010}\u{103A} \u{1044}\u{1048}\u{1042}\u{1049}\u{1043}\u{1041} \u{1000}\u{102D}\u{102F} \u{1019}\u{100A}\u{103A}\u{101E}\u{1030}\u{1037}\u{1000}\u{102D}\u{102F}\u{1019}\u{103E} \u{1019}\u{1015}\u{103C}\u{1031}\u{102C}\u{1015}\u{102B}\u{1014}\u{103E}\u{1004}\u{1037}\u{103A}\u{104B}";
        assert_eq!(extract_otp(no_keyword), None);
    }

    #[test]
    fn otp_trailing_pattern() {
        assert_eq!(
            extract_otp("482931 is your MyID verification code"),
            Some("482931".into())
        );
    }

    #[test]
    fn otp_keyword_with_code() {
        assert_eq!(extract_otp("Your code 123456"), Some("123456".into()));
    }

    #[test]
    fn otp_four_digit() {
        assert_eq!(extract_otp("Your code is 1234"), Some("1234".into()));
    }

    #[test]
    fn otp_no_match() {
        assert_eq!(extract_otp("Hello world"), None);
        assert_eq!(extract_otp(""), None);
    }

    #[test]
    fn otp_requires_keyword() {
        assert_eq!(extract_otp("meeting at 123456 street"), None);
    }

    // ── normalizeNumber / USSD ──

    #[test]
    fn normalize_959() {
        assert_eq!(normalize_number("959780001122"), "09780001122");
    }

    #[test]
    fn normalize_with_dashes() {
        assert_eq!(normalize_number("09-780-001-122"), "09780001122");
    }

    #[test]
    fn normalize_plain() {
        assert_eq!(normalize_number("12345"), "12345");
    }

    #[test]
    fn extract_number_plain_ussd() {
        let r = "+CUSD: 2,\"Your number is 09780001122\",15";
        assert_eq!(extract_number_from_ussd(r), Some("09780001122".into()));
    }

    #[test]
    fn extract_number_hex_ussd() {
        let r = "+CUSD: 2,\"09780001122\",15";
        assert_eq!(extract_number_from_ussd(r), Some("09780001122".into()));
    }

    // ── ICCID ──

    #[test]
    fn iccid_from_ccid_reply() {
        let r = "+CCID: 8995010912345678901\r\n\r\nOK";
        assert_eq!(
            extract_iccid(r),
            Some("8995010912345678901".into()),
            "19-digit ICCID behind the +CCID prefix"
        );
    }

    #[test]
    fn iccid_strips_filler_nibble() {
        let r = "+ICCID: 8995010912345678901F\r\nOK";
        assert_eq!(extract_iccid(r), Some("8995010912345678901".into()));
    }

    #[test]
    fn iccid_from_bare_digits() {
        let r = "AT+CCID\r\n89950109123456789012\r\nOK";
        assert_eq!(extract_iccid(r), Some("89950109123456789012".into()));
    }

    #[test]
    fn iccid_absent_on_error() {
        assert_eq!(extract_iccid("+CME ERROR: 10\r\n"), None);
        assert_eq!(extract_iccid("OK"), None);
        // A phone number is far too short to be mistaken for a card serial.
        assert_eq!(extract_iccid("+CNUM: \"\",\"09780001122\",129"), None);
    }

    // ── AT+CNUM ──

    #[test]
    fn cnum_reads_local_number() {
        let r = "+CNUM: \"MSISDN\",\"09780001122\",129\r\nOK";
        assert_eq!(extract_number_from_cnum(r), Some("09780001122".into()));
    }

    #[test]
    fn cnum_reads_international_number() {
        let r = "+CNUM: \"\",\"+959780001122\",145\r\nOK";
        assert_eq!(
            normalize_number(&extract_number_from_cnum(r).unwrap()),
            "09780001122"
        );
    }

    #[test]
    fn cnum_skips_record_without_myanmar_number() {
        let r = "+CNUM: \"Voice Mail\",\"1234\",129\r\n+CNUM: \"Own\",\"09780001122\",129\r\nOK";
        assert_eq!(extract_number_from_cnum(r), Some("09780001122".into()));
    }

    #[test]
    fn cnum_blank_msisdn_is_not_a_number() {
        assert_eq!(extract_number_from_cnum("OK"), None);
        assert_eq!(extract_number_from_cnum("+CNUM: \"\",\"\",129\r\nOK"), None);
    }

    // ── decodeHexOrRaw ──

    #[test]
    fn hex_decode() {
        assert_eq!(decode_hex_or_raw("00480069"), "Hi");
    }

    #[test]
    fn hex_raw_passthrough() {
        assert_eq!(decode_hex_or_raw("+CMGL header"), "+CMGL header");
    }

    #[test]
    fn hex_decode_quoted() {
        assert_eq!(decode_hex_or_raw("\"0048\""), "H");
    }

    // ── CMTI ──

    #[test]
    fn cmti_parsing() {
        let results = find_cmti("+CMTI: \"SM\",3\n+CMTI: \"SM\",5");
        assert_eq!(results.len(), 2);
        assert_eq!(parse_cmti_index(&results[0]), Some(3));
        assert_eq!(parse_cmti_index(&results[1]), Some(5));
    }

    // ── Text mode parsing ──

    #[test]
    fn text_mode_parse_ucs2() {
        let resp =
            "+CMGL: 3,\"REC UNREAD\",\"0039003500390035003100320033003400350036003700380039\",\
            \"26/08/24,12:34:56+32\"\n\
            00480069\n\
            OK\n";
        let list = parse_text_mode_list(resp, "COM3");
        assert_eq!(list.len(), 1);
        let m = &list[0];
        assert_eq!(m.index, 3);
        assert_eq!(m.status, "REC UNREAD");
        assert_eq!(m.text, "Hi");
        assert_eq!(m.port, "COM3");
        assert_eq!(m.received.year(), 2026);
        assert_eq!(m.received.minute(), 34);
    }

    // ── Timezone handling ──

    #[test]
    fn text_mode_date_applies_modem_timezone() {
        let dt = parse_date("26/08/26,14:41:45+26");
        assert_eq!(dt.to_rfc3339(), "2026-08-26T08:11:45+00:00");
    }

    #[test]
    fn text_mode_date_negative_offset() {
        let dt = parse_date("26/08/26,14:41:45-08");
        assert_eq!(dt.to_rfc3339(), "2026-08-26T16:41:45+00:00");
    }

    #[test]
    fn text_mode_date_without_offset_treated_as_utc() {
        let dt = parse_date("2026/08/26 14:41:45");
        assert_eq!(dt.to_rfc3339(), "2026-08-26T14:41:45+00:00");
    }

    #[test]
    fn scts_applies_pdu_timezone() {
        let bytes = [0x62, 0x80, 0x62, 0x41, 0x14, 0x54, 0x62];
        let dt = parse_scts(&bytes, 0);
        assert_eq!(dt.to_rfc3339(), "2026-08-26T08:11:45+00:00");
    }

    #[test]
    fn scts_negative_timezone() {
        let bytes = [0x62, 0x80, 0x62, 0x41, 0x14, 0x54, 0x6A];
        let dt = parse_scts(&bytes, 0);
        assert_eq!(dt.to_rfc3339(), "2026-08-26T21:11:45+00:00");
    }

    // ── PDU decoding ──

    #[test]
    fn pdu_ucs2_deliver() {
        let payload = utf16be_bytes("မင်္ဂလာပါ Hi");
        let pdu = build_deliver_pdu("959123456789", 0x08, &payload, None);
        let m = decode_deliver(&pdu, "/dev/ttyUSB5");
        assert!(m.is_some());
        let m = m.unwrap().message;
        assert_eq!(m.from, "959123456789");
        assert_eq!(m.text, "မင်္ဂလာပါ Hi");
        assert_eq!(m.received.year(), 2001);
    }

    #[test]
    fn pdu_gsm7_deliver() {
        let payload = pack_gsm7("Hello world!");
        let pdu = build_deliver_pdu("09680001122", 0x00, &payload, Some(12));
        let m = decode_deliver(&pdu, "COM7");
        assert!(m.is_some());
        let m = m.unwrap().message;
        assert_eq!(m.text, "Hello world!");
        assert_eq!(m.from, "09680001122");
    }

    #[test]
    fn concat_iei_16bit() {
        let info = extract_concat_from_udh(&[0x08, 4, 0x01, 0x02, 3, 1]);
        assert_eq!(
            info,
            Some(ConcatInfo {
                ref_num: 0x0102,
                total: 3,
                seq: 1
            })
        );
    }

    #[test]
    fn concat_iei_8bit() {
        let info = extract_concat_from_udh(&[0x00, 3, 42, 3, 2]);
        assert_eq!(
            info,
            Some(ConcatInfo {
                ref_num: 42,
                total: 3,
                seq: 2
            })
        );
    }

    /// Build a deliver PDU carrying a User Data Header (for concatenated SMS).
    fn build_deliver_pdu_udh(sender: &str, dcs: u8, udh: &[u8], payload: &[u8]) -> String {
        let mut b = vec![0x00u8, 0x44]; // SCA length, MTI=deliver + UDHI
        b.push(sender.len() as u8);
        b.push(0x91);
        for i in (0..sender.len()).step_by(2) {
            let lo = sender.as_bytes()[i] - b'0';
            let hi = if i + 1 < sender.len() {
                sender.as_bytes()[i + 1] - b'0'
            } else {
                0x0F
            };
            b.push((hi << 4) | lo);
        }
        b.push(0x00); // PID
        b.push(dcs); // DCS
        b.extend_from_slice(&[0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x00]); // SCTS
        b.push((udh.len() + payload.len()) as u8); // UDL includes UDH
        b.extend_from_slice(&[udh.len() as u8]); // UDHL
        b.extend_from_slice(udh);
        b.extend_from_slice(payload);
        b.iter().map(|x| format!("{:02x}", x)).collect()
    }

    #[test]
    fn pdu_concat_ucs2_two_parts() {
        let udh16 = |ref_hi: u8, ref_lo: u8, total: u8, seq: u8| -> Vec<u8> {
            vec![0x08, 4, ref_hi, ref_lo, total, seq]
        };
        let pdu1 = build_deliver_pdu_udh(
            "959123456789",
            0x08,
            &udh16(0x12, 0x34, 2, 1),
            &utf16be_bytes("ဝိုး! ဒေတာ 1.5GB ကို ၉၉၉ ကျပ်နဲ့ "),
        );
        let pdu2 = build_deliver_pdu_udh(
            "959123456789",
            0x08,
            &udh16(0x12, 0x34, 2, 2),
            &utf16be_bytes("15 ရက်စာ အသုံးပြုနိုင်ပါသည်"),
        );
        let resp = format!(
            "+CMGL: 11,\"REC UNREAD\",\"\",26/08/26,12:00:00+26\n{}\n+CMGL: 12,\"REC UNREAD\",\"\",26/08/26,12:00:00+26\n{}\nOK\n",
            pdu1, pdu2
        );
        let infos = parse_pdu_list(&resp, "COM3");
        assert_eq!(infos.len(), 2);
        assert_eq!(infos[0].concat.map(|c| (c.ref_num, c.total, c.seq)), Some((0x1234, 2, 1)));
        assert_eq!(infos[0].message.index, 11);

        use crate::core::reassemble::Reassembler;
        let mut asm = Reassembler::new();
        let mut out: Vec<SmsMessage> = Vec::new();
        for d in infos {
            match d.concat {
                Some(c) => {
                    if let Some(done) = asm.push(&d.message, c) {
                        out.push(done);
                    }
                }
                None => out.push(d.message),
            }
        }
        assert_eq!(out.len(), 1);
        assert_eq!(
            out[0].text,
            "ဝိုး! ဒေတာ 1.5GB ကို ၉၉၉ ကျပ်နဲ့ 15 ရက်စာ အသုံးပြုနိုင်ပါသည်"
        );
        assert_eq!(out[0].part_indices, vec![11, 12]);
    }

    /// Build a deliver PDU carrying GSM-7 user data behind a UDH. GSM-7 septets
    /// must start on the septet boundary that follows the UDH, so the payload is
    /// packed with leading fill bits and UDL counts septets, not bytes.
    fn build_deliver_pdu_udh_gsm7(sender: &str, udh: &[u8], text: &str) -> String {
        let header_bits = (udh.len() + 1) * 8;
        let skip = header_bits.div_ceil(7);
        let fill = skip * 7 - header_bits;

        let mut payload = Vec::new();
        let mut carry: u32 = 0;
        let mut bits: u32 = fill as u32;
        for s in text.chars().map(gsm_index) {
            carry |= (s as u32) << bits;
            bits += 7;
            while bits >= 8 {
                payload.push((carry & 0xFF) as u8);
                carry >>= 8;
                bits -= 8;
            }
        }
        if bits > 0 {
            payload.push((carry & 0xFF) as u8);
        }

        let mut b = vec![0x00u8, 0x44]; // SCA length, MTI=deliver + UDHI
        b.push(sender.len() as u8);
        b.push(0x91);
        for i in (0..sender.len()).step_by(2) {
            let lo = sender.as_bytes()[i] - b'0';
            let hi = if i + 1 < sender.len() {
                sender.as_bytes()[i + 1] - b'0'
            } else {
                0x0F
            };
            b.push((hi << 4) | lo);
        }
        b.push(0x00); // PID
        b.push(0x00); // DCS: GSM-7
        b.extend_from_slice(&[0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x00]); // SCTS
        b.push((skip + text.chars().count()) as u8); // UDL in septets
        b.push(udh.len() as u8); // UDHL
        b.extend_from_slice(udh);
        b.extend_from_slice(&payload);
        b.iter().map(|x| format!("{:02x}", x)).collect()
    }

    #[test]
    fn pdu_concat_gsm7_two_parts() {
        // 8-bit reference concat header: UDHL=5, so the septet stream starts one
        // fill bit after the header — the case that garbles OTP texts when the
        // decoder skips the header twice.
        let udh8 = |r: u8, total: u8, seq: u8| -> Vec<u8> { vec![0x00, 3, r, total, seq] };
        let pdu1 = build_deliver_pdu_udh_gsm7(
            "959123456789",
            &udh8(0x42, 2, 1),
            "719815 is your OTP code to login MYID. ",
        );
        let pdu2 = build_deliver_pdu_udh_gsm7(
            "959123456789",
            &udh8(0x42, 2, 2),
            "If you didn't request it, please call 966.",
        );
        let resp = format!(
            "+CMGL: 3,\"REC UNREAD\",\"\",26/08/29,08:54:06+26\n{}\n+CMGL: 4,\"REC UNREAD\",\"\",26/08/29,08:54:08+26\n{}\nOK\n",
            pdu1, pdu2
        );
        let infos = parse_pdu_list(&resp, "/dev/ttyUSB20");
        assert_eq!(infos.len(), 2);
        assert_eq!(
            infos[0].concat.map(|c| (c.ref_num, c.total, c.seq)),
            Some((0x42, 2, 1))
        );

        use crate::core::reassemble::Reassembler;
        let mut asm = Reassembler::new();
        let mut out: Vec<SmsMessage> = Vec::new();
        for d in infos {
            match d.concat {
                Some(c) => {
                    if let Some(done) = asm.push(&d.message, c) {
                        out.push(done);
                    }
                }
                None => out.push(d.message),
            }
        }
        assert_eq!(out.len(), 1);
        assert_eq!(
            out[0].text,
            "719815 is your OTP code to login MYID. If you didn't request it, please call 966."
        );
        assert_eq!(extract_otp(&out[0].text).as_deref(), Some("719815"));
    }

    #[test]
    fn pdu_concat_gsm7_16bit_ref_no_fill_bits() {
        // UDHL=6 makes the header land exactly on a septet boundary (zero fill
        // bits) — the sibling of the UDHL=5 case, and the one that would silently
        // pass if the skip were applied to the wrong base offset by chance.
        let udh16 = vec![0x08u8, 4, 0x12, 0x34, 2, 1];
        let pdu = build_deliver_pdu_udh_gsm7("966", &udh16, "Balance low. Top up now.");
        let d = decode_deliver(&pdu, "COM5").unwrap();
        assert_eq!(d.message.text, "Balance low. Top up now.");
        assert_eq!(
            d.concat.map(|c| (c.ref_num, c.total, c.seq)),
            Some((0x1234, 2, 1))
        );
    }

    #[test]
    fn pdu_single_gsm7_no_udh_unaffected() {
        let pdu = build_deliver_pdu("966", 0x00, &pack_gsm7("Your OTP is 483920"), Some(18));
        let d = decode_deliver(&pdu, "COM1").unwrap();
        assert_eq!(d.message.text, "Your OTP is 483920");
    }

    /// Build a UDHI deliver PDU with a hand-written user-data field, so the
    /// UDHL byte can lie about how much header follows. `udl` is written to the
    /// PDU verbatim — malformed PDUs disagree with the bytes they carry, which is
    /// the whole point of these cases.
    fn build_deliver_pdu_raw_ud(dcs: u8, udl: u8, ud: &[u8]) -> String {
        let mut b = vec![0x00u8, 0x44]; // SCA length 0; MTI=deliver + UDHI
        b.push(3); // address length (digits)
        b.push(0x91); // TOA: international
        b.push(0x69); // "96"
        b.push(0xF6); // "6" + filler
        b.push(0x00); // PID
        b.push(dcs);
        b.extend_from_slice(&[0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x00]); // SCTS
        b.push(udl);
        b.extend_from_slice(ud);
        b.iter().map(|x| format!("{:02x}", x)).collect()
    }

    // ── Malformed UDH: must never panic ──
    //
    // A panic here is not a lost message: `commands::start_scan` catches it per
    // worker (so one port's whole read is dropped) and `live::run_live` catches
    // it per worker, which retires that port from live monitoring for the rest of
    // the session. Rejecting the PDU costs one SMS instead.

    /// UDHL says five header bytes follow, the PDU carries two. This is the input
    /// that used to reach `len = bytes.len() - i` with `i` past the end and panic
    /// with "attempt to subtract with overflow" in a debug build.
    #[test]
    fn pdu_truncated_mid_udh_is_rejected() {
        for dcs in [0x08u8, 0x04, 0x00] {
            let pdu = build_deliver_pdu_raw_ud(dcs, 7, &[0x05, 0x00, 0x03]);
            assert_eq!(
                decode_deliver(&pdu, "COM1").map(|d| d.message.text),
                None,
                "dcs {:#04x}: a header that does not fit is rejected, not decoded",
                dcs
            );
        }
    }

    /// UDHL far past the end of the buffer, in all three alphabets — the UCS-2
    /// and 8-bit branches panicked, the GSM-7 branch silently produced garbage.
    #[test]
    fn pdu_udh_length_beyond_the_buffer_is_rejected() {
        for dcs in [0x08u8, 0x04, 0x00] {
            let pdu = build_deliver_pdu_raw_ud(dcs, 0x21, &[0x20, 0x00, 0x03, 0x42]);
            assert!(
                decode_deliver(&pdu, "COM1").is_none(),
                "dcs {:#04x} should reject a UDHL of 32 with 3 bytes present",
                dcs
            );
        }
        // Nothing at all after the UDL byte, UDHI still set.
        assert!(decode_deliver(&build_deliver_pdu_raw_ud(0x08, 6, &[]), "COM1").is_none());
    }

    /// The header itself fits, but an information element inside it claims more
    /// bytes than the header holds. The payload start *is* known here, so the
    /// message is kept — just without concat info, i.e. as a standalone SMS.
    #[test]
    fn pdu_udh_element_running_past_the_header_keeps_the_text() {
        let mut ud = vec![0x05u8, 0x00, 0x0A, 0x42, 0x02, 0x01];
        ud.extend_from_slice(&utf16be_bytes("Hi"));
        let pdu = build_deliver_pdu_raw_ud(0x08, 9, &ud);
        let d = decode_deliver(&pdu, "COM1").expect("payload is intact, so the SMS survives");
        assert_eq!(d.message.text, "Hi");
        assert_eq!(
            d.concat, None,
            "a bogus element must not fabricate a fragment"
        );
    }

    /// UDHI set with an empty header (UDHL = 0). Legal, if pointless: no concat
    /// info, and the payload starts right after the UDHL byte.
    #[test]
    fn pdu_zero_length_udh_decodes_the_payload() {
        let mut ud = vec![0x00u8];
        ud.extend_from_slice(&utf16be_bytes("Hi"));
        let pdu = build_deliver_pdu_raw_ud(0x08, 4, &ud);
        let d = decode_deliver(&pdu, "COM1").expect("an empty UDH is not a malformed PDU");
        assert_eq!(d.message.text, "Hi");
        assert_eq!(d.concat, None);
    }

    /// A zero-length information element is padding, not a stop sign: the concat
    /// header behind it still has to be found, or every part of a long SMS from
    /// such a gateway is filed as a separate message.
    #[test]
    fn zero_length_udh_element_does_not_hide_the_concat_header() {
        let info = extract_concat_from_udh(&[0x1F, 0x00, 0x08, 4, 0x12, 0x34, 2, 1]);
        assert_eq!(
            info,
            Some(ConcatInfo {
                ref_num: 0x1234,
                total: 2,
                seq: 1
            })
        );
        // A truncated trailing element is where the scan stops.
        assert_eq!(extract_concat_from_udh(&[0x1F, 0x00, 0x08, 4, 0x12]), None);
        assert_eq!(extract_concat_from_udh(&[]), None);
        assert_eq!(extract_concat_from_udh(&[0x08]), None);
    }

    /// UDL declares more payload than arrived (a truncated serial read). The
    /// header is complete, so the surviving prefix — often the whole OTP — is
    /// returned rather than thrown away, with the fragment number intact.
    #[test]
    fn pdu_payload_truncated_after_a_valid_udh_keeps_the_prefix() {
        let mut ud = vec![0x06u8, 0x08, 0x04, 0x12, 0x34, 0x02, 0x01];
        ud.extend_from_slice(&utf16be_bytes("Hi"));
        let pdu = build_deliver_pdu_raw_ud(0x08, 46, &ud); // claims 40 payload bytes, carries 4
        let d = decode_deliver(&pdu, "COM1").expect("a short tail is recoverable");
        assert_eq!(d.message.text, "Hi");
        assert_eq!(
            d.concat.map(|c| (c.ref_num, c.total, c.seq)),
            Some((0x1234, 2, 1))
        );
    }

    /// Every truncation of a well-formed concatenated PDU, in all three
    /// alphabets, through both entry points the port workers use. The assertion
    /// is that this test finishes: any panic fails it.
    #[test]
    fn every_truncation_of_a_concat_pdu_decodes_without_panicking() {
        let ucs2 = build_deliver_pdu_udh(
            "959123456789",
            0x08,
            &[0x08, 4, 0x12, 0x34, 2, 1],
            &utf16be_bytes("ကုဒ် 719815"),
        );
        let eight_bit = build_deliver_pdu_udh(
            "959123456789",
            0x04,
            &[0x00, 3, 0x42, 2, 1],
            &utf16be_bytes("code 719815"),
        );
        let gsm7 = build_deliver_pdu_udh_gsm7(
            "959123456789",
            &[0x00, 3, 0x42, 2, 1],
            "719815 is your OTP code",
        );
        for pdu in [ucs2, eight_bit, gsm7] {
            for cut in (0..pdu.len()).step_by(2) {
                let head = &pdu[..cut];
                let _ = decode_deliver(head, "COM1");
                let resp =
                    format!("+CMGL: 3,\"REC UNREAD\",\"\",26/08/29,08:54:06+26\n{head}\nOK\n");
                let _ = parse_pdu_list(&resp, "COM1");
                let _ = parse_pdu_cmgr(&format!("+CMGR: 0,,{}\n{head}\nOK\n", cut / 2), "COM1", 3);
            }
        }
    }

    #[test]
    fn pdumgr_single_read() {
        let payload = utf16be_bytes("Hi there");
        let pdu = build_deliver_pdu("959123456789", 0x08, &payload, None);
        let resp = format!("+CMGR: 0,,{0}\n{1}\nOK\n", pdu.len() / 2, pdu);
        let d = parse_pdu_cmgr(&resp, "COM7", 4);
        assert!(d.is_some());
        let d = d.unwrap();
        assert_eq!(d.message.text, "Hi there");
        assert_eq!(d.message.status, "REC UNREAD");
        assert_eq!(d.message.port, "COM7");
        assert!(d.concat.is_none());
    }

    /// A live-mode read is the only path that learns a message's SIM slot from
    /// the command it issued rather than from a list header. Losing it there
    /// meant `delete_selected` sent `AT+CMGD=0`, the modem refused, slot 0 was
    /// missing from the confirming `AT+CMGL` (it does not exist), and the row
    /// was dropped from the inbox while the SMS stayed on the card.
    #[test]
    fn pdu_cmgr_carries_the_slot_it_was_read_from() {
        let payload = utf16be_bytes("code 471928");
        let pdu = build_deliver_pdu("959123456789", 0x08, &payload, None);
        let resp = format!("+CMGR: 0,,{0}\n{1}\nOK\n", pdu.len() / 2, pdu);
        let d = parse_pdu_cmgr(&resp, "COM7", 17).expect("decodes");
        assert_eq!(d.message.index, 17);
    }

    #[test]
    fn text_mode_cmgr_carries_the_slot_it_was_read_from() {
        let resp = "+CMGR: \"REC UNREAD\",\"959123456789\",,\"26/08/29,08:54:06+26\"\n\
                    code 471928\nOK\n";
        let m = parse_cmgr(resp, "COM7", 9).expect("decodes");
        assert_eq!(m.index, 9);
        assert_eq!(m.text, "code 471928");
    }

    /// The reassembler takes each fragment's slot from `SmsMessage::index`, so a
    /// long SMS read fragment-by-fragment over `AT+CMGR` only lands every real
    /// slot in `part_indices` if the CMGR parser recorded them.
    #[test]
    fn concat_from_cmgr_reads_keeps_every_real_fragment_slot() {
        let mut asm = crate::core::reassemble::Reassembler::default();
        let mut assembled = None;
        for (seq, slot) in [(1u8, 5i32), (2, 6)] {
            let pdu = build_deliver_pdu_udh(
                "959123456789",
                0x08,
                &[0x00, 3, 0x42, 2, seq],
                &utf16be_bytes("part"),
            );
            let resp = format!("+CMGR: 0,,{0}\n{1}\nOK\n", pdu.len() / 2, pdu);
            let info = parse_pdu_cmgr(&resp, "COM7", slot).expect("decodes");
            let c = info.concat.expect("has a UDH");
            assembled = asm.push(&info.message, c);
        }
        let done = assembled.expect("both fragments arrived");
        assert_eq!(done.part_indices, vec![5, 6]);
        assert_eq!(done.index, 5);
    }

    #[test]
    fn pdu_dcs_store_ucs2_0xe0() {
        let payload = utf16be_bytes("၂၀၀ ကျပ်တန် အထူးအစီအစဉ်များ!");
        let pdu = build_deliver_pdu("966", 0xE0, &payload, None);
        let d = decode_deliver(&pdu, "COM17");
        assert!(d.is_some());
        let d = d.unwrap();
        assert_eq!(d.message.text, "၂၀၀ ကျပ်တန် အထူးအစီအစဉ်များ!");
        assert_eq!(d.message.from, "966");
    }

    #[test]
    fn pdu_dcs_8bit_ucs2_payload() {
        let payload = utf16be_bytes("၂၀၀ ကျပ်တန်");
        let pdu = build_deliver_pdu("966", 0x04, &payload, None);
        let d = decode_deliver(&pdu, "COM17");
        assert!(d.is_some());
        let d = d.unwrap();
        assert_eq!(d.message.text, "၂၀၀ ကျပ်တန်");
    }

    #[test]
    fn test_parse_dcs_mappings() {
        assert_eq!(parse_dcs(0x00), SmsAlphabet::Gsm7);
        assert_eq!(parse_dcs(0x04), SmsAlphabet::EightBit);
        assert_eq!(parse_dcs(0x08), SmsAlphabet::Ucs2);
        assert_eq!(parse_dcs(0x18), SmsAlphabet::Ucs2);
        assert_eq!(parse_dcs(0x48), SmsAlphabet::Ucs2);
        assert_eq!(parse_dcs(0xE0), SmsAlphabet::Ucs2);
        assert_eq!(parse_dcs(0xE4), SmsAlphabet::Ucs2);
        assert_eq!(parse_dcs(0xF4), SmsAlphabet::EightBit);
    }
}
