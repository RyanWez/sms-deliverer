/// Shared line formatter used by every sink (debug terminal, release file).
/// Keeps the exact `[HH:MM:SS.mmm LEVEL module] message` layout consistent
/// across build profiles so dev/prod logs compare 1:1.
pub(crate) fn format_line(
    now: &str,
    level: Level,
    target: &str,
    args: &std::fmt::Arguments,
) -> String {
    let module = target.split("::").next().unwrap_or(target);
    format!("[{now} {level:<5} {module}] {args}\n")
}

// `Level` is only referenced by the shared formatter here; each sink module
// re-imports what it needs privately.
use log::Level;

#[cfg(debug_assertions)]
mod imp {
    use std::io::Write;

    use chrono::Local;
    use log::Level;

    use super::format_line;

    pub struct TerminalLogger;

    impl log::Log for TerminalLogger {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            metadata.level() <= Level::Debug
        }

        fn log(&self, record: &log::Record) {
            if !self.enabled(record.metadata()) {
                return;
            }
            let now = Local::now().format("%H:%M:%S%.3f").to_string();
            let line = format_line(&now, record.level(), record.target(), record.args());
            let _ = std::io::stderr().write_all(line.as_bytes());
        }

        fn flush(&self) {
            let _ = std::io::stderr().flush();
        }
    }

    pub static TERMINAL_LOGGER: TerminalLogger = TerminalLogger;

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn format_line_includes_time_level_module_and_message() {
            let line = super::super::format_line(
                "12:33:45.123",
                Level::Info,
                "sms_tauri_lib::commands::start_scan",
                &format_args!("Scan started: {} port(s)", 4),
            );
            assert_eq!(
                line,
                "[12:33:45.123 INFO  sms_tauri_lib] Scan started: 4 port(s)\n"
            );
        }

        #[test]
        fn format_line_pads_short_levels() {
            let line = super::super::format_line(
                "00:00:00.000",
                Level::Warn,
                "core::modem",
                &format_args!("x"),
            );
            assert_eq!(line, "[00:00:00.000 WARN  core] x\n");
        }
    }
}

#[cfg(not(debug_assertions))]
mod imp {
    //! Production file logger: appends to `<data_dir>/app.log`, capped at
    //! [`MAX_BYTES`]; overflow rotates to `app.log.old` (total ≤ ~2 MB).
    use std::fs::{self, File, OpenOptions};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    use chrono::Local;
    use directories::ProjectDirs;
    use log::Level;

    use super::format_line;

    pub const MAX_BYTES: u64 = 1024 * 1024; // 1 MB

    pub struct FileLogger {
        path: PathBuf,
        cap: u64,
        file: Mutex<Option<File>>,
    }

    impl FileLogger {
        /// Writes to `<data_dir>/app.log`; rotates on start when oversized.
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
                file: Mutex::new(None), // lazy-opened on first write
            }
        }

        fn write_line(&self, line: &str) {
            let mut guard = self.file.lock().unwrap();
            if guard.is_none() {
                // `.ok()` → unwritable disk drops the line instead of crashing.
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
                *guard = None; // close before rename — required on Windows
                swap_to_old(&self.path); // next write reopens a fresh log
            }
        }
    }

    /// Move `path` over its sibling `*.log.old`, replacing any previous backup.
    fn swap_to_old(path: &Path) {
        let old = path.with_extension("log.old");
        let _ = fs::remove_file(&old); // rename fails on Windows if target exists
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
            let now = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
            let line = format_line(&now, record.level(), record.target(), record.args());
            self.write_line(&line);
        }

        fn flush(&self) {
            let _ = self.file.lock().unwrap().as_mut().map(|f| f.flush());
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use log::Log; // trait method `enabled` needed below

        fn scratch(label: &str) -> PathBuf {
            let dir =
                std::env::temp_dir().join(format!("sms-logtest-{}-{label}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            dir.join("app.log")
        }

        #[test]
        fn rotates_mid_run_when_cap_crossed() {
            let path = scratch("midrun");
            let logger = FileLogger::with_cap(path.clone(), 48);
            logger.write_line(&format!("[x] {}\n", "A".repeat(20)));
            logger.write_line(&format!("[y] {}\n", "B".repeat(20))); // crosses cap
            let archived = fs::read_to_string(path.with_extension("log.old")).unwrap();
            // both 25-byte lines land in the archived file before rotation
            assert_eq!(archived.trim_end().len(), 49);
            logger.write_line("fresh\n"); // reopens a new app.log
            assert_eq!(fs::read_to_string(&path).unwrap(), "fresh\n");
            let _ = fs::remove_dir_all(path.parent().unwrap());
        }

        #[test]
        fn rotates_oversized_file_on_startup() {
            let path = scratch("startup");
            fs::write(&path, "X".repeat(64)).unwrap();
            let logger = FileLogger::with_cap(path.clone(), 32);
            assert!(fs::read(&path).unwrap_or_default().is_empty()); // fresh log
            assert_eq!(fs::read(path.with_extension("log.old")).unwrap().len(), 64);
            logger.write_line("ok\n"); // still writable after startup rotation
            assert_eq!(fs::read_to_string(&path).unwrap(), "ok\n");
            let _ = fs::remove_dir_all(path.parent().unwrap());
        }

        #[test]
        fn accepts_info_but_filters_debug() {
            let path = scratch("levels");
            let logger = FileLogger::with_cap(path.clone(), 10_000);
            assert!(!logger.enabled(&log::Metadata::builder().level(Level::Debug).build()));
            assert!(logger.enabled(&log::Metadata::builder().level(Level::Info).build()));
            let _ = fs::remove_dir_all(path.parent().unwrap());
        }
    }
}

#[cfg(debug_assertions)]
pub fn init() {
    if log::set_logger(&imp::TERMINAL_LOGGER).is_err() {
        return;
    }
    log::set_max_level(log::LevelFilter::Debug);
}

#[cfg(not(debug_assertions))]
pub fn init() {
    // Info level + capped rotation keeps production disk usage tiny while
    // preserving enough context to diagnose field failures.
    let Some(logger) = imp::FileLogger::open() else {
        return; // no data dir available — run without logs rather than crash
    };
    if log::set_logger(Box::leak(Box::new(logger))).is_err() {
        return;
    }
    log::set_max_level(log::LevelFilter::Info);
}

#[cfg(test)]
mod init_tests {
    #[test]
    #[cfg(debug_assertions)]
    fn init_installs_logger_and_sets_debug_level() {
        super::init();
        assert_eq!(log::max_level(), log::LevelFilter::Debug);
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn release_init_sets_info_level() {
        super::init();
        assert_eq!(log::max_level(), log::LevelFilter::Info);
    }
}
