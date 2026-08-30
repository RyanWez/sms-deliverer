use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

use super::modem;

const NOTIFICATION_PREFIXES: [&str; 7] = [
    "+CMTI:", "+CUSD:", "+CRING:", "RING", "+CMT:", "+CBM:", "+CDS:",
];

/// Hard cap on the unterminated tail carried in `partial`.
///
/// `ingest` only drains `partial` when it finds a '\n', and `send` deliberately
/// no longer clears it (see the comment there — wiping it destroyed `+CMTI`
/// notifications split across two USB reads). Nothing else empties it, so a
/// modem that streams bytes without ever producing a newline — binary garbage
/// after a USB glitch, firmware wedged mid-transfer — would grow this buffer for
/// the whole life of the channel. Live mode holds a channel open for days across
/// up to 64 ports, so that is unbounded per-port growth.
///
/// Worst-case *legitimate* single line, from the commands this app actually
/// sends:
///   * PDU from `AT+CMGR`/`AT+CMGL=4`: SMSC address ≤ 12 octets + TPDU ≤ 163
///     octets (1 first octet + 12 TP-OA + 1 PID + 1 DCS + 7 SCTS + 1 UDL + 140
///     UD) = 175 octets, hex-encoded → **350 chars**.
///   * Text-mode body under `AT+CSCS="UCS2"`: a 160-character GSM-7 message is
///     re-encoded as UCS2 hex, 4 chars per character → **640 chars**. This, not
///     the PDU, is the real maximum on the read path.
///   * `+CUSD:` reply: a USSD string is ≤ 182 GSM-7 characters, which in UCS2 hex
///     is **~730 chars** on one line.
///
/// 4096 leaves ~5.6× headroom over that 730-char worst case, and even 64 ports
/// each holding a full buffer is a quarter of a megabyte.
const MAX_PARTIAL_BYTES: usize = 4096;

pub trait Transport {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
    fn write_all(&mut self, data: &[u8]) -> io::Result<()>;
}

pub struct SerialTransport(modem::Port);

impl Transport for SerialTransport {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }

    fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        self.0.write_all(data)
    }
}

/// Scripted in-memory transport for tests.
///
/// Replays `script` in `bytes_per_read` chunks and then reports `TimedOut`
/// forever — which is exactly how an empty SIM slot behaves: the tty node reads
/// cleanly and never produces a byte. Lives outside `mod tests` so the modem
/// and live layers can drive an `AtChannel` without touching real hardware.
#[cfg(test)]
pub(crate) struct FakeTransport {
    script: Vec<u8>,
    pos: usize,
    bytes_per_read: usize,
    fail_reads: bool,
    /// Report every write as having sent nothing, the way Windows `WriteFile`
    /// behaves when the port's write timeout expires against a stalled bridge.
    fail_writes: bool,
    /// Withhold the script until this many commands have been written. Models a
    /// modem that swallows the first `AT` while it finishes booting.
    reply_after_writes: usize,
    writes: usize,
}

#[cfg(test)]
impl FakeTransport {
    pub(crate) fn new(script: &str, bytes_per_read: usize) -> Self {
        Self {
            script: script.as_bytes().to_vec(),
            pos: 0,
            bytes_per_read,
            fail_reads: false,
            fail_writes: false,
            reply_after_writes: 0,
            writes: 0,
        }
    }

    /// Silent until `n` commands have been sent, then replays `script`.
    pub(crate) fn silent_for_writes(script: &str, n: usize) -> Self {
        Self {
            reply_after_writes: n,
            ..Self::new(script, 1024)
        }
    }

    pub(crate) fn failing() -> Self {
        Self {
            fail_reads: true,
            ..Self::new("", 1024)
        }
    }

    /// Reads fine, but nothing can be sent. The script would answer if the
    /// command ever reached the modem, so a probe that reports this port alive is
    /// reporting a reply to a command it never managed to write.
    pub(crate) fn write_failing(script: &str) -> Self {
        Self {
            fail_writes: true,
            ..Self::new(script, 1024)
        }
    }
}

#[cfg(test)]
impl Transport for FakeTransport {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.fail_reads {
            return Err(io::Error::other("device lost"));
        }
        if self.writes < self.reply_after_writes {
            return Err(io::ErrorKind::TimedOut.into());
        }
        let remaining = self.script.len() - self.pos;
        if remaining == 0 {
            return Err(io::ErrorKind::TimedOut.into());
        }
        let n = remaining.min(buf.len()).min(self.bytes_per_read);
        buf[..n].copy_from_slice(&self.script[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }

    fn write_all(&mut self, _data: &[u8]) -> io::Result<()> {
        if self.fail_writes {
            return Err(io::Error::new(io::ErrorKind::WriteZero, "write timed out"));
        }
        self.writes += 1;
        Ok(())
    }
}

pub fn preview(s: &str, max_chars: usize) -> String {
    let mut out: String = s
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(max_chars)
        .collect();
    if s.chars().count() > max_chars {
        out.push('…');
    }
    out
}

pub fn is_notification(line: &str) -> bool {
    NOTIFICATION_PREFIXES.iter().any(|p| line.starts_with(p))
}

pub fn is_final(line: &str) -> bool {
    line == "OK"
        || line == "ERROR"
        || line.starts_with("+CME ERROR")
        || line.starts_with("+CMS ERROR")
}

/// Line-oriented AT channel over a blocking serial transport.
///
/// Instead of poll-and-sleep loops, every wait blocks inside `read()`:
/// data arrival wakes it instantly, idle periods cost one syscall per
/// port-timeout slice (~100 ms) and no artificial latency is added.
pub struct AtChannel {
    t: Box<dyn Transport>,
    pub name: String,
    partial: String,
    /// Set after [`MAX_PARTIAL_BYTES`] was exceeded: everything up to and
    /// including the next '\n' is discarded before parsing resumes.
    resyncing: bool,
    response: String,
    notifications: VecDeque<String>,
    done: bool,
    dead: bool,
    /// OS-level I/O error text that killed the channel, if any. Surfaced to the
    /// UI so the operator can tell a real device loss (ENODEV/EIO) from a
    /// transient timeout instead of staring at a bare "Port lost".
    dead_reason: Option<String>,
}

impl AtChannel {
    pub fn open(name: &str) -> Result<Self, String> {
        let port = modem::open_port(name)?;
        Ok(Self {
            t: Box::new(SerialTransport(port)),
            name: name.to_string(),
            partial: String::new(),
            resyncing: false,
            response: String::new(),
            notifications: VecDeque::new(),
            done: false,
            dead: false,
            dead_reason: None,
        })
    }

    #[cfg(test)]
    pub(crate) fn with_transport(name: &str, t: Box<dyn Transport>) -> Self {
        Self {
            t,
            name: name.to_string(),
            partial: String::new(),
            resyncing: false,
            response: String::new(),
            notifications: VecDeque::new(),
            done: false,
            dead: false,
            dead_reason: None,
        }
    }

    pub fn is_dead(&self) -> bool {
        self.dead
    }

    /// Why the channel died, if it did (raw OS error text). `None` while alive
    /// or after a clean shutdown path that never hit a hard I/O failure.
    pub fn death_reason(&self) -> Option<&str> {
        self.dead_reason.as_deref()
    }

    /// Sends a command and returns the accumulated response text.
    /// Completes on the final result code (OK/ERROR/+CXE ERROR) or on timeout.
    /// Unsolicited lines are routed to the notification queue, never mixed in.
    ///
    /// A failed write kills the channel instead of being ignored. Windows applies
    /// the port timeout to writes as well as reads, so a stalled bridge makes
    /// `WriteFile` come back having sent nothing; swallowing that meant the
    /// command never left the host and the caller still spent the full read
    /// timeout before blaming the modem for not answering.
    pub fn send(&mut self, cmd: &str, timeout_ms: u64) -> String {
        log::debug!(">> {}", cmd);
        if let Err(e) = self.t.write_all(format!("{}\r", cmd).as_bytes()) {
            self.dead = true;
            self.dead_reason = Some(e.to_string());
            log::warn!("{}: serial write failed: {}", self.name, e);
            return String::new();
        }
        // Only `response` is reset: stale reply text must not leak into this
        // command's answer. `partial` is deliberately kept — it holds the tail of
        // a line that has not seen its '\n' yet, and a modem can split
        // `+CMTI: "SM",7` across two USB reads with a command of ours (a queued
        // CMGR, the 10-minute SIM sweep) going out in between. Wiping it made the
        // remainder (`I: "SM",7`) stop looking like a notification prefix, so the
        // index was filed into `response` instead of the notification queue and
        // that SMS stayed invisible until the next reconnect re-read the SIM.
        self.response.clear();
        self.done = false;
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        while !self.done && !self.dead && Instant::now() < deadline {
            self.pump();
        }
        let text = std::mem::take(&mut self.response);
        log::debug!(
            "<< {}{}",
            preview(&text, 160),
            if self.done { "" } else { " (timeout)" }
        );
        text
    }

    /// Waits for the next unsolicited line (+CMTI/+CUSD/...).
    /// Blocks inside read(); returns None on timeout or dead port.
    pub fn wait_notification(&mut self, timeout_ms: u64) -> Option<String> {
        if self.dead {
            return None;
        }
        if let Some(n) = self.notifications.pop_front() {
            return Some(n);
        }
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        while !self.dead && Instant::now() < deadline {
            self.pump();
            if let Some(n) = self.notifications.pop_front() {
                return Some(n);
            }
        }
        None
    }

    pub fn take_notifications(&mut self) -> Vec<String> {
        self.notifications.drain(..).collect()
    }

    fn pump(&mut self) {
        let mut buf = [0u8; 1024];
        match self.t.read(&mut buf) {
            Ok(0) => {}
            Ok(n) => self.ingest(&buf[..n]),
            Err(e)
                if e.kind() == io::ErrorKind::TimedOut || e.kind() == io::ErrorKind::WouldBlock => {
            }
            Err(e) => {
                self.dead = true;
                self.dead_reason = Some(e.to_string());
                log::warn!("{}: serial read failed: {}", self.name, e);
            }
        }
    }

    fn ingest(&mut self, chunk: &[u8]) {
        self.partial.push_str(&String::from_utf8_lossy(chunk));

        // Recovering from an overflow: throw bytes away until a line boundary
        // arrives, then resume normally.
        if self.resyncing {
            match self.partial.find('\n') {
                Some(pos) => {
                    self.partial.drain(..=pos);
                    self.resyncing = false;
                }
                None => {
                    // No boundary yet. Silent — the one warning was already
                    // logged, and a modem spewing megabytes must not spew log
                    // lines with it.
                    self.partial.clear();
                    return;
                }
            }
        }

        while let Some(pos) = self.partial.find('\n') {
            let line: String = self.partial.drain(..=pos).collect();
            let line = line.trim_end_matches(['\r', '\n']).trim().to_string();
            if line.is_empty() {
                continue;
            }
            self.handle_line(line);
        }

        if self.partial.len() > MAX_PARTIAL_BYTES {
            // Drop the *whole* buffer rather than keeping either end of it.
            //
            // The overflowing line is garbage by definition, but a valid line may
            // well follow it, and keeping the newest N bytes would splice the tail
            // of the garbage onto that line's head: `<junk>+CMTI: "SM",7` no
            // longer starts with a notification prefix, so it would be filed as a
            // command response and the SMS lost — the same class of bug as wiping
            // `partial` in `send`. Keeping the oldest bytes is worse still, since
            // they are the garbage itself.
            //
            // So: discard everything and stay out of the parser until the next
            // '\n'. That costs at most one real line (the one straddling the
            // overflow) and guarantees no half-line is ever handed to
            // `handle_line`. A modem's reply is preceded by CRLF, so in practice
            // the resync ends before the reply's first character.
            log::warn!(
                "{}: no line terminator in {} bytes — discarding buffered garbage \
                 and resyncing to the next newline",
                self.name,
                self.partial.len()
            );
            self.partial.clear();
            self.resyncing = true;
        }
    }

    fn handle_line(&mut self, line: String) {
        if is_notification(&line) {
            log::debug!("++ {}", preview(&line, 120));
            self.notifications.push_back(line);
            return;
        }
        self.response.push_str(&line);
        self.response.push_str("\r\n");
        if is_final(&line) {
            self.done = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_matches_sms_net_rules() {
        assert!(is_final("OK"));
        assert!(is_final("ERROR"));
        assert!(is_final("+CME ERROR: 3"));
        assert!(is_final("+CMS ERROR: 500"));
        assert!(!is_final("+CMGL: 1"));
        assert!(is_notification("+CMTI: \"SM\",5"));
        assert!(is_notification("+CUSD: 2,\"abc\",15"));
        assert!(is_notification("RING"));
        assert!(is_notification("+CDS: 1"));
        assert!(!is_notification("+CMGR: 1"));
    }

    #[test]
    fn send_writes_command_and_returns_response() {
        let fake = FakeTransport::new("\r\nOK\r\n", 1024);
        let mut ch = AtChannel::with_transport("COM1", Box::new(fake));
        let resp = ch.send("ATE0", 100);
        assert_eq!(resp, "OK\r\n");
        assert!(!ch.is_dead());
        assert_eq!(ch.take_notifications(), Vec::<String>::new());
    }

    #[test]
    fn send_completes_on_error_final() {
        let mut ch = AtChannel::with_transport(
            "COM1",
            Box::new(FakeTransport::new("\r\n+CME ERROR: 3\r\n", 1024)),
        );
        let resp = ch.send("AT+CMGL=\"ALL\"", 100);
        assert_eq!(resp, "+CME ERROR: 3\r\n");
    }

    #[test]
    fn notifications_never_mix_into_response() {
        let script = "+CMTI: \"SM\",7\r\n+CMGL: 1,2,,12\r\n0891...\r\nOK\r\n";
        let mut ch = AtChannel::with_transport("COM1", Box::new(FakeTransport::new(script, 1024)));
        let resp = ch.send("AT+CMGL=\"ALL\"", 100);
        assert_eq!(resp, "+CMGL: 1,2,,12\r\n0891...\r\nOK\r\n");
        assert_eq!(
            ch.wait_notification(10),
            Some("+CMTI: \"SM\",7".to_string())
        );
    }

    #[test]
    fn chunked_single_byte_reads_keep_lines_intact() {
        let script = "AT+CMGR=3\r\r\n+CMGR: 1,,25\r\n001122335599884422556677889944556677889944556677\r\nOK\r\n";
        let mut ch = AtChannel::with_transport("ttyUSB2", Box::new(FakeTransport::new(script, 1)));
        let resp = ch.send("AT+CMGR=3", 500);
        assert!(resp.contains("+CMGR: 1,,25"));
        assert!(resp.ends_with("OK\r\n"));
    }

    #[test]
    fn timeout_returns_partial_without_done_line() {
        let mut ch = AtChannel::with_transport(
            "COM9",
            Box::new(FakeTransport::new("+CMGL: 1,,4\r\n1234", 1024)),
        );
        let resp = ch.send("AT", 20);
        assert_eq!(resp, "+CMGL: 1,,4\r\n");

        let mut ch2 =
            AtChannel::with_transport("COM9", Box::new(FakeTransport::new("\r\nRING\r\n", 1024)));
        let resp2 = ch2.send("AT", 20);
        assert_eq!(resp2, "");
        assert_eq!(ch2.wait_notification(0), Some("RING".to_string()));
    }

    #[test]
    fn hard_read_error_marks_channel_dead_and_stops_spinning() {
        let mut ch = AtChannel::with_transport("COM1", Box::new(FakeTransport::failing()));
        let resp = ch.send("AT", 50);
        assert_eq!(resp, "");
        assert!(ch.is_dead());
        assert_eq!(ch.wait_notification(50), None);
    }

    #[test]
    fn failed_write_marks_channel_dead_without_waiting_for_a_reply() {
        // The script would answer, so returning "" proves we never fell through to
        // the read loop: the command did not leave the host, and reporting that as
        // a silent modem is what made a stalled Windows bridge look like an empty
        // SIM slot.
        let mut ch = AtChannel::with_transport(
            "COM1",
            Box::new(FakeTransport::write_failing("\r\nOK\r\n")),
        );
        let start = Instant::now();
        let resp = ch.send("AT", 5000);
        assert_eq!(resp, "");
        assert!(ch.is_dead());
        assert!(ch.death_reason().is_some());
        assert!(
            start.elapsed() < Duration::from_millis(500),
            "send waited on a command it never wrote: {:?}",
            start.elapsed()
        );
    }

    #[test]
    fn pre_queued_notification_returned_immediately() {
        let script = "+CMTI: \"SM\",9\r\nOK\r\n";
        let mut ch = AtChannel::with_transport("COM1", Box::new(FakeTransport::new(script, 1024)));
        let _ = ch.send("AT", 100);
        assert_eq!(ch.wait_notification(0), Some("+CMTI: \"SM\",9".to_string()));
    }

    #[test]
    fn empty_lines_are_skipped() {
        let mut ch = AtChannel::with_transport(
            "COM1",
            Box::new(FakeTransport::new("\r\n\r\n\r\nOK\r\n", 1024)),
        );
        let resp = ch.send("AT", 100);
        assert_eq!(resp, "OK\r\n");
    }

    /// Hands out the first command's reply plus the *head* of an unsolicited
    /// line, then releases the tail only after the next command is written —
    /// the split-read race a real modem produces when a `+CMTI` lands while a
    /// CMGR (or the SIM sweep) is going out.
    struct SplitNotificationTransport {
        writes: usize,
        head_sent: bool,
        tail_sent: bool,
    }

    impl Transport for SplitNotificationTransport {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let data: &[u8] = if self.writes == 1 && !self.head_sent {
                self.head_sent = true;
                b"\r\n+CMGR: 1,,10\r\nOK\r\n+CMT"
            } else if self.writes >= 2 && !self.tail_sent {
                self.tail_sent = true;
                b"I: \"SM\",7\r\nOK\r\n"
            } else {
                return Err(io::ErrorKind::TimedOut.into());
            };
            let n = data.len().min(buf.len());
            buf[..n].copy_from_slice(&data[..n]);
            Ok(n)
        }

        fn write_all(&mut self, _data: &[u8]) -> io::Result<()> {
            self.writes += 1;
            Ok(())
        }
    }

    #[test]
    fn half_received_notification_survives_the_next_command() {
        let mut ch = AtChannel::with_transport(
            "ttyUSB2",
            Box::new(SplitNotificationTransport {
                writes: 0,
                head_sent: false,
                tail_sent: false,
            }),
        );
        let first = ch.send("AT+CMGR=1", 200);
        assert!(first.contains("+CMGR: 1,,10"), "first reply: {first:?}");

        // Second command: its reply must be only `OK`. If `send` had wiped the
        // carried-over `+CMT`, the tail would land here as a response line and
        // the notification would be lost.
        let second = ch.send("AT+CMGL=\"ALL\"", 200);
        assert!(
            !second.contains("I: \"SM\""),
            "notification tail leaked into the response: {second:?}"
        );
        assert_eq!(second, "OK\r\n");
        assert_eq!(
            ch.wait_notification(0),
            Some("+CMTI: \"SM\",7".to_string())
        );
    }

    /// Streams newline-free bytes for as long as only one command has been
    /// written, then answers the next command normally. Models a USB glitch that
    /// leaves binary garbage on the line: reads keep succeeding and never contain
    /// a terminator.
    struct NewlinelessGarbageTransport {
        left: usize,
        writes: usize,
        replied: bool,
    }

    impl Transport for NewlinelessGarbageTransport {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.writes < 2 && self.left > 0 {
                let n = buf.len().min(self.left).min(1024);
                buf[..n].fill(b'x');
                self.left -= n;
                return Ok(n);
            }
            if self.writes >= 2 && !self.replied {
                self.replied = true;
                let data = b"\r\nOK\r\n";
                buf[..data.len()].copy_from_slice(data);
                return Ok(data.len());
            }
            Err(io::ErrorKind::TimedOut.into())
        }

        fn write_all(&mut self, _data: &[u8]) -> io::Result<()> {
            self.writes += 1;
            Ok(())
        }
    }

    #[test]
    fn newlineless_garbage_cannot_grow_the_buffer_without_bound() {
        let garbage = MAX_PARTIAL_BYTES * 8;
        let mut ch = AtChannel::with_transport(
            "ttyUSB4",
            Box::new(NewlinelessGarbageTransport {
                left: garbage,
                writes: 0,
                replied: false,
            }),
        );
        let resp = ch.send("AT", 100);
        assert_eq!(resp, "", "garbage must not be parsed as a response");
        assert!(!ch.is_dead(), "garbage must not kill the channel");
        assert!(
            ch.partial.len() <= MAX_PARTIAL_BYTES,
            "buffered {} bytes after {} bytes of garbage",
            ch.partial.len(),
            garbage
        );

        // Still usable, and the leftover garbage did not splice itself onto the
        // front of the next reply — that splice is what would turn a following
        // `+CMTI:` line into an unrecognised response line.
        let resp2 = ch.send("AT", 200);
        assert_eq!(resp2, "OK\r\n");
    }

    #[test]
    fn maximum_length_legitimate_lines_are_never_truncated() {
        // Longest PDU `AT+CMGR` can return on one line: SMSC address (≤12 octets)
        // plus a full TPDU (163 octets), hex-encoded → 350 chars.
        let pdu = "0F".repeat(175);
        assert_eq!(pdu.len(), 350);
        assert!(pdu.len() < MAX_PARTIAL_BYTES);
        let script = format!("\r\n+CMGR: 1,,175\r\n{pdu}\r\nOK\r\n");
        let mut ch = AtChannel::with_transport("ttyUSB2", Box::new(FakeTransport::new(&script, 7)));
        let resp = ch.send("AT+CMGR=1", 2000);
        assert!(resp.contains(&pdu), "PDU line was truncated: {resp:?}");
        assert!(resp.ends_with("OK\r\n"));

        // The actual worst case on the read path: a 160-character GSM-7 body
        // re-encoded as UCS2 hex under `AT+CSCS="UCS2"` is 640 chars on one line.
        let ucs2_body = "0041".repeat(160);
        assert_eq!(ucs2_body.len(), 640);
        let script = format!(
            "\r\n+CMGL: 1,\"REC READ\",\"00390036\",,\"26/08/29,11:00:00+00\"\r\n{ucs2_body}\r\nOK\r\n"
        );
        let mut ch =
            AtChannel::with_transport("ttyUSB2", Box::new(FakeTransport::new(&script, 13)));
        let resp = ch.send("AT+CMGL=\"ALL\"", 2000);
        assert!(
            resp.contains(&ucs2_body),
            "UCS2 body line was truncated: {} chars",
            resp.len()
        );
        assert!(resp.ends_with("OK\r\n"));
    }
}
