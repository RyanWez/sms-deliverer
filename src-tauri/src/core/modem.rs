use crate::core::at;
use crate::core::decoder;
use crate::core::models::SmsMessage;
use std::time::{Duration, Instant};

pub const BAUD: u32 = 115200;

/// Error text used whenever a port has no modem answering on it. The frontend
/// keys "empty slot" styling off this exact string, so keep the two in sync.
pub const NOT_RESPONDING: &str = "Modem not responding";

/// Timeout for a single liveness probe.
///
/// A powered, enumerated modem answers a bare `AT` in a few milliseconds. An
/// empty SIM slot still exposes a `/dev/ttyUSB*` node — the device file is
/// created by the USB bridge, not by the presence of a SIM — but never answers
/// at all. Waiting longer than this therefore buys nothing and multiplies
/// across every empty slot in the bank.
pub const PROBE_TIMEOUT_MS: u64 = 800;

/// Probe attempts before declaring a port dead. Two, because the first `AT`
/// after opening can be swallowed while the modem drains its own boot chatter.
pub const PROBE_ATTEMPTS: usize = 2;

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

/// Is a modem actually answering on this already-open channel?
///
/// This is the gate that keeps an empty SIM slot from costing a full AT
/// conversation. Without it every command path ran its whole sequence against
/// silence and paid the sum of its timeouts — 24 s for a scan, 35 s for a USSD
/// number lookup — per empty slot, which is what turned a 64-stick bank with 7
/// SIMs into a multi-minute wait.
///
/// Any final result code counts as alive: a modem that answers `ERROR` is
/// present and worth talking to, it just dislikes the command.
pub fn probe_channel(ch: &mut at::AtChannel) -> bool {
    probe_channel_with(ch, PROBE_TIMEOUT_MS, PROBE_ATTEMPTS)
}

fn probe_channel_with(ch: &mut at::AtChannel, timeout_ms: u64, attempts: usize) -> bool {
    for _ in 0..attempts {
        if ch.is_dead() {
            return false;
        }
        let resp = ch.send("AT", timeout_ms);
        if resp.lines().any(|l| at::is_final(l.trim())) {
            return true;
        }
    }
    false
}

/// Open `port_name` and report whether a modem answers on it.
///
/// `Ok(false)` means the node exists but nothing replied (empty slot, unpowered
/// stick); `Err` means the device could not be opened at all (unplugged,
/// permissions, already held by another process).
pub fn probe_port(port_name: &str) -> Result<bool, String> {
    let mut ch = at::AtChannel::open(port_name)?;
    let alive = probe_channel(&mut ch);
    log::debug!("{}: probe -> {}", port_name, if alive { "alive" } else { "silent" });
    Ok(alive)
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
    if !probe_channel(&mut ch) {
        log::warn!("{}: {} (no reply to AT)", port_name, NOT_RESPONDING);
        return ReadResult {
            ok: false,
            messages: vec![],
            error: Some(NOT_RESPONDING.into()),
        };
    }

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

    // Reached only when the modem answered `AT` but then went quiet on the SMS
    // commands — a wedged stick rather than an empty slot.
    log::warn!("{}: stopped answering after probe", port_name);
    let _ = ch.send("AT+CSCS=\"GSM\"", 1000);
    ReadResult {
        ok: false,
        messages: vec![],
        error: Some("Modem stopped responding mid-read".into()),
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

    if !probe_channel(&mut ch) {
        log::warn!("{}: {} (no reply to AT)", port_name, NOT_RESPONDING);
        return OpResult {
            ok: false,
            error: Some(NOT_RESPONDING.into()),
            deleted: 0,
            indices: vec![],
        };
    }

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

    // Probe before anything else: `ATE0` and `AT+CSCS` are 3 s each, so running
    // them first would hand every empty slot a 6 s bill before we even learn
    // there is nothing there.
    if !probe_channel(&mut ch) {
        log::warn!("{}: {} (no reply to AT)", port_name, NOT_RESPONDING);
        return (None, Some(NOT_RESPONDING.into()));
    }

    ch.send("ATE0", 3000);
    ch.send("AT+CSCS=\"GSM\"", 3000);

    // Network pre-check: USSD fails opaquely (+CME ERROR: 100 or no reply)
    // when the modem has no service, so ask the modem for its registration
    // state and signal level first. Skipping hopeless probes also saves the
    // 2×9 s timeout per port across a 64-stick bank.
    let creg_resp = ch.send("AT+CREG?", 4000);
    let rssi = parse_csq_rssi(&ch.send("AT+CSQ", 4000));
    // A modem that answered `AT` but returns no result code at all for
    // `AT+CREG?` is wedged. This used to fall straight through to the USSD
    // queries — `network_problem(None, None)` reads as "no problem found" — so
    // the pre-check that exists to save 2×9 s never fired on exactly the ports
    // that needed it most.
    if !creg_resp.lines().any(|l| at::is_final(l.trim())) {
        let _ = ch.send("AT+CSCS=\"GSM\"", 1000);
        log::warn!("{}: USSD skipped — no reply to AT+CREG?", port_name);
        return (None, Some("Modem not answering network queries".into()));
    }
    let creg_stat = parse_creg_stat(&creg_resp);
    if let Some(problem) = network_problem(creg_stat, rssi) {
        let _ = ch.send("AT+CSCS=\"GSM\"", 1000);
        log::warn!("{}: USSD skipped — {}", port_name, problem);
        return (None, Some(problem));
    }

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
            // Surface the modem's network state alongside the failure so the
            // operator can tell "carrier not answering" from "no signal".
            let detail = match (creg_stat, rssi) {
                (Some(s), Some(r)) => format!(" (reg stat {}, signal {}/31)", s, r),
                (Some(s), None) => format!(" (reg stat {})", s),
                _ => String::new(),
            };
            (None, Some(format!("No number returned{}", detail)))
        }
    }
}

/// Extract the registration status digit from `+CREG: <n>,<stat>[,...]`.
/// 0 not-registered, 1 home, 2 searching, 3 denied, 4 unknown, 5 roaming.
fn parse_creg_stat(resp: &str) -> Option<u32> {
    let line = resp.lines().find(|l| l.starts_with("+CREG:"))?;
    line.split(',').nth(1)?.trim().parse().ok()
}

/// Extract the RSSI from `+CSQ: <rssi>,<ber>` (0–31, 99 = unknown).
fn parse_csq_rssi(resp: &str) -> Option<u32> {
    let line = resp.lines().find(|l| l.starts_with("+CSQ:"))?;
    line.trim_start_matches("+CSQ:")
        .split(',')
        .next()?
        .trim()
        .parse()
        .ok()
}

/// Map the modem's network state to a human-readable blocker, if any.
/// Registered-and-covered modems return `None` and USSD is attempted.
fn network_problem(creg: Option<u32>, rssi: Option<u32>) -> Option<String> {
    match creg {
        Some(0) => Some("No network — not registered, not searching".into()),
        Some(2) => Some("No network — still searching for carrier".into()),
        Some(3) => Some("No network — registration denied (check SIM/account)".into()),
        Some(4) => Some("No network — registration state unknown".into()),
        // 1 = home network, 5 = roaming → registered; check signal next.
        _ => match rssi {
            Some(0) => Some("No signal (CSQ 0) — check antenna/coverage".into()),
            _ => None,
        },
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

/// Delete every message on `port_name` received before `cutoff_ms`.
///
/// Used for SIM housekeeping while live mode is *off* — it opens the port
/// itself, so it must never run against a port a live worker owns (live mode
/// prunes on its own channel instead).
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
    let old = crate::core::models::expired_indices(&r.messages, cutoff_ms);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::at::{AtChannel, FakeTransport};
    use std::time::Instant;

    fn channel(script: &str) -> AtChannel {
        AtChannel::with_transport("ttyUSBtest", Box::new(FakeTransport::new(script, 1024)))
    }

    // ── Liveness probe ──

    #[test]
    fn probe_accepts_a_modem_that_answers_ok() {
        let mut ch = channel("\r\nOK\r\n");
        assert!(probe_channel_with(&mut ch, 50, 2));
    }

    #[test]
    fn probe_accepts_a_modem_that_answers_error() {
        // Present but grumpy still means there is something to talk to; treating
        // ERROR as dead would skip a working stick in a bad command state.
        let mut ch = channel("\r\n+CME ERROR: 3\r\n");
        assert!(probe_channel_with(&mut ch, 50, 2));
    }

    #[test]
    fn probe_rejects_a_silent_port() {
        // An empty SIM slot: the tty node reads cleanly and never answers.
        let mut ch = channel("");
        assert!(!probe_channel_with(&mut ch, 50, 2));
    }

    #[test]
    fn probe_rejects_unsolicited_chatter_without_a_result_code() {
        // A stick emitting RING but no OK has not acknowledged our command.
        let mut ch = channel("\r\nRING\r\n");
        assert!(!probe_channel_with(&mut ch, 50, 2));
    }

    #[test]
    fn probe_retries_when_the_first_at_is_swallowed() {
        // This is the whole reason PROBE_ATTEMPTS is 2: a modem still draining
        // boot chatter can eat the first command.
        let mut ch = AtChannel::with_transport(
            "ttyUSBtest",
            Box::new(FakeTransport::silent_for_writes("\r\nOK\r\n", 2)),
        );
        assert!(probe_channel_with(&mut ch, 50, 2));
    }

    #[test]
    fn probe_gives_up_within_its_own_budget() {
        // The point of the probe is that a dead port costs ~1 s instead of the
        // 24-35 s an ungated command sequence used to spend on it.
        let mut ch = channel("");
        let start = Instant::now();
        assert!(!probe_channel_with(&mut ch, 100, 2));
        assert!(
            start.elapsed() < Duration::from_millis(1000),
            "probe overran its budget: {:?}",
            start.elapsed()
        );
    }

    #[test]
    fn probe_stops_immediately_on_a_dead_channel() {
        let mut ch = AtChannel::with_transport("ttyUSBtest", Box::new(FakeTransport::failing()));
        assert!(!probe_channel_with(&mut ch, 500, 4));
        assert!(ch.is_dead());
    }

    // ── Network pre-check ──

    #[test]
    fn network_problem_flags_unregistered_states() {
        assert!(network_problem(Some(0), Some(20)).is_some());
        assert!(network_problem(Some(2), Some(20)).is_some());
        assert!(network_problem(Some(3), Some(20)).is_some());
        assert!(network_problem(Some(4), Some(20)).is_some());
    }

    #[test]
    fn network_problem_passes_registered_modems_with_signal() {
        assert!(network_problem(Some(1), Some(20)).is_none());
        assert!(network_problem(Some(5), Some(7)).is_none());
        // 99 = "unknown", not "no signal" — let USSD try rather than guessing.
        assert!(network_problem(Some(1), Some(99)).is_none());
    }

    #[test]
    fn network_problem_flags_zero_signal() {
        assert!(network_problem(Some(1), Some(0)).is_some());
    }

    #[test]
    fn creg_and_csq_parse_real_responses() {
        assert_eq!(parse_creg_stat("+CREG: 0,1\r\nOK\r\n"), Some(1));
        assert_eq!(parse_creg_stat("+CREG: 2,5,\"1A2B\",\"3C4D\"\r\n"), Some(5));
        assert_eq!(parse_creg_stat("OK\r\n"), None);
        assert_eq!(parse_csq_rssi("+CSQ: 17,99\r\nOK\r\n"), Some(17));
        assert_eq!(parse_csq_rssi("+CSQ: 0,0\r\n"), Some(0));
        assert_eq!(parse_csq_rssi("OK\r\n"), None);
    }

    // ── Port naming ──

    #[test]
    fn port_num_reads_the_trailing_digits() {
        assert_eq!(port_num("/dev/ttyUSB20"), 20);
        assert_eq!(port_num("COM7"), 7);
        assert_eq!(port_num("/dev/ttyACM0"), 0);
        assert_eq!(port_num("no-digits"), 0);
    }
}
