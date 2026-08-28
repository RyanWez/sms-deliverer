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

#[derive(Clone)]
struct Part {
    seq: u8,
    index: i32,
    text: String,
}

#[derive(Clone)]
struct Group {
    port: String,
    total: u8,
    from: String,
    received: chrono::DateTime<chrono::Utc>,
    status: String,
    parts: Vec<Part>,
    last_seen: Instant,
}

/// Identity of a multipart group. Keying on `(from, ref_num, total)` instead of
/// the reference number alone keeps two different senders that happen to reuse
/// the same 8/16-bit reference — or a sender reusing a stale ref for a *new*
/// message of a different size — from silently merging into one corrupt group.
#[derive(Clone, Hash, PartialEq, Eq)]
struct GroupKey {
    from: String,
    ref_num: u16,
    total: u8,
}

#[derive(Default)]
pub struct Reassembler {
    groups: HashMap<GroupKey, Group>,
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

        let key = GroupKey {
            from: msg.from.clone(),
            ref_num: c.ref_num,
            total: c.total,
        };
        let g = self.groups.entry(key.clone()).or_insert_with(|| Group {
            port: msg.port.clone(),
            total: c.total,
            from: msg.from.clone(),
            received: msg.received,
            status: msg.status.clone(),
            parts: Vec::new(),
            last_seen: Instant::now(),
        });
        g.last_seen = Instant::now();
        if c.seq == 1 && msg.received.timestamp_millis() > 0 {
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
            let g = self.groups.remove(&key)?;
            return Some(finish(g));
        }
        None
    }

    /// Emit best-effort joined messages for groups whose last seen fragment is
    /// older than `older_than`. A duration of ZERO flushes everything (used by
    /// one-shot scans).
    pub fn flush_stale(&mut self, older_than: Duration) -> Vec<SmsMessage> {
        let stale: Vec<GroupKey> = self
            .groups
            .iter()
            .filter(|(_, g)| g.last_seen.elapsed() >= older_than)
            .map(|(k, _)| k.clone())
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

    /// Best-effort copy of one group's currently collected parts, joined in
    /// sequence order — without removing the group. Used to surface a partial
    /// message immediately (so nothing on the SIM ever feels hidden) while the
    /// reassembler keeps waiting for the remaining parts; when they arrive, the
    /// complete message replaces the partial one.
    pub fn peek_partials(&self) -> Vec<SmsMessage> {
        let mut out: Vec<SmsMessage> = Vec::new();
        for g in self.groups.values() {
            let mut parts = g.parts.clone();
            parts.sort_by_key(|p| p.seq);
            let text: String = parts.iter().map(|p| p.text.as_str()).collect();
            let mut indices: Vec<i32> = parts.iter().map(|p| p.index).collect();
            indices.sort_unstable();
            indices.dedup();
            out.push(SmsMessage {
                port: g.port.clone(),
                index: indices.first().copied().unwrap_or(0),
                from: g.from.clone(),
                received: g.received,
                status: g.status.clone(),
                text,
                part_indices: indices,
            });
        }
        out
    }
}

fn finish(g: Group) -> SmsMessage {
    let mut parts = g.parts;
    parts.sort_by_key(|p| p.seq);

    let mut indices: Vec<i32> = parts.iter().map(|p| p.index).collect();
    indices.sort_unstable();
    indices.dedup();

    let text: String = parts.iter().map(|p| p.text.as_str()).collect();

    let index = indices.first().copied().unwrap_or(0);

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

    #[test]
    fn peek_partials_returns_pending_without_removing() {
        let mut r = Reassembler::new();
        assert!(r.push(&frag(1, "hello "), concat(30, 3, 1)).is_none());
        let partials = r.peek_partials();
        assert_eq!(partials.len(), 1);
        assert_eq!(partials[0].text, "hello ");
        // Group is still pending and can still complete afterwards.
        assert_eq!(r.pending_groups(), 1);
        assert!(r.push(&frag(2, "wor"), concat(30, 3, 2)).is_none());
        let done = r.push(&frag(3, "ld"), concat(30, 3, 3)).unwrap();
        assert_eq!(done.text, "hello world");
        assert_eq!(r.pending_groups(), 0);
        assert!(r.peek_partials().is_empty());
    }

    fn frag_from(index: i32, from: &str, text: &str) -> SmsMessage {
        SmsMessage {
            from: from.into(),
            ..frag(index, text)
        }
    }

    #[test]
    fn same_ref_different_senders_stay_separate() {
        let mut r = Reassembler::new();
        // Two senders that both used ref 42 for a 2-part message.
        assert!(r.push(&frag_from(1, "MYTEL", "A1"), concat(42, 2, 1)).is_none());
        assert!(r.push(&frag_from(2, "KBZPay", "B1"), concat(42, 2, 1)).is_none());
        let done_a = r.push(&frag_from(3, "MYTEL", "A2"), concat(42, 2, 2)).unwrap();
        let done_b = r.push(&frag_from(4, "KBZPay", "B2"), concat(42, 2, 2)).unwrap();
        assert_eq!(done_a.text, "A1A2");
        assert_eq!(done_b.text, "B1B2");
        assert_eq!(r.pending_groups(), 0);
    }

    #[test]
    fn ref_reuse_with_different_total_stays_separate() {
        let mut r = Reassembler::new();
        // Sender reuses ref 7: an old 4-part group and a fresh 2-part message.
        assert!(r.push(&frag_from(1, "MYTEL", "old1"), concat(7, 4, 1)).is_none());
        assert!(r.push(&frag_from(2, "MYTEL", "new1"), concat(7, 2, 1)).is_none());
        let done = r.push(&frag_from(3, "MYTEL", "new2"), concat(7, 2, 2)).unwrap();
        assert_eq!(done.text, "new1new2");
        // The stale 4-part group is still pending, untouched by the new message.
        assert_eq!(r.pending_groups(), 1);
    }
}