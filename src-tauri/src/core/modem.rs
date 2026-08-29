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

/// Device-node basename: `/dev/ttyUSB7` → `ttyUSB7`, `COM3` → `COM3`.
///
/// `serialport` hands us full device paths on Linux while `/dev/serial/by-path`
/// symlinks point at `../../ttyUSB7`, so the two must be compared on the
/// basename. Comparing the raw strings never matched, which silently reduced
/// every "stable" id below to the mutable tty name.
pub fn device_basename(name: &str) -> &str {
    name.rsplit('/').next().unwrap_or(name)
}

/// Best-effort stable identity for a serial port.
///
/// On Linux we walk `/dev/serial/by-path/<topological-id> -> ../../ttyUSBx`
/// and return the symlink *name* (e.g. `pci-0000:03:00.3-usb-0:4.1:1.0-port0`)
/// as the stable key. The name reflects physical USB topology, so ttyUSB
/// renumbering after a reboot/hotplug can't scramble SIM-number assignments.
/// When no by-path link matches (other OSes, exotic setups) we fall back to the
/// mutable name.
pub fn stable_id(name: &str) -> String {
    #[cfg(target_os = "linux")]
    {
        stable_id_in(std::path::Path::new("/dev/serial/by-path"), name)
            .unwrap_or_else(|| name.to_string())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = name;
        name.to_string()
    }
}

/// The by-path lookup, with the directory injected so it can be tested.
#[cfg(target_os = "linux")]
fn stable_id_in(dir: &std::path::Path, name: &str) -> Option<String> {
    let want = device_basename(name);
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let Ok(target) = std::fs::read_link(e.path()) else {
            continue;
        };
        if target.file_name().map(|f| f == want).unwrap_or(false) {
            return e.file_name().to_str().map(|s| s.to_string());
        }
    }
    None
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
/// What a single probe learned about a port.
pub struct ProbeResult {
    pub alive: bool,
    /// The SIM's own serial number, when the modem would give it up. This is the
    /// only identity that survives both ttyUSB renumbering and a SIM being moved
    /// to another slot, so it — not the port — is what a phone number is filed
    /// against.
    pub iccid: Option<String>,
}

pub fn probe_port(port_name: &str) -> Result<ProbeResult, String> {
    let mut ch = at::AtChannel::open(port_name)?;
    let alive = probe_channel(&mut ch);
    log::debug!(
        "{}: probe -> {}",
        port_name,
        if alive { "alive" } else { "silent" }
    );
    let iccid = if alive { read_iccid(&mut ch) } else { None };
    Ok(ProbeResult { alive, iccid })
}

/// Read the SIM's ICCID. Cheap (a file on the card, no network) but not
/// universally spelled the same, so try the three common forms and stop at the
/// first that yields digits.
fn read_iccid(ch: &mut at::AtChannel) -> Option<String> {
    for cmd in ["AT+CCID", "AT+ICCID", "AT^ICCID"] {
        let resp = ch.send(cmd, 1500);
        if let Some(id) = decoder::extract_iccid(&resp) {
            return Some(id);
        }
    }
    None
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

/// Delete one SIM slot and report whether the modem actually confirmed it.
///
/// `AT+CMGD` can fail per index — the slot may already be empty, or the SIM may
/// reject the write — and the reply is the only evidence either way. The caller
/// used to ignore it and report every requested index as deleted, so the UI's
/// "Deleted: 1 ok" said nothing about the state of the SIM.
fn delete_index(ch: &mut at::AtChannel, idx: i32) -> bool {
    ch.send(&format!("AT+CMGD={idx}"), 3000)
        .lines()
        .any(|l| l.trim() == "OK")
}

/// Delete the given SIM slots, highest index first, and return the ones the
/// modem confirmed.
///
/// Highest-first matches the live worker: some modems compact their index space
/// after a delete, and walking down means a shift can never move a slot we have
/// not visited yet.
fn delete_each(ch: &mut at::AtChannel, indices: &[i32]) -> Vec<i32> {
    let mut order: Vec<i32> = indices.to_vec();
    order.sort_unstable_by(|a, b| b.cmp(a));
    order.retain(|idx| delete_index(ch, *idx));
    order
}

/// Which of `wanted` are still occupied, re-read from the SIM.
///
/// The per-command reply is not enough on its own: some modems drop every
/// fragment of a concatenated message when handed the index of any one of them,
/// so the follow-up `AT+CMGD` for its siblings answers `+CMS ERROR: 321` — a
/// refusal that means "already gone", not "still there". Absence from the list
/// is the only evidence that distinguishes the two. `None` means the list itself
/// was unusable, leaving the caller on its per-command count.
fn slots_still_present(ch: &mut at::AtChannel, wanted: &[i32]) -> Option<Vec<i32>> {
    let lst = ch.send("AT+CMGL=\"ALL\"", 15000);
    if !lst.lines().any(|l| l.trim() == "OK") {
        return None;
    }
    let present = decoder::parse_indices(&lst);
    Some(
        wanted
            .iter()
            .copied()
            .filter(|i| present.contains(i))
            .collect(),
    )
}

/// Turn a delete attempt into a result, preferring the SIM's own account of
/// which slots survived over the per-command replies.
fn confirm_delete(
    ch: &mut at::AtChannel,
    port_name: &str,
    wanted: &[i32],
    confirmed: Vec<i32>,
) -> OpResult {
    let Some(left) = slots_still_present(ch, wanted) else {
        log::warn!(
            "{}: deleted {} msg(s) — could not re-read the SIM to confirm",
            port_name,
            confirmed.len()
        );
        return OpResult {
            ok: !confirmed.is_empty(),
            error: None,
            deleted: confirmed.len(),
            indices: confirmed,
        };
    };

    let gone: Vec<i32> = wanted
        .iter()
        .copied()
        .filter(|i| !left.contains(i))
        .collect();
    if left.is_empty() {
        log::info!("{}: deleted {} msg(s)", port_name, gone.len());
        return OpResult {
            ok: true,
            error: None,
            deleted: gone.len(),
            indices: gone,
        };
    }
    log::warn!(
        "{}: deleted {}/{} msg(s) — still on SIM: {:?}",
        port_name,
        gone.len(),
        wanted.len(),
        left
    );
    OpResult {
        ok: !gone.is_empty(),
        error: Some(format!("Deleted {}/{} from SIM", gone.len(), wanted.len())),
        deleted: gone.len(),
        indices: gone,
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
            let found = decoder::parse_indices(&lst);
            if found.is_empty() {
                log::info!("{}: nothing left to delete", port_name);
                return OpResult {
                    ok: true,
                    error: None,
                    deleted: 0,
                    indices: vec![],
                };
            }
            let confirmed = delete_each(&mut ch, &found);
            confirm_delete(&mut ch, port_name, &found, confirmed)
        }
        Some(idxs) => {
            let confirmed = delete_each(&mut ch, idxs);
            confirm_delete(&mut ch, port_name, idxs, confirmed)
        }
    }
}

/// What one "Get SIM Numbers" pass learned about a port.
pub struct SimIdentity {
    pub number: Option<String>,
    /// Read whenever the modem answered at all, so the number can be filed
    /// against the card rather than against the tty name.
    pub iccid: Option<String>,
    pub error: Option<String>,
}

impl SimIdentity {
    fn failed(iccid: Option<String>, error: impl Into<String>) -> Self {
        Self {
            number: None,
            iccid,
            error: Some(error.into()),
        }
    }
}

pub fn get_sim_number(port_name: &str) -> SimIdentity {
    let mut ch = match at::AtChannel::open(port_name) {
        Ok(ch) => ch,
        Err(e) => return SimIdentity::failed(None, e),
    };

    // Probe before anything else: `ATE0` and `AT+CSCS` are 3 s each, so running
    // them first would hand every empty slot a 6 s bill before we even learn
    // there is nothing there.
    if !probe_channel(&mut ch) {
        log::warn!("{}: {} (no reply to AT)", port_name, NOT_RESPONDING);
        return SimIdentity::failed(None, NOT_RESPONDING);
    }

    ch.send("ATE0", 3000);
    ch.send("AT+CSCS=\"GSM\"", 3000);

    // Identify the card before asking anything about the number. Whatever is
    // learned below belongs to this ICCID, not to whichever tty name the stick
    // happens to have today.
    let iccid = read_iccid(&mut ch);

    // Cancel any USSD session left open by an earlier run, or by a crash in the
    // middle of a dialogue. Firmware that still believes a session is active
    // answers the next `AT+CUSD=1` with an immediate `+CME ERROR: 100`, which is
    // exactly what a bank of otherwise-registered sticks was reporting.
    let _ = ch.send("AT+CUSD=2", 1500);

    // EF_MSISDN first: it is a file on the SIM, so it needs no network, spends no
    // USSD dialogue and answers in milliseconds. Operators often leave it blank
    // on prepaid SIMs, so a bare `OK` here is normal and not a fault.
    let cnum_resp = ch.send("AT+CNUM", 2000);
    if let Some(n) = decoder::extract_number_from_cnum(&cnum_resp) {
        let normalized = decoder::normalize_number(&n);
        log::info!("{}: SIM number {} (AT+CNUM)", port_name, normalized);
        return SimIdentity {
            number: Some(normalized),
            iccid,
            error: None,
        };
    }

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
        return SimIdentity::failed(iccid, "Modem not answering network queries");
    }
    let creg_stat = parse_creg_stat(&creg_resp);
    if let Some(problem) = network_problem(creg_stat, rssi) {
        let _ = ch.send("AT+CSCS=\"GSM\"", 1000);
        log::warn!("{}: USSD skipped — {}", port_name, problem);
        return SimIdentity::failed(iccid, problem);
    }

    let mut num = None;
    for code in OWN_NUMBER_USSD_CODES {
        num = ussd_query(&mut ch, code, 9000);
        if num.is_some() {
            break;
        }
    }

    let _ = ch.send("AT+CUSD=2", 2000);
    let _ = ch.send("AT+CSCS=\"GSM\"", 1000);

    match num {
        Some(n) => {
            let normalized = decoder::normalize_number(&n);
            log::info!("{}: SIM number {}", port_name, normalized);
            SimIdentity {
                number: Some(normalized),
                iccid,
                error: None,
            }
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
            SimIdentity::failed(iccid, format!("No number returned{}", detail))
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

/// USSD codes that return (or echo) the subscriber's own number, tried in
/// order. Mytel is the only carrier in this deployment: `*88#` is its
/// own-number code, and `*124#` is the balance dialogue, whose reply text
/// usually carries the MSISDN too. Each entry costs up to 9 s on a port that
/// never answers, so keep the list short and the most reliable code first.
const OWN_NUMBER_USSD_CODES: [&str; 2] = ["*88#", "*124#"];

enum UssdOutcome {
    Number(String),
    /// The modem refused the command itself (`+CME`/`+CMS ERROR`).
    Rejected,
    /// No usable reply: silence until the deadline, or a dialogue with no number.
    NoAnswer,
}

fn ussd_query(ch: &mut at::AtChannel, code: &str, timeout_ms: u64) -> Option<String> {
    // `,15` names the plain-GSM data coding scheme and is what most firmware
    // wants, but some rejects the argument outright. A rejection comes back in
    // milliseconds, so retrying the bare form costs almost nothing and recovers
    // sticks that would otherwise report no number at all.
    match ussd_attempt(ch, &format!("AT+CUSD=1,\"{code}\",15"), code, timeout_ms) {
        UssdOutcome::Number(n) => Some(n),
        UssdOutcome::NoAnswer => None,
        UssdOutcome::Rejected => {
            let _ = ch.send("AT+CUSD=2", 1000);
            match ussd_attempt(ch, &format!("AT+CUSD=1,\"{code}\""), code, timeout_ms) {
                UssdOutcome::Number(n) => Some(n),
                _ => None,
            }
        }
    }
}

fn ussd_attempt(ch: &mut at::AtChannel, command: &str, code: &str, timeout_ms: u64) -> UssdOutcome {
    let _stale = ch.take_notifications();
    let resp = ch.send(command, timeout_ms);
    if let Some(err_line) = resp
        .lines()
        .find(|l| l.starts_with("+CME ERROR") || l.starts_with("+CMS ERROR"))
    {
        log::warn!("{}: USSD {} rejected ({})", ch.name, code, err_line.trim());
        ch.take_notifications();
        return UssdOutcome::Rejected;
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
            return UssdOutcome::NoAnswer;
        }
        match ch.wait_notification(remaining.as_millis() as u64) {
            Some(note) => {
                if !note.contains('"') {
                    continue;
                }
                match decoder::extract_number_from_ussd(&note) {
                    Some(n) => return UssdOutcome::Number(n),
                    None => {
                        log::warn!(
                            "{}: USSD {} replied without a number: {}",
                            ch.name,
                            code,
                            at::preview(&note, 100)
                        );
                        return UssdOutcome::NoAnswer;
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
        deleted: d.deleted,
        indices: d.indices,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::at::{AtChannel, FakeTransport, Transport};
    use std::io;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    fn channel(script: &str) -> AtChannel {
        AtChannel::with_transport("ttyUSBtest", Box::new(FakeTransport::new(script, 1024)))
    }

    /// A SIM with a known set of occupied slots.
    ///
    /// `group` models the modem behaviour that made the field logs read
    /// `deleted 3/6`: deleting any member of a concatenated message removes all
    /// of them, so the siblings then answer `+CMS ERROR: 321`. `AT+CMGL="ALL"`
    /// reports whatever is left, which is what `confirm_delete` goes on.
    struct SimTransport {
        slots: Vec<i32>,
        group: Vec<i32>,
        pending: Vec<u8>,
        sent: Arc<Mutex<Vec<String>>>,
    }

    impl SimTransport {
        fn reply_to(&mut self, cmd: &str) -> Vec<u8> {
            if cmd == "AT+CMGL=\"ALL\"" {
                let mut out = String::new();
                for idx in &self.slots {
                    out.push_str(&format!(
                        "\r\n+CMGL: {idx},\"REC READ\",\"966\",,\"26/08/29,11:00:00+00\"\r\ntext\r\n"
                    ));
                }
                out.push_str("\r\nOK\r\n");
                return out.into_bytes();
            }
            let Some(idx) = cmd
                .strip_prefix("AT+CMGD=")
                .and_then(|n| n.parse::<i32>().ok())
            else {
                return b"\r\nOK\r\n".to_vec();
            };
            if !self.slots.contains(&idx) {
                return b"\r\n+CMS ERROR: 321\r\n".to_vec();
            }
            if self.group.contains(&idx) {
                self.slots.retain(|s| !self.group.contains(s));
            } else {
                self.slots.retain(|s| *s != idx);
            }
            b"\r\nOK\r\n".to_vec()
        }
    }

    impl Transport for SimTransport {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.pending.is_empty() {
                return Err(io::ErrorKind::TimedOut.into());
            }
            let n = self.pending.len().min(buf.len());
            buf[..n].copy_from_slice(&self.pending[..n]);
            self.pending.drain(..n);
            Ok(n)
        }

        fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
            let cmd = String::from_utf8_lossy(data).trim().to_string();
            self.sent.lock().unwrap().push(cmd.clone());
            let reply = self.reply_to(&cmd);
            self.pending.extend_from_slice(&reply);
            Ok(())
        }
    }

    fn sim_channel(slots: &[i32], group: &[i32]) -> (AtChannel, Arc<Mutex<Vec<String>>>) {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let ch = AtChannel::with_transport(
            "ttyUSBtest",
            Box::new(SimTransport {
                slots: slots.to_vec(),
                group: group.to_vec(),
                pending: Vec::new(),
                sent: Arc::clone(&sent),
            }),
        );
        (ch, sent)
    }

    // ── SIM deletion ──

    #[test]
    fn delete_each_reports_only_the_slots_the_modem_confirmed() {
        let (mut ch, _) = sim_channel(&[1, 7], &[]);
        assert_eq!(delete_each(&mut ch, &[1, 4, 7]), vec![7, 1]);
    }

    #[test]
    fn delete_each_walks_the_highest_slot_first() {
        // A modem that compacts its index space after a delete would otherwise
        // shift a slot we have not visited yet onto a number we already passed.
        let (mut ch, sent) = sim_channel(&[1, 4, 7], &[]);
        delete_each(&mut ch, &[1, 4, 7]);
        assert_eq!(
            *sent.lock().unwrap(),
            vec!["AT+CMGD=7", "AT+CMGD=4", "AT+CMGD=1"]
        );
    }

    #[test]
    fn delete_each_reports_nothing_when_the_sim_refuses_everything() {
        let (mut ch, _) = sim_channel(&[], &[]);
        assert!(delete_each(&mut ch, &[1, 2]).is_empty());
    }

    #[test]
    fn confirm_delete_trusts_the_sim_over_the_refusals() {
        // The field case: slots 3-6 are one concatenated message, so deleting 6
        // takes 5, 4 and 3 with it and those three answer +CMS ERROR. Counting
        // replies reported 3/6; counting what is left on the SIM reports 6.
        let (mut ch, _) = sim_channel(&[1, 2, 3, 4, 5, 6], &[3, 4, 5, 6]);
        let wanted = [1, 2, 3, 4, 5, 6];
        let confirmed = delete_each(&mut ch, &wanted);
        assert_eq!(confirmed.len(), 3);
        let r = confirm_delete(&mut ch, "ttyUSBtest", &wanted, confirmed);
        assert!(r.ok);
        assert_eq!(r.deleted, 6);
        assert!(r.error.is_none());
    }

    #[test]
    fn confirm_delete_reports_slots_that_really_survived() {
        // Slot 9 was asked for but never went away — the case the warning exists
        // for, and the one a "some replies were errors" count cannot tell apart
        // from the group delete above.
        let (mut ch, _) = sim_channel(&[1, 2, 9], &[]);
        let confirmed = delete_each(&mut ch, &[1, 2]);
        let r = confirm_delete(&mut ch, "ttyUSBtest", &[1, 2, 9], confirmed);
        assert!(r.ok);
        assert_eq!(r.deleted, 2);
        assert_eq!(r.error.as_deref(), Some("Deleted 2/3 from SIM"));
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

    #[test]
    fn device_basename_strips_the_directory() {
        assert_eq!(device_basename("/dev/ttyUSB20"), "ttyUSB20");
        assert_eq!(device_basename("COM7"), "COM7");
        assert_eq!(device_basename("ttyUSB3"), "ttyUSB3");
    }

    /// The bug this guards: `serialport` reports `/dev/ttyUSB7` while by-path
    /// symlinks point at `../../ttyUSB7`. Comparing those two strings never
    /// matched, so every port fell back to its mutable tty name as its "stable"
    /// key — and a number learned on one stick reappeared on whichever stick
    /// inherited that name after a hotplug.
    #[cfg(target_os = "linux")]
    #[test]
    fn stable_id_resolves_a_by_path_symlink() {
        use std::os::unix::fs::symlink;
        let dir = std::env::temp_dir().join(format!("sms-bypath-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        symlink("../../ttyUSB7", dir.join("pci-0000:03:00.3-usb-0:4.1:1.0-port0")).unwrap();
        symlink("../../ttyUSB8", dir.join("pci-0000:03:00.3-usb-0:4.1:1.2-port0")).unwrap();

        assert_eq!(
            stable_id_in(&dir, "/dev/ttyUSB7").as_deref(),
            Some("pci-0000:03:00.3-usb-0:4.1:1.0-port0")
        );
        assert_eq!(
            stable_id_in(&dir, "/dev/ttyUSB8").as_deref(),
            Some("pci-0000:03:00.3-usb-0:4.1:1.2-port0")
        );
        assert_eq!(stable_id_in(&dir, "/dev/ttyUSB9"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── ICCID ──

    /// Answers `AT+CCID` with an error and `AT+ICCID` with the card serial, which
    /// is how a good part of the cheap-modem population behaves.
    struct IccidTransport {
        pending: Vec<u8>,
    }

    impl Transport for IccidTransport {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.pending.is_empty() {
                return Err(io::ErrorKind::TimedOut.into());
            }
            let n = self.pending.len().min(buf.len());
            buf[..n].copy_from_slice(&self.pending[..n]);
            self.pending.drain(..n);
            Ok(n)
        }

        fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
            let cmd = String::from_utf8_lossy(data).trim().to_string();
            let reply: &[u8] = match cmd.as_str() {
                "AT+CCID" => b"\r\n+CME ERROR: 4\r\n",
                "AT+ICCID" => b"\r\n+ICCID: 8995010912345678901F\r\n\r\nOK\r\n",
                _ => b"\r\nOK\r\n",
            };
            self.pending.extend_from_slice(reply);
            Ok(())
        }
    }

    #[test]
    fn read_iccid_falls_through_to_the_vendor_spelling() {
        let mut ch = AtChannel::with_transport(
            "ttyUSBtest",
            Box::new(IccidTransport {
                pending: Vec::new(),
            }),
        );
        assert_eq!(read_iccid(&mut ch).as_deref(), Some("8995010912345678901"));
    }

    #[test]
    fn read_iccid_gives_up_quietly_when_nothing_answers() {
        let mut ch = channel("\r\nOK\r\n");
        assert_eq!(read_iccid(&mut ch), None);
    }
}
