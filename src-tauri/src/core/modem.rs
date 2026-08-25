use crate::core::models::SmsMessage;
use crate::core::decoder;
use std::io::{Read, Write};
use std::time::{Duration, Instant};
use std::thread;

pub const BAUD: u32 = 115200;

pub struct ReadResult {
    pub ok: bool,
    pub messages: Vec<SmsMessage>,
    pub error: Option<String>,
}

pub struct OpResult {
    pub ok: bool,
    pub error: Option<String>,
    pub deleted: usize,
    pub indices: Vec<i32>,
}

pub fn get_port_names() -> Vec<String> {
    let mut list: Vec<String> = serialport::available_ports()
        .map(|ports| {
            ports.into_iter()
                .filter_map(|p| {
                    let name = p.port_name;
                    if name.starts_with("COM")
                        || name.contains("ttyUSB")
                        || name.contains("ttyACM")
                    {
                        Some(name)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    list.sort_by_key(|n| port_num(n));
    list.dedup();
    list
}

pub fn port_num(s: &str) -> u32 {
    s.chars().rev().take_while(|c| c.is_ascii_digit()).collect::<String>()
        .chars().rev().collect::<String>()
        .parse().unwrap_or(0)
}

pub fn open_port(name: &str) -> Result<Box<dyn serialport::SerialPort>, String> {
    let sp = serialport::new(name, BAUD)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .timeout(Duration::from_millis(100))
        .open()
        .map_err(|e| format!("Cannot open {}: {}", name, e))?;
    Ok(sp)
}

fn send(sp: &mut Box<dyn serialport::SerialPort>, cmd: &str, timeout_ms: u64) -> String {
    let _ = sp.write_all(format!("{}\r", cmd).as_bytes());
    let _ = sp.flush();
    let mut sb = String::new();
    let end = Instant::now() + Duration::from_millis(timeout_ms);
    let mut buf = [0u8; 1024];
    while Instant::now() < end {
        match sp.read(&mut buf) {
            Ok(n) if n > 0 => {
                sb.push_str(&String::from_utf8_lossy(&buf[..n]));
                if sb.contains("OK") || sb.contains("ERROR") {
                    return sb;
                }
            }
            _ => {}
        }
        thread::sleep(Duration::from_millis(15));
    }
    sb
}

fn send_ussd(sp: &mut Box<dyn serialport::SerialPort>, code: &str, timeout_ms: u64) -> String {
    let _ = sp.write_all(format!("AT+CUSD=1,\"{}\",15\r", code).as_bytes());
    let _ = sp.flush();
    let mut sb = String::new();
    let end = Instant::now() + Duration::from_millis(timeout_ms);
    let mut buf = [0u8; 1024];
    while Instant::now() < end {
        match sp.read(&mut buf) {
            Ok(n) if n > 0 => {
                sb.push_str(&String::from_utf8_lossy(&buf[..n]));
                if sb.contains("+CUSD:") || sb.contains("+CME ERROR") || sb.contains("+CMS ERROR") {
                    return sb;
                }
            }
            _ => {}
        }
        thread::sleep(Duration::from_millis(120));
    }
    sb
}

pub fn probe_port(port_name: &str) -> Option<String> {
    match open_port(port_name) {
        Ok(_) => None,
        Err(e) => Some(e),
    }
}

pub fn read_port(port_name: &str) -> ReadResult {
    match open_port(port_name) {
        Ok(mut sp) => {
            send(&mut sp, "ATE0;+CMGF=1;+CSCS=\"UCS2\"", 4000);
            let r = send(&mut sp, "AT+CMGL=\"ALL\"", 15000);
            if r.contains("+CMGL:") {
                let msgs = decoder::parse_text_mode_list(&r, port_name);
                let _ = send(&mut sp, "AT+CSCS=\"GSM\"", 1500);
                return ReadResult { ok: true, messages: msgs, error: None };
            }
            if r.contains("OK") {
                let _ = send(&mut sp, "AT+CSCS=\"GSM\"", 1500);
                return ReadResult { ok: true, messages: vec![], error: None };
            }
            send(&mut sp, "AT+CMGF=0", 4000);
            let r2 = send(&mut sp, "AT+CMGL=4", 15000);
            if r2.contains("+CMGL:") {
                let msgs = decoder::parse_pdu_list(&r2, port_name);
                let _ = send(&mut sp, "AT+CSCS=\"GSM\"", 1500);
                return ReadResult { ok: true, messages: msgs, error: None };
            }
            let _ = send(&mut sp, "AT+CSCS=\"GSM\"", 1500);
            ReadResult { ok: false, messages: vec![], error: Some("Modem not responding".into()) }
        }
        Err(e) => ReadResult { ok: false, messages: vec![], error: Some(e) },
    }
}

pub fn delete_messages(port_name: &str, indices: Option<&[i32]>) -> OpResult {
    match open_port(port_name) {
        Ok(mut sp) => {
            send(&mut sp, "ATE0", 3000);
            send(&mut sp, "AT+CMGF=1", 3000);
            match indices {
                None => {
                    let r = send(&mut sp, "AT+CMGD=1,4", 5000);
                    if r.contains("OK") {
                        return OpResult { ok: true, error: None, deleted: 0, indices: vec![] };
                    }
                    send(&mut sp, "AT+CSCS=\"UCS2\"", 2000);
                    let lst = send(&mut sp, "AT+CMGL=\"ALL\"", 15000);
                    let idxs = decoder::parse_indices(&lst);
                    for idx in &idxs {
                        send(&mut sp, &format!("AT+CMGD={idx}"), 3000);
                    }
                    OpResult { ok: true, error: None, deleted: idxs.len(), indices: idxs }
                }
                Some(idxs) => {
                    for idx in idxs {
                        send(&mut sp, &format!("AT+CMGD={idx}"), 3000);
                    }
                    OpResult { ok: true, error: None, deleted: idxs.len(), indices: idxs.to_vec() }
                }
            }
        }
        Err(e) => OpResult { ok: false, error: Some(e), deleted: 0, indices: vec![] },
    }
}

pub fn get_sim_number(port_name: &str) -> (Option<String>, Option<String>) {
    match open_port(port_name) {
        Ok(mut sp) => {
            send(&mut sp, "ATE0", 3000);
            send(&mut sp, "AT+CSCS=\"GSM\"", 3000);
            let r = send_ussd(&mut sp, "*88#", 9000);
            let mut num = decoder::extract_number_from_ussd(&r);
            if num.is_none() {
                let r2 = send_ussd(&mut sp, "*124#", 9000);
                num = decoder::extract_number_from_ussd(&r2);
            }
            let _ = sp.write_all(b"AT+CUSD=2\r");
            let _ = send(&mut sp, "AT+CSCS=\"GSM\"", 1000);
            match num {
                Some(n) => (Some(decoder::normalize_number(&n)), None),
                None => (None, Some("No number returned".into())),
            }
        }
        Err(e) => (None, Some(e)),
    }
}

pub fn expire_old(port_name: &str, cutoff_ms: i64) -> OpResult {
    let r = read_port(port_name);
    if !r.ok {
        return OpResult { ok: false, error: r.error, deleted: 0, indices: vec![] };
    }
    let old: Vec<i32> = r.messages.iter()
        .filter(|m| m.received.timestamp_millis() > 0 && m.received.timestamp_millis() < cutoff_ms)
        .map(|m| m.index)
        .collect();
    if old.is_empty() {
        return OpResult { ok: true, error: None, deleted: 0, indices: vec![] };
    }
    let d = delete_messages(port_name, Some(&old));
    OpResult { ok: d.ok, error: d.error, deleted: if d.ok { old.len() } else { 0 }, indices: d.indices }
}