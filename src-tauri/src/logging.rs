use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use chrono::Local;
use log::Level;
use serde::Serialize;

pub const MAX_RING_BUFFER: usize = 1000;

#[derive(Serialize, Clone, Debug)]
pub struct LogEntry {
    pub id: u64,
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

static ID_COUNTER: AtomicU64 = AtomicU64::new(1);
static LOG_BUFFER: OnceLock<Arc<Mutex<VecDeque<LogEntry>>>> = OnceLock::new();

pub fn get_log_buffer() -> &'static Arc<Mutex<VecDeque<LogEntry>>> {
    LOG_BUFFER.get_or_init(|| Arc::new(Mutex::new(VecDeque::with_capacity(MAX_RING_BUFFER))))
}

pub fn capture_entry(level: Level, target: &str, message: &str) -> Option<LogEntry> {
    if level > Level::Info {
        return None;
    }
    let id = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let now = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    let module = target.split("::").next().unwrap_or(target).to_string();
    let entry = LogEntry {
        id,
        timestamp: now,
        level: level.to_string(),
        target: module,
        message: message.to_string(),
    };

    let buf = get_log_buffer();
    if let Ok(mut lock) = buf.lock() {
        if lock.len() >= MAX_RING_BUFFER {
            lock.pop_front();
        }
        lock.push_back(entry.clone());
    }

    Some(entry)
}

pub fn get_all_logs(limit: Option<usize>, min_level: Option<String>) -> Vec<LogEntry> {
    let buf = get_log_buffer();
    let lock = buf.lock().unwrap();
    let min_lvl = min_level.as_deref().and_then(|l| match l.to_uppercase().as_str() {
        "ERROR" => Some(Level::Error),
        "WARN" => Some(Level::Warn),
        "INFO" => Some(Level::Info),
        _ => None,
    });

    let iter = lock.iter().filter(|e| {
        if let Some(target_lvl) = min_lvl {
            let entry_lvl = match e.level.as_str() {
                "ERROR" => Level::Error,
                "WARN" => Level::Warn,
                "INFO" => Level::Info,
                "DEBUG" => Level::Debug,
                "TRACE" => Level::Trace,
                _ => Level::Info,
            };
            entry_lvl <= target_lvl
        } else {
            true
        }
    });

    let entries: Vec<LogEntry> = if let Some(lim) = limit {
        iter.rev().take(lim).cloned().collect::<Vec<_>>().into_iter().rev().collect()
    } else {
        iter.cloned().collect()
    };

    entries
}

pub fn clear_log_buffer() {
    let buf = get_log_buffer();
    if let Ok(mut lock) = buf.lock() {
        lock.clear();
    }
}

pub fn get_log_file_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "sms-tauri")
        .map(|d| d.data_dir().join("app.log"))
}

/// Number of trailing characters a masked number keeps.
const MASK_KEEP: usize = 3;

/// Mask a subscriber number for logging: keep the last three characters, hide
/// the rest (`+959771234456` -> `***456`).
///
/// Nothing that identifies a subscriber may reach a sink. `app.log` only
/// rotates at 5 MB and is never pruned by age, and the ring buffer is exposed
/// verbatim on the Logs page, so a plaintext MSISDN written there outlives the
/// inbox retention window indefinitely. Three digits are enough to tell two
/// slots apart while debugging.
///
/// Iterates by `char` rather than slicing bytes: numbers arrive from USSD
/// replies and PDU sender fields, which can carry a `+`, spaces or mangled
/// multi-byte junk, and a byte slice landing mid-codepoint would panic.
pub fn mask_number(n: &str) -> String {
    let count = n.chars().count();
    if count <= MASK_KEEP {
        // Too short to reveal a tail from — mask it whole. Real MSISDNs never
        // land here; a truncated or garbage value can.
        return "*".repeat(count);
    }
    let tail: String = n.chars().skip(count - MASK_KEEP).collect();
    format!("{}{}", "*".repeat(MASK_KEEP), tail)
}

/// How to describe an OTP in a log line: whether one was found and how long it
/// is, never the value itself.
pub fn otp_summary(otp: Option<&str>) -> String {
    match otp {
        Some(code) => format!("found ({} digits)", code.chars().count()),
        None => "none".to_string(),
    }
}

/// Shared line formatter used by every sink (debug terminal, release file).
pub(crate) fn format_line(
    now: &str,
    level: Level,
    target: &str,
    args: &std::fmt::Arguments,
) -> String {
    let module = target.split("::").next().unwrap_or(target);
    format!("[{now} {level:<5} {module}] {args}\n")
}

#[cfg(debug_assertions)]
mod imp {
    use std::io::Write;
    use chrono::Local;
    use log::Level;
    use super::{capture_entry, format_line};

    pub struct TerminalLogger;

    impl log::Log for TerminalLogger {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            metadata.level() <= Level::Info
        }

        fn log(&self, record: &log::Record) {
            if !self.enabled(record.metadata()) {
                return;
            }
            let msg = format!("{}", record.args());
            capture_entry(record.level(), record.target(), &msg);

            let now = Local::now().format("%H:%M:%S%.3f").to_string();
            let line = format_line(&now, record.level(), record.target(), record.args());
            let _ = std::io::stderr().write_all(line.as_bytes());
        }

        fn flush(&self) {
            let _ = std::io::stderr().flush();
        }
    }

    pub static TERMINAL_LOGGER: TerminalLogger = TerminalLogger;
}

#[cfg(not(debug_assertions))]
mod imp {
    use std::fs::{self, File, OpenOptions};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use chrono::Local;
    use directories::ProjectDirs;
    use log::Level;
    use super::{capture_entry, format_line};

    pub const MAX_BYTES: u64 = 5 * 1024 * 1024; // 5 MB

    pub struct FileLogger {
        path: PathBuf,
        cap: u64,
        file: Mutex<Option<File>>,
    }

    impl FileLogger {
        pub fn open() -> Option<Self> {
            let path = ProjectDirs::from("", "", "sms-tauri")
                .map(|d| d.data_dir().to_path_buf())?
                .join("app.log");
            Some(Self::with_cap(path, MAX_BYTES))
        }

        pub fn with_cap(path: PathBuf, cap: u64) -> Self {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if fs::metadata(&path).map(|m| m.len()).unwrap_or(0) > cap {
                swap_to_old(&path);
            }
            Self {
                path,
                cap,
                file: Mutex::new(None),
            }
        }

        fn write_line(&self, line: &str) {
            let mut guard = self.file.lock().unwrap();
            if guard.is_none() {
                *guard = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.path)
                    .ok();
            }
            if let Some(f) = guard.as_mut() {
                let _ = f.write_all(line.as_bytes());
                let _ = f.flush();
            }
            if fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0) > self.cap {
                *guard = None;
                swap_to_old(&self.path);
            }
        }
    }

    fn swap_to_old(path: &Path) {
        let old = path.with_extension("log.old");
        let _ = fs::remove_file(&old);
        let _ = fs::rename(path, &old);
    }

    impl log::Log for FileLogger {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            metadata.level() <= Level::Info
        }

        fn log(&self, record: &log::Record) {
            if !self.enabled(record.metadata()) {
                return;
            }
            let msg = format!("{}", record.args());
            capture_entry(record.level(), record.target(), &msg);

            let now = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
            let line = format_line(&now, record.level(), record.target(), record.args());
            self.write_line(&line);
        }

        fn flush(&self) {
            let _ = self.file.lock().unwrap().as_mut().map(|f| f.flush());
        }
    }
}

pub fn init() {
    #[cfg(debug_assertions)]
    {
        if log::set_logger(&imp::TERMINAL_LOGGER).is_err() {
            return;
        }
    }

    #[cfg(not(debug_assertions))]
    {
        if let Some(logger) = imp::FileLogger::open() {
            let _ = log::set_logger(Box::leak(Box::new(logger)));
        }
    }

    log::set_max_level(log::LevelFilter::Info);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masking_keeps_only_the_last_three_digits() {
        assert_eq!(mask_number("+959771234456"), "***456");
        assert_eq!(mask_number("09771234456"), "***456");
        assert_eq!(mask_number("1234"), "***234");
    }

    #[test]
    fn masking_never_panics_on_short_or_empty_input() {
        // A number field can be empty or truncated (bad USSD reply, odd PDU
        // sender), and a byte-slice implementation would panic here.
        assert_eq!(mask_number(""), "");
        assert_eq!(mask_number("1"), "*");
        assert_eq!(mask_number("12"), "**");
        assert_eq!(mask_number("123"), "***");
    }

    #[test]
    fn masking_handles_multi_byte_characters() {
        // Sender fields are alphanumeric on some networks; a mangled decode can
        // leave multi-byte characters in there.
        assert_eq!(mask_number("МТС−телеком"), "***ком");
        assert_eq!(mask_number("日本"), "**");
    }

    #[test]
    fn otp_summary_reports_presence_and_length_only() {
        assert_eq!(otp_summary(Some("123456")), "found (6 digits)");
        assert_eq!(otp_summary(None), "none");
        // The value itself must never appear in the summary.
        assert!(!otp_summary(Some("987654")).contains("987654"));
    }
}
