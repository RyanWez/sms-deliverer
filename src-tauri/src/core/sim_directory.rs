//! Persistent map of SIM → phone number, plus which slot each SIM was last in.
//!
//! This is the only state the Rust side owns across runs. User preferences
//! (retention period, notifications, appearance, …) live in exactly one place:
//! the frontend settings store.
//!
//! # Why the key is the ICCID
//!
//! Earlier versions filed phone numbers against the serial port. That is wrong
//! twice over. `/dev/ttyUSB*` numbering is assigned in enumeration order, so a
//! reboot or a hotplug reshuffles it, and the lookup also fell back to the port
//! *name*, so a number learned when a SIM was `ttyUSB43` reappeared on whatever
//! unrelated stick was called `ttyUSB43` next time. On a 64-slot bank that showed
//! up as one number on two ports, and as numbers on ports with no modem at all —
//! the count of ports "with SIM" exceeded the count of modems.
//!
//! The ICCID is printed on the card and returned by `AT+CCID`; it survives
//! renumbering and follows the SIM if it is moved to another slot. So:
//!
//! - `numbers: ICCID → phone number` is the durable record. Nothing deletes from
//!   it, so a card that comes back is recognised immediately.
//! - `slots: stable port path → ICCID` is a hint, so numbers can be shown before
//!   anything has been probed in this session. Every probe corrects it, and a
//!   slot with no modem drops out of it.

use directories::ProjectDirs;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Marks the ICCID-keyed format. Files without it are port-keyed and are
/// discarded on load rather than migrated: their keys are mutable tty names
/// whose meaning has already been lost.
const HEADER: &str = "# sms-tauri sim directory v2";

#[derive(Debug, Clone, Default)]
pub struct SimDirectory {
    /// ICCID → phone number. The durable record.
    pub numbers: HashMap<String, String>,
    /// Stable port path → ICCID last seen in that slot.
    pub slots: HashMap<String, String>,
}

impl SimDirectory {
    pub fn cache_path() -> PathBuf {
        ProjectDirs::from("", "", "sms-tauri")
            .map(|d| d.data_dir().join("sim_numbers.csv"))
            .unwrap_or_else(|| PathBuf::from("sim_numbers.csv"))
    }

    pub fn load() -> Self {
        let path = Self::cache_path();
        match fs::read_to_string(&path) {
            Ok(content) => {
                let d = Self::parse(&content);
                // A port-keyed file cannot be migrated — its keys are tty names
                // that have already been reassigned — but it is the operator's
                // data, so move it aside rather than overwriting it on the next
                // save. The numbers in it are still readable by hand.
                if d.numbers.is_empty() && d.slots.is_empty() && !content.trim().is_empty() {
                    let backup = path.with_extension("csv.v1-port-keyed");
                    match fs::rename(&path, &backup) {
                        Ok(()) => log::warn!("Previous SIM directory kept at {}", backup.display()),
                        Err(e) => log::warn!("Could not set the old SIM directory aside: {}", e),
                    }
                }
                d
            }
            Err(_) => Self::default(),
        }
    }

    pub fn parse(content: &str) -> Self {
        let mut d = Self::default();
        if !content.lines().any(|l| l.trim() == HEADER) {
            if !content.trim().is_empty() {
                log::warn!(
                    "SIM directory was keyed by serial port — discarded. Port names are \
                     reassigned on every hotplug, so those entries could not be trusted. \
                     Run Detect Modems, then Get SIM Numbers, to rebuild it."
                );
            }
            return d;
        }
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.splitn(3, ',');
            let (Some(kind), Some(key), Some(value)) = (parts.next(), parts.next(), parts.next())
            else {
                continue;
            };
            let key = key.trim();
            let value = value.trim();
            if key.is_empty() || value.is_empty() {
                continue;
            }
            match kind.trim() {
                "sim" => {
                    let num = crate::core::decoder::normalize_number(value);
                    if !num.is_empty() {
                        d.numbers.insert(key.to_string(), num);
                    }
                }
                "slot" => {
                    d.slots.insert(key.to_string(), value.to_string());
                }
                _ => {}
            }
        }
        d
    }

    pub fn serialize(&self) -> String {
        let mut out = String::from(HEADER);
        out.push('\n');
        let mut sims: Vec<(&String, &String)> = self.numbers.iter().collect();
        sims.sort();
        for (iccid, num) in sims {
            out.push_str(&format!("sim,{},{}\n", iccid, num));
        }
        let mut slots: Vec<(&String, &String)> = self.slots.iter().collect();
        slots.sort();
        for (path, iccid) in slots {
            out.push_str(&format!("slot,{},{}\n", path, iccid));
        }
        out
    }

    pub fn save(&self) {
        let path = Self::cache_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(path, self.serialize());
    }

    /// Record the phone number belonging to a card.
    pub fn set_number(&mut self, iccid: &str, number: &str) {
        if iccid.is_empty() || number.is_empty() {
            return;
        }
        self.numbers.insert(iccid.to_string(), number.to_string());
    }

    /// Note which card is in a slot. Replacing the card replaces the mapping, so
    /// the previous tenant's number stops being shown for that slot.
    pub fn set_slot(&mut self, stable_path: &str, iccid: &str) {
        if stable_path.is_empty() || iccid.is_empty() {
            return;
        }
        self.slots.insert(stable_path.to_string(), iccid.to_string());
    }

    /// Forget which card is in a slot — used when a probe finds no modem there.
    /// The card's number stays on file for whenever it turns up again.
    pub fn clear_slot(&mut self, stable_path: &str) {
        self.slots.remove(stable_path);
    }

    /// Number for a card, if one is known.
    pub fn number_for_iccid(&self, iccid: &str) -> String {
        self.numbers.get(iccid).cloned().unwrap_or_default()
    }

    /// Number to display for a slot: the card confirmed in it this session when
    /// there is one, otherwise the card remembered from last time.
    pub fn number_of(&self, stable_path: &str, iccid: Option<&str>) -> String {
        match iccid {
            Some(id) => self.number_for_iccid(id),
            None => match self.slots.get(stable_path) {
                Some(id) => self.number_for_iccid(id),
                None => String::new(),
            },
        }
    }

    /// ICCID remembered for a slot.
    pub fn iccid_of(&self, stable_path: &str) -> Option<String> {
        self.slots.get(stable_path).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ICCID_A: &str = "8995010912345678901";
    const ICCID_B: &str = "8995010999999999999";
    const SLOT_A: &str = "pci-0000:03:00.3-usb-0:4.1:1.0-port0";
    const SLOT_B: &str = "pci-0000:03:00.3-usb-0:4.2:1.0-port0";

    fn dir() -> SimDirectory {
        let mut d = SimDirectory::default();
        d.set_number(ICCID_A, "09651995803");
        d.set_slot(SLOT_A, ICCID_A);
        d
    }

    #[test]
    fn confirmed_card_wins_over_the_slot_hint() {
        let mut d = dir();
        d.set_number(ICCID_B, "09777000111");
        // The stick answered and turned out to hold a different card than last
        // session; the number shown must follow the card, not the slot.
        assert_eq!(d.number_of(SLOT_A, Some(ICCID_B)), "09777000111");
    }

    #[test]
    fn slot_hint_used_before_anything_is_probed() {
        assert_eq!(dir().number_of(SLOT_A, None), "09651995803");
    }

    #[test]
    fn unknown_slot_has_no_number() {
        assert_eq!(dir().number_of(SLOT_B, None), "");
    }

    #[test]
    fn confirmed_card_with_no_number_on_file_shows_nothing() {
        assert_eq!(dir().number_of(SLOT_A, Some(ICCID_B)), "");
    }

    #[test]
    fn clearing_a_slot_keeps_the_number_on_file() {
        let mut d = dir();
        d.clear_slot(SLOT_A);
        assert_eq!(d.number_of(SLOT_A, None), "");
        assert_eq!(d.number_for_iccid(ICCID_A), "09651995803");
    }

    #[test]
    fn moving_a_card_to_another_slot_carries_its_number() {
        let mut d = dir();
        d.clear_slot(SLOT_A);
        d.set_slot(SLOT_B, ICCID_A);
        assert_eq!(d.number_of(SLOT_B, None), "09651995803");
    }

    #[test]
    fn round_trips_through_the_file_format() {
        let parsed = SimDirectory::parse(&dir().serialize());
        assert_eq!(parsed.numbers, dir().numbers);
        assert_eq!(parsed.slots, dir().slots);
    }

    #[test]
    fn port_keyed_file_is_discarded_not_migrated() {
        // v1 format. Its keys are tty names that have since been reassigned,
        // which is what put one number on two ports.
        let old = "/dev/ttyUSB39,09651995803\n/dev/ttyUSB43,09651995803\n";
        let d = SimDirectory::parse(old);
        assert!(d.numbers.is_empty());
        assert!(d.slots.is_empty());
    }

    #[test]
    fn empty_file_is_not_a_discard() {
        assert!(SimDirectory::parse("").numbers.is_empty());
    }
}
