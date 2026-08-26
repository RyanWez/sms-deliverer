use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

use super::modem;

const NOTIFICATION_PREFIXES: [&str; 7] = [
    "+CMTI:", "+CUSD:", "+CRING:", "RING", "+CMT:", "+CBM:", "+CDS:",
];

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
    response: String,
    notifications: VecDeque<String>,
    done: bool,
    dead: bool,
}

impl AtChannel {
    pub fn open(name: &str) -> Result<Self, String> {
        let port = modem::open_port(name)?;
        Ok(Self {
            t: Box::new(SerialTransport(port)),
            name: name.to_string(),
            partial: String::new(),
            response: String::new(),
            notifications: VecDeque::new(),
            done: false,
            dead: false,
        })
    }

    #[cfg(test)]
    pub(crate) fn with_transport(name: &str, t: Box<dyn Transport>) -> Self {
        Self {
            t,
            name: name.to_string(),
            partial: String::new(),
            response: String::new(),
            notifications: VecDeque::new(),
            done: false,
            dead: false,
        }
    }

    pub fn is_dead(&self) -> bool {
        self.dead
    }

    /// Sends a command and returns the accumulated response text.
    /// Completes on the final result code (OK/ERROR/+CXE ERROR) or on timeout.
    /// Unsolicited lines are routed to the notification queue, never mixed in.
    pub fn send(&mut self, cmd: &str, timeout_ms: u64) -> String {
        log::debug!(">> {}", cmd);
        let _ = self.t.write_all(format!("{}\r", cmd).as_bytes());
        self.response.clear();
        self.partial.clear();
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
            Err(_) => {
                self.dead = true;
                log::warn!("{}: serial read failed", self.name);
            }
        }
    }

    fn ingest(&mut self, chunk: &[u8]) {
        self.partial.push_str(&String::from_utf8_lossy(chunk));
        while let Some(pos) = self.partial.find('\n') {
            let line: String = self.partial.drain(..=pos).collect();
            let line = line.trim_end_matches(['\r', '\n']).trim().to_string();
            if line.is_empty() {
                continue;
            }
            self.handle_line(line);
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

    struct FakeTransport {
        script: Vec<u8>,
        pos: usize,
        bytes_per_read: usize,
        fail_reads: bool,
        written: Vec<u8>,
    }

    impl FakeTransport {
        fn new(script: &str, bytes_per_read: usize) -> Self {
            Self {
                script: script.as_bytes().to_vec(),
                pos: 0,
                bytes_per_read,
                fail_reads: false,
                written: Vec::new(),
            }
        }

        fn failing() -> Self {
            Self {
                script: vec![],
                pos: 0,
                bytes_per_read: 1024,
                fail_reads: true,
                written: Vec::new(),
            }
        }
    }

    impl Transport for FakeTransport {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.fail_reads {
                return Err(io::Error::other("device lost"));
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

        fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
            self.written.extend_from_slice(data);
            Ok(())
        }
    }

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
        let mut fake = FakeTransport::new("\r\nOK\r\n", 1024);
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
}
