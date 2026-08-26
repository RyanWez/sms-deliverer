#[cfg(debug_assertions)]
mod imp {
    use std::io::Write;

    use chrono::Local;
    use log::Level;

    pub struct TerminalLogger;

    impl TerminalLogger {
        pub fn format_line(
            now: &str,
            level: Level,
            target: &str,
            args: &std::fmt::Arguments,
        ) -> String {
            let module = target.split("::").next().unwrap_or(target);
            format!("[{now} {level:<5} {module}] {args}\n")
        }
    }

    impl log::Log for TerminalLogger {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            metadata.level() <= Level::Debug
        }

        fn log(&self, record: &log::Record) {
            if !self.enabled(record.metadata()) {
                return;
            }
            let now = Local::now().format("%H:%M:%S%.3f").to_string();
            let line = Self::format_line(&now, record.level(), record.target(), record.args());
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
            let line = TerminalLogger::format_line(
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
            let line = TerminalLogger::format_line(
                "00:00:00.000",
                Level::Warn,
                "core::modem",
                &format_args!("x"),
            );
            assert_eq!(line, "[00:00:00.000 WARN  core] x\n");
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
#[inline]
pub fn init() {}

#[cfg(test)]
mod init_tests {
    #[test]
    fn init_installs_logger_and_sets_debug_level() {
        super::init();
        assert_eq!(log::max_level(), log::LevelFilter::Debug);
    }
}
