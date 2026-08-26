use directories::ProjectDirs;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Settings {
    pub update_url: String,
    pub sound_on: bool,
    pub auto_copy_otp: bool,
    pub auto_expire_hours: i32,
    pub sim_numbers: HashMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            update_url: String::new(),
            sound_on: true,
            auto_copy_otp: true,
            auto_expire_hours: 48,
            sim_numbers: HashMap::new(),
        }
    }
}

impl Settings {
    pub fn settings_path() -> PathBuf {
        ProjectDirs::from("", "", "sms-tauri")
            .map(|d| d.config_dir().join("settings.txt"))
            .unwrap_or_else(|| PathBuf::from("settings.txt"))
    }

    pub fn sim_cache_path() -> PathBuf {
        ProjectDirs::from("", "", "sms-tauri")
            .map(|d| d.data_dir().join("sim_numbers.csv"))
            .unwrap_or_else(|| PathBuf::from("sim_numbers.csv"))
    }

    pub fn load() -> Self {
        let mut s = Self::default();
        if let Ok(content) = fs::read_to_string(Self::settings_path()) {
            for line in content.lines() {
                if let Some(eq) = line.find('=') {
                    let k = line[..eq].trim();
                    let v = line[eq + 1..].trim();
                    match k {
                        "UpdateUrl" => s.update_url = v.to_string(),
                        "SoundOn" => s.sound_on = v == "1" || v.eq_ignore_ascii_case("true"),
                        "AutoCopyOtp" => {
                            s.auto_copy_otp = v == "1" || v.eq_ignore_ascii_case("true")
                        }
                        "AutoExpireHours" => s.auto_expire_hours = v.parse().unwrap_or(48),
                        _ => {}
                    }
                }
            }
        }
        s.load_sim_numbers();
        s
    }

    pub fn save(&self) {
        let path = Self::settings_path();
        let _ = fs::create_dir_all(path.parent().unwrap());
        let content = format!(
            "UpdateUrl={}\nSoundOn={}\nAutoCopyOtp={}\nAutoExpireHours={}\n",
            self.update_url,
            if self.sound_on { "1" } else { "0" },
            if self.auto_copy_otp { "1" } else { "0" },
            self.auto_expire_hours,
        );
        let _ = fs::write(path, content);
    }

    fn load_sim_numbers(&mut self) {
        let path = Self::sim_cache_path();
        if let Ok(content) = fs::read_to_string(&path) {
            for line in content.lines() {
                let parts: Vec<&str> = line.splitn(2, ',').collect();
                if parts.len() >= 2
                    && (parts[0].starts_with("COM")
                        || parts[0].contains("ttyUSB")
                        || parts[0].contains("ttyACM"))
                {
                    let num = crate::core::decoder::normalize_number(parts[1].trim());
                    if !num.is_empty() {
                        self.sim_numbers.insert(parts[0].trim().to_string(), num);
                    }
                }
            }
        }
    }

    pub fn save_sim_numbers(&self) {
        let path = Self::sim_cache_path();
        let _ = fs::create_dir_all(path.parent().unwrap());
        let mut lines: Vec<(&String, &String)> = self.sim_numbers.iter().collect();
        lines.sort_by_key(|(k, _)| crate::core::modem::port_num(k));
        let content: String = lines
            .iter()
            .map(|(k, v)| format!("{},{}\n", k, v))
            .collect();
        let _ = fs::write(path, content);
    }

    pub fn sim_of(&self, port: &str) -> &str {
        self.sim_numbers.get(port).map(|s| s.as_str()).unwrap_or("")
    }
}
