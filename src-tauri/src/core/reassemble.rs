//! Reassembly of concatenated (multipart) SMS fragments.
//!
//! Long SMS (>160 GSM-7 chars / >70 UCS-2 chars) arrive as several SIM-stored
//! fragments, each carrying a User Data Header that references its group
//! (`ref_num`, `total`, `seq`). This component collects fragments and yields
//! one fully assembled [`SmsMessage`] once every part has been seen.
//! Incomplete groups are flushed best-effort after a timeout so nothing is
//! silently lost forever.

use crate::core::decoder::ConcatInfo;
use crate::core::models::SmsMessage;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// How long an incomplete group is kept waiting for missing parts.
pub const STALE_AFTER: Duration = Duration::from_secs(90);

struct Part {
    seq: u8,
    index: i32,
    text: String,
}

struct Group {
    port: String,
    total: u8,
    from: String,
    received: chrono::DateTime<chrono::Utc>,
    status: String,
    parts: Vec<Part>,
    last_seen: Instant,
}

#[derive(Default)]
pub struct Reassembler {
    groups: HashMap<u16, Group>,
}

impl Reassembler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed one fragment. Returns the assembled message when all parts have
    /// arrived, otherwise `None` while still incomplete.
    pub fn push(&mut self, msg: &SmsMessage, c: ConcatInfo) -> Option<SmsMessage> {
        // Malformed header (total 0/1 or nonsensical seq): treat as standalone.
        if c.total <= 1 || c.seq == 0 || c.seq > c.total {
            return Some(msg.clone());
        }

        let g = self.groups.entry(c.ref_num).or_insert_with(|| Group {
            port: msg.port.clone(),
            total: c.total,
            from: msg.from.clone(),
            received: msg.received,
            status: msg.status.clone(),
            parts: Vec::new(),
            last_seen: Instant::now(),
        });
        g.last_seen = Instant::now();
        if c.seq == 1 && !msg.received.to_rfc3339().starts_with("1970") {
            g.received = msg.received;
            g.status = msg.status.clone();
        }

        if let Some(p) = g.parts.iter_mut().find(|p| p.seq == c.seq) {
            p.text = msg.text.clone();
            p.index = msg.index;
        } else {
            g.parts.push(Part {
                seq: c.seq,
                index: msg.index,
                text: msg.text.clone(),
            });
        }

        if g.parts.len() >= g.total as usize {
            let g = self.groups.remove(&c.ref_num)?;
            return Some(finish(g));
        }
        None
    }

    /// Emit best-effort joined messages for groups whose last seen fragment is
    /// older than `older_than`. A duration of ZERO flushes everything (used by
    /// one-shot scans).
    pub fn flush_stale(&mut self, older_than: Duration) -> Vec<SmsMessage> {
        let stale: Vec<u16> = self
            .groups
            .iter()
            .filter(|(_, g)| g.last_seen.elapsed() >= older_than)
            .map(|(k, _)| *k)
            .collect();
        stale
            .into_iter()
            .filter_map(|k| self.groups.remove(&k))
            .inspect(|g| {
                log::warn!(
                    "concat group flushed incomplete: {} of {} part(s)",
                    g.parts.len(),
                    g.total
                );
            })
            .map(finish)
            .collect()
    }

    pub fn pending_groups(&self) -> usize {
        self.groups.len()
    }
}

fn finish(g: Group) -> SmsMessage {
    let mut parts = g.parts;
    parts.sort_by_key(|p| p.seq);

    let mut indices: Vec<i32> = parts.iter().map(|p| p.index).collect();
    indices.sort_unstable();
    indices.dedup();

    let text: String = parts.iter().map(|p| p.text.as_str()).collect();
    let received_min = parts.iter().map(|_| ()).next(); // silence unused lint helper
    let _ = received_min;

    let index = indices.first().copied().unwrap_or(0);
    log::info!(
        "concatenated SMS assembled: {} part(s), {} chars",
        parts.len(),
        text.chars().count()
    );

    SmsMessage {
        port: g.port,
        index,
        from: g.from,
        received: g.received,
        status: g.status,
        text,
        part_indices: indices,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frag(index: i32, text: &str) -> SmsMessage {
        SmsMessage {
            port: "ttyUSB0".into(),
            index,
            from: "09780001122".into(),
            received: chrono::Utc::now(),
            status: "REC UNREAD".into(),
            text: text.into(),
            part_indices: Vec::new(),
        }
    }

    fn concat(ref_num: u16, total: u8, seq: u8) -> ConcatInfo {
        ConcatInfo { ref_num, total, seq }
    }

    #[test]
    fn assembles_in_order() {
        let mut r = Reassembler::new();
        assert!(r
            .push(&frag(1, "[seq2]"), concat(7, 3, 2))
            .is_none());
        assert!(r.push(&frag(2, "[seq3]"), concat(7, 3, 3)).is_none());
        let done = r.push(&frag(3, "[seq1]"), concat(7, 3, 1)).unwrap();
        assert_eq!(done.text, "[seq1][seq2][seq3]");
        assert_eq!(done.index, 1);
        assert_eq!(done.part_indices, vec![1, 2, 3]);
        assert_eq!(r.pending_groups(), 0);
    }

    #[test]
    fn duplicate_seq_replaces_part() {
        let mut r = Reassembler::new();
        assert!(r.push(&frag(1, "bad"), concat(9, 2, 1)).is_none());
        assert!(r.push(&frag(1, "good "), concat(9, 2, 1)).is_none());
        assert!(r.push(&frag(2, "end"), concat(9, 2, 2)).is_some());
        let mut r2 = Reassembler::new();
        r2.push(&frag(1, "good "), concat(9, 2, 1));
        let done = r2.push(&frag(2, "end"), concat(9, 2, 2)).unwrap();
        assert_eq!(done.text, "good end");
    }

    #[test]
    fn malformed_header_is_standalone() {
        let mut r = Reassembler::new();
        let done = r.push(&frag(4, "solo"), concat(5, 1, 1)).unwrap();
        assert_eq!(done.text, "solo");
        let done = r.push(&frag(4, "solo"), concat(5, 4, 9)).unwrap();
        assert_eq!(done.text, "solo");
    }

    #[test]
    fn flush_zero_emits_everything_best_effort() {
        let mut r = Reassembler::new();
        assert!(r.push(&frag(1, "only-one "), concat(11, 3, 1)).is_none());
        let out = r.flush_stale(Duration::ZERO);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].text, "only-one ");
        assert_eq!(out[0].part_indices, vec![1]);
    }

    #[test]
    fn flush_keeps_fresh_groups() {
        let mut r = Reassembler::new();
        assert!(r.push(&frag(1, "x"), concat(12, 2, 1)).is_none());
        let out = r.flush_stale(Duration::from_secs(60));
        assert!(out.is_empty());
        assert_eq!(r.pending_groups(), 1);
    }
}