use crate::core::at;
use crate::core::decoder;
use crate::core::models::SmsMessage;
use std::time::{Duration, Instant};

pub const BAUD: u32 = 115200;

pub type Port = Box<dyn serialport::SerialPort>;

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
            ports
                .into_iter()
                .filter_map(|p| {
                    let name = p.port_name;
                    if name.starts_with("COM") || name.contains("ttyUSB") || name.contains("ttyACM")
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
    s.chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

/// Best-effort stable identity for a serial port.
///
/// On Linux we walk `/dev/serial/by-path/<topological-id> -> ../../ttyUSBx`
/// and return the symlink *name* (e.g. `pci-0000:03:00.3-usb-0:4.1:1.0-port0`)
/// as the stable key. The name reflects physical USB topology, so ttyUSB
/// renumbering after a reboot/hotplug can't scramble SIM-number assignments.
/// When no by-path link matches (other OSes, exotic setups) we fall back to the
/// mutable name — behaviour identical to before this feature was added.
pub fn stable_id(name: &str) -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(entries) = std::fs::read_dir("/dev/serial/by-path") {
            for e in entries.flatten() {
                let Ok(target) = std::fs::read_link(e.path()) else {
                    continue;
                };
                let matches = target.file_name().map(|f| f == name).unwrap_or(false);
                if matches {
                    if let Some(id) = e.file_name().to_str() {
                        return id.to_string();
                    }
                }
            }
        }
        name.to_string()
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = name;
        name.to_string()
    }
}

pub fn open_port(name: &str) -> Result<Port, String> {
    match serialport::new(name, BAUD)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .timeout(Duration::from_millis(100))
        .open()
    {
        Ok(sp) => {
            log::debug!("Port opened: {}", name);
            Ok(sp)
        }
        Err(e) => {
            let msg = format!("Cannot open {}: {}", name, e);
            log::warn!("{}", msg);
            Err(msg)
        }
    }
}

pub fn probe_port(port_name: &str) -> Option<String> {
    open_port(port_name).err()
}

pub fn read_port(port_name: &str) -> ReadResult {
    let mut ch = match at::AtChannel::open(port_name) {
        Ok(ch) => ch,
        Err(e) => {
            return ReadResult {
                ok: false,
                messages: vec![],
                error: Some(e),
            }
        }
    };

    // PDU mode first: it exposes the UDH needed to reassemble long
    // (concatenated) SMS instead of showing truncated fragments.
    if ch.send("ATE0;+CMGF=0", 4000).contains("OK") {
        let r = ch.send("AT+CMGL=4", 15000);
        if r.contains("+CMGL:") || r.contains("OK") {
            let mut msgs: Vec<SmsMessage> = Vec::new();
            let mut asm = crate::core::reassemble::Reassembler::new();
            for d in decoder::parse_pdu_list(&r, port_name) {
                match d.concat {
                    Some(c) => {
                        if let Some(done) = asm.push(&d.message, c) {
                            msgs.push(done);
                        }
                    }
                    None => msgs.push(d.message),
                }
            }
            msgs.extend(asm.flush_stale(Duration::ZERO));
            log::info!("{}: pdu-mode read -> {} msg(s)", port_name, msgs.len());
            let _ = ch.send("AT+CSCS=\"GSM\"", 1000);
            let _ = ch.send("AT+CMGF=1", 1000);
            return ReadResult {
                ok: true,
                messages: msgs,
                error: None,
            };
        }
    }

    log::debug!("{}: falling back to text mode", port_name);
    ch.send("AT+CMGF=1;+CSCS=\"UCS2\"", 4000);
    let r = ch.send("AT+CMGL=\"ALL\"", 15000);

    if r.contains("+CMGL:") {
        let msgs = decoder::parse_text_mode_list(&r, port_name);
        log::info!("{}: text-mode read -> {} msg(s)", port_name, msgs.len());
        ch.send("AT+CSCS=\"GSM\"", 1500);
        return ReadResult {
            ok: true,
            messages: msgs,
            error: None,
        };
    }
    if r.contains("OK") {
        log::debug!("{}: text-mode read -> no messages", port_name);
        ch.send("AT+CSCS=\"GSM\"", 1500);
        return ReadResult {
            ok: true,
            messages: vec![],
            error: None,
        };
    }

    log::warn!("{}: modem not responding", port_name);
    let _ = ch.send("AT+CSCS=\"GSM\"", 1000);
    ReadResult {
        ok: false,
        messages: vec![],
        error: Some("Modem not responding".into()),
    }
}

pub fn delete_messages(port_name: &str, indices: Option<&[i32]>) -> OpResult {
    let mut ch = match at::AtChannel::open(port_name) {
        Ok(ch) => ch,
        Err(e) => {
            return OpResult {
                ok: false,
                error: Some(e),
                deleted: 0,
                indices: vec![],
            }
        }
    };

    ch.send("ATE0", 3000);
    ch.send("AT+CMGF=1", 3000);

    match indices {
        None => {
            let r = ch.send("AT+CMGD=1,4", 5000);
            if r.contains("OK") {
                log::info!("{}: deleted all messages (bulk)", port_name);
                return OpResult {
                    ok: true,
                    error: None,
                    deleted: 0,
                    indices: vec![],
                };
            }
            ch.send("AT+CSCS=\"UCS2\"", 2000);
            let lst = ch.send("AT+CMGL=\"ALL\"", 15000);
            let idxs = decoder::parse_indices(&lst);
            for idx in &idxs {
                ch.send(&format!("AT+CMGD={idx}"), 3000);
            }
            log::info!("{}: deleted {} msg(s) (one-by-one)", port_name, idxs.len());
            OpResult {
                ok: true,
                error: None,
                deleted: idxs.len(),
                indices: idxs,
            }
        }
        Some(idxs) => {
            for idx in idxs {
                ch.send(&format!("AT+CMGD={idx}"), 3000);
            }
            log::info!("{}: deleted {} msg(s)", port_name, idxs.len());
            OpResult {
                ok: true,
                error: None,
                deleted: idxs.len(),
                indices: idxs.to_vec(),
            }
        }
    }
}

pub fn get_sim_number(port_name: &str) -> (Option<String>, Option<String>) {
    let mut ch = match at::AtChannel::open(port_name) {
        Ok(ch) => ch,
        Err(e) => return (None, Some(e)),
    };

    ch.send("ATE0", 3000);
    ch.send("AT+CSCS=\"GSM\"", 3000);

    let mut num = ussd_query(&mut ch, "*88#", 9000);
    if num.is_none() {
        num = ussd_query(&mut ch, "*124#", 9000);
    }

    let _ = ch.send("AT+CUSD=2", 2000);
    let _ = ch.send("AT+CSCS=\"GSM\"", 1000);

    match num {
        Some(n) => {
            let normalized = decoder::normalize_number(&n);
            log::info!("{}: SIM number {}", port_name, normalized);
            (Some(normalized), None)
        }
        None => {
            log::warn!("{}: no SIM number returned", port_name);
            (None, Some("No number returned".into()))
        }
    }
}

fn ussd_query(ch: &mut at::AtChannel, code: &str, timeout_ms: u64) -> Option<String> {
    let _stale = ch.take_notifications();
    let resp = ch.send(&format!("AT+CUSD=1,\"{code}\",15"), timeout_ms);
    if let Some(err_line) = resp
        .lines()
        .find(|l| l.starts_with("+CME ERROR") || l.starts_with("+CMS ERROR"))
    {
        log::warn!("{}: USSD {} rejected ({})", ch.name, code, err_line.trim());
        ch.take_notifications();
        return None;
    }

    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            log::warn!(
                "{}: USSD {} no reply within {}s",
                ch.name,
                code,
                timeout_ms / 1000
            );
            return None;
        }
        match ch.wait_notification(remaining.as_millis() as u64) {
            Some(note) => {
                if !note.contains('"') {
                    continue;
                }
                match decoder::extract_number_from_ussd(&note) {
                    Some(n) => return Some(n),
                    None => {
                        log::warn!(
                            "{}: USSD {} replied without a number: {}",
                            ch.name,
                            code,
                            at::preview(&note, 100)
                        );
                        return None;
                    }
                }
            }
            None => continue,
        }
    }
}

pub fn expire_old(port_name: &str, cutoff_ms: i64) -> OpResult {
    let r = read_port(port_name);
    if !r.ok {
        return OpResult {
            ok: false,
            error: r.error,
            deleted: 0,
            indices: vec![],
        };
    }
    let old: Vec<i32> = {
        let mut ids: Vec<i32> = Vec::new();
        for m in r.messages.iter().filter(|m| {
            let t = m.received.timestamp_millis();
            t > 0 && t < cutoff_ms
        }) {
            if m.part_indices.len() > 1 {
                ids.extend(m.part_indices.iter().copied());
            } else {
                ids.push(m.index);
            }
        }
        ids.sort_unstable();
        ids.dedup();
        ids
    };
    if old.is_empty() {
        return OpResult {
            ok: true,
            error: None,
            deleted: 0,
            indices: vec![],
        };
    }
    let d = delete_messages(port_name, Some(&old));
    OpResult {
        ok: d.ok,
        error: d.error,
        deleted: if d.ok { old.len() } else { 0 },
        indices: d.indices,
    }
}
