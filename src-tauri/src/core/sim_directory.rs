//! Persistent map of serial port → SIM phone number.
//!
//! This is the only state the Rust side owns across runs. User preferences
//! (retention period, notifications, appearance, …) live in exactly one place:
//! the frontend settings store. The backend used to keep a second, parallel
//! `settings.txt` with its own `AutoExpireHours`/`SoundOn`/`AutoCopyOtp` keys
//! that nothing ever read, so a value changed in the UI and the value the
//! backend believed could never agree. Those keys are gone; whatever the
//! backend needs is passed in per call.

use directories::ProjectDirs;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct SimDirectory {
    /// Keyed by stable port path where known, legacy port name otherwise.
    pub numbers: HashMap<String, String>,
}

impl SimDirectory {
    pub fn cache_path() -> PathBuf {
        ProjectDirs::from("", "", "sms-tauri")
            .map(|d| d.data_dir().join("sim_numbers.csv"))
            .unwrap_or_else(|| PathBuf::from("sim_numbers.csv"))
    }

    pub fn load() -> Self {
        let mut d = Self::default();
        if let Ok(content) = fs::read_to_string(Self::cache_path()) {
            for line in content.lines() {
                let parts: Vec<&str> = line.splitn(2, ',').collect();
                if parts.len() < 2 {
                    continue;
                }
                let num = crate::core::decoder::normalize_number(parts[1].trim());
                if !num.is_empty() {
                    d.numbers.insert(parts[0].trim().to_string(), num);
                }
            }
        }
        d
    }

    pub fn save(&self) {
        let path = Self::cache_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut lines: Vec<(&String, &String)> = self.numbers.iter().collect();
        lines.sort_by_key(|(k, _)| crate::core::modem::port_num(k));
        let content: String = lines
            .iter()
            .map(|(k, v)| format!("{},{}\n", k, v))
            .collect();
        let _ = fs::write(path, content);
    }

    /// SIM number lookup keyed by stable path, with a legacy fallback to the
    /// old mutable-name key so existing `sim_numbers.csv` files keep working.
    pub fn number_of(&self, stable: &str, legacy: &str) -> String {
        if let Some(v) = self.numbers.get(stable) {
            return v.clone();
        }
        self.numbers.get(legacy).cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir_with(map: HashMap<String, String>) -> SimDirectory {
        SimDirectory { numbers: map }
    }

    #[test]
    fn stable_key_preferred_over_legacy() {
        let d = dir_with(HashMap::from([
            ("pci-usb-0:4.1:1.0-port0".into(), "+951".into()),
            ("ttyUSB0".into(), "+999".into()),
        ]));
        assert_eq!(d.number_of("pci-usb-0:4.1:1.0-port0", "ttyUSB0"), "+951");
    }

    #[test]
    fn falls_back_to_legacy_name_key() {
        let d = dir_with(HashMap::from([("ttyUSB0".into(), "+777".into())]));
        assert_eq!(d.number_of("pci-usb-0:4.1:1.0-port0", "ttyUSB0"), "+777");
    }

    #[test]
    fn empty_when_no_match() {
        let d = dir_with(HashMap::new());
        assert_eq!(d.number_of("path-a", "ttyUSB3"), "");
    }
}
