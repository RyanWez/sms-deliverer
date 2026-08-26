use crate::core::models::SmsMessage;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use regex::Regex;
use std::sync::LazyLock;

const KW_KODE: &str = "\u{1000}\u{102F}\u{1012}\u{103A}";
const KW_CONFIRM: &str = "\u{1021}\u{1010}\u{1014}\u{103A}\u{1015}\u{103C}\u{102F}";
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
static DATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\d{2,4})/(\d{1,2})/(\d{1,2})[ ,](\d{1,2}):(\d{1,2}):(\d{1,2})").unwrap()
});
static PDU_DATA_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[0-9A-Fa-f]{8,}$").unwrap());

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
    if t.len() >= 4 && t.len() % 4 == 0 && is_hex(t) {
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
    if digits.starts_with("95") && digits.len() >= 11 {
        format!("0{}", &digits[2..])
    } else if digits.starts_with("09") && digits.len() >= 10 {
        digits
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
                return Utc.from_utc_datetime(&t);
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

pub fn parse_cmgr(resp: &str, port: &str) -> Option<SmsMessage> {
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
        index: 0,
        status,
        from,
        received,
        text: decode_hex_or_raw(&hex_buf),
    })
}

// ── PDU mode ──

pub fn parse_pdu_list(resp: &str, port: &str) -> Vec<SmsMessage> {
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
            if let Some(m) = decode_deliver(line, port) {
                list.push(SmsMessage {
                    port: port.to_string(),
                    index: cur_index,
                    from: m.from,
                    received: m.received,
                    status: cur_stat.clone(),
                    text: m.text,
                });
            }
            cur_index = 0;
        }
    }
    list
}

fn decode_deliver(pdu_hex: &str, port: &str) -> Option<SmsMessage> {
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
    let addr_bytes = (addr_len + 1) / 2;
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
    let udh = (first & 0x40) != 0;
    let is_ucs2 = ((dcs >> 2) & 0x03) == 2;
    let text = if is_ucs2 {
        let mut udhl = 0;
        if udh && i < bytes.len() {
            udhl = bytes[i] as usize;
            i += 1 + udhl;
        }
        let mut len = udl.saturating_sub(udhl);
        if i + len > bytes.len() {
            len = bytes.len() - i;
        }
        utf16be_to_string(&bytes, i, len)
    } else {
        let mut septets = udl;
        let mut skip = 0;
        if udh && i < bytes.len() {
            let udhl = bytes[i] as usize;
            i += 1 + udhl;
            skip = ((udhl + 1) * 8 + 6) / 7;
            septets = udl.saturating_sub(skip);
        }
        decode_gsm7(&bytes, i, septets, skip)
    };
    Some(SmsMessage {
        port: port.to_string(),
        index: 0,
        from,
        received: ts,
        status: String::new(),
        text,
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
    let mut v = [0u8; 7];
    for i in 0..7 {
        if offset + i >= bytes.len() {
            break;
        }
        v[i] = ((bytes[offset + i] & 0x0F) << 4) | ((bytes[offset + i] >> 4) & 0x0F);
    }
    NaiveDate::from_ymd_opt(2000 + v[0] as i32, v[1] as u32, v[2] as u32)
        .and_then(|d| d.and_hms_opt(v[3] as u32, v[4] as u32, v[5] as u32))
        .map(|dt| Utc.from_utc_datetime(&dt))
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
        let mut b: Vec<u8> = Vec::new();
        b.push(0x00); // SCA length 0
        b.push(0x04); // first octet: MTI=deliver, no UDHI
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

    // ── PDU decoding ──

    #[test]
    fn pdu_ucs2_deliver() {
        let payload = utf16be_bytes("မင်္ဂလာပါ Hi");
        let pdu = build_deliver_pdu("959123456789", 0x08, &payload, None);
        let m = decode_deliver(&pdu, "/dev/ttyUSB5");
        assert!(m.is_some());
        let m = m.unwrap();
        assert_eq!(m.from, "959123456789");
        assert_eq!(m.text, "မင်္ဂလာပါ Hi");
        assert_eq!(m.received.year(), 2001);
    }

    #[test]
    fn pdu_gsm7_deliver() {
        let payload = pack_gsm7("Hello world!");
        let pdu = build_deliver_pdu("09780001122", 0x00, &payload, Some(12));
        let m = decode_deliver(&pdu, "COM7");
        assert!(m.is_some());
        let m = m.unwrap();
        assert_eq!(m.text, "Hello world!");
        assert_eq!(m.from, "09780001122");
    }
}
