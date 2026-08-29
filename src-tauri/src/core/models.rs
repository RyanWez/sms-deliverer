//! Wire types shared with the frontend, plus the retention policy applied to
//! both the in-app inbox and SIM storage.
//!
//! View state (filters, toasts, pagination) lives entirely in the Svelte
//! stores; the structs here are only what actually crosses the IPC boundary.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsMessage {
    pub port: String,
    pub index: i32,
    pub from: String,
    pub received: DateTime<Utc>,
    pub status: String,
    pub text: String,
    /// SIM memory indices of every fragment this message was assembled from.
    /// Empty for single-part messages (only `index` applies).
    #[serde(default)]
    pub part_indices: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsItem {
    pub id: u64,
    pub message: SmsMessage,
    pub otp: Option<String>,
    pub is_new: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortInfo {
    /// Mutable `/dev/ttyUSB*` / `COM*` name — used to actually open the device.
    pub name: String,
    /// Stable identity key (Linux `/dev/serial/by-path` topological id, else the
    /// name itself). Survives ttyUSB renumbering so SIM assignments stay put.
    pub path: String,
    pub checked: bool,
    pub sim_number: String,
    /// ICCID of the card in this slot, once a probe has read it. Phone numbers
    /// are filed against this rather than against the port, because tty
    /// numbering is reassigned on every hotplug.
    #[serde(default)]
    pub iccid: Option<String>,
    /// Result of the last liveness probe: `Some(true)` a modem answered,
    /// `Some(false)` the node exists but nothing replied (empty SIM slot),
    /// `None` never probed. A SIM bank creates a tty per channel whether or not
    /// a SIM is inserted, so this is the only way to tell the two apart.
    #[serde(default)]
    pub alive: Option<bool>,
    pub live_ready: bool,
    pub live_error: Option<String>,
}

/// Wall-clock cutoff (epoch millis) for a retention window: anything received
/// before this point has outlived its keep period.
pub fn retention_cutoff_ms(retention: Duration) -> i64 {
    let secs = i64::try_from(retention.as_secs()).unwrap_or(i64::MAX);
    (Utc::now() - chrono::Duration::seconds(secs)).timestamp_millis()
}

/// A missing/zero SCTS means the modem gave us no timestamp. Never treat that
/// as expired, or an undated message would be deleted the moment it is read.
pub fn is_expired(m: &SmsMessage, cutoff_ms: i64) -> bool {
    let t = m.received.timestamp_millis();
    t > 0 && t < cutoff_ms
}

/// Every SIM slot occupied by messages older than `cutoff_ms`, sorted and
/// deduplicated. A concatenated message contributes all of its fragment slots,
/// so nothing is left behind half-deleted.
pub fn expired_indices(msgs: &[SmsMessage], cutoff_ms: i64) -> Vec<i32> {
    let mut idxs: Vec<i32> = Vec::new();
    for m in msgs.iter().filter(|m| is_expired(m, cutoff_ms)) {
        if m.part_indices.len() > 1 {
            idxs.extend(m.part_indices.iter().copied());
        } else {
            idxs.push(m.index);
        }
    }
    idxs.sort_unstable();
    idxs.dedup();
    idxs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(index: i32, ago_secs: i64, parts: Vec<i32>) -> SmsMessage {
        SmsMessage {
            port: "ttyUSB0".into(),
            index,
            from: "MYTEL".into(),
            received: Utc::now() - chrono::Duration::seconds(ago_secs),
            status: "REC READ".into(),
            text: "x".into(),
            part_indices: parts,
        }
    }

    #[test]
    fn undated_messages_are_never_expired() {
        let mut m = msg(1, 0, vec![]);
        m.received = DateTime::UNIX_EPOCH;
        assert!(!is_expired(&m, retention_cutoff_ms(Duration::from_secs(60))));
    }

    #[test]
    fn only_messages_past_the_cutoff_are_collected() {
        let cutoff = retention_cutoff_ms(Duration::from_secs(3600));
        let msgs = vec![msg(1, 7200, vec![]), msg(2, 60, vec![])];
        assert_eq!(expired_indices(&msgs, cutoff), vec![1]);
    }

    #[test]
    fn concatenated_messages_contribute_every_fragment_slot() {
        let cutoff = retention_cutoff_ms(Duration::from_secs(3600));
        let msgs = vec![msg(3, 7200, vec![5, 3, 4]), msg(4, 7200, vec![4])];
        assert_eq!(expired_indices(&msgs, cutoff), vec![3, 4, 5]);
    }

    #[test]
    fn nothing_to_do_when_everything_is_fresh() {
        let cutoff = retention_cutoff_ms(Duration::from_secs(3600));
        assert!(expired_indices(&[msg(1, 10, vec![])], cutoff).is_empty());
    }
}
