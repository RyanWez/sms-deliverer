use crate::core::at::AtChannel;
use crate::core::decoder;
use crate::core::models::SmsMessage;
use crate::core::reassemble::Reassembler;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub enum LiveEvent {
    Ready {
        port: String,
    },
    Batch {
        port: String,
        items: Vec<SmsMessage>,
    },
    Sms {
        port: String,
        message: SmsMessage,
        is_new: bool,
    },
    Closed {
        port: String,
        error: Option<String>,
    },
}

pub fn run_live<F>(port_name: String, stop: Arc<AtomicBool>, on_event: F)
where
    F: Fn(LiveEvent) + Send + 'static,
{
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_live_inner(&port_name, &stop, &on_event);
    }));
    if result.is_err() {
        log::error!("Live worker crashed: {}", port_name);
        on_event(LiveEvent::Closed {
            port: port_name,
            error: Some("Worker crashed".into()),
        });
    }
}

fn run_live_inner<F>(port_name: &str, stop: &Arc<AtomicBool>, on_event: &F)
where
    F: Fn(LiveEvent) + Send + 'static,
{
    let mut ch = match AtChannel::open(port_name) {
        Ok(ch) => ch,
        Err(e) => {
            log::warn!("Live open {} failed: {}", port_name, e);
            on_event(LiveEvent::Closed {
                port: port_name.to_string(),
                error: Some(e),
            });
            return;
        }
    };

    // PDU mode lets us read the UDH of concatenated (long) SMS so fragments
    // can be joined into one complete message instead of truncated pieces.
    let pdu_ok = ch.send("ATE0;+CMGF=0", 4000).contains("OK");
    ch.send("AT+CNMI=2,1,0,0,0", 3000);
    let stale = ch.take_notifications();
    if !stale.is_empty() {
        log::debug!(
            "{}: dropped {} stale notification(s)",
            port_name,
            stale.len()
        );
    }

    let mut asm = Reassembler::new();

    // Initial batch of everything stored on the SIM.
    let initial: Vec<SmsMessage> = if pdu_ok {
        let r = ch.send("AT+CMGL=4", 15000);
        collect_parts(&r, port_name, &mut asm)
    } else {
        // Text-mode fallback: no UDH available; fragments appear as-is.
        let r = ch.send("AT+CMGL=\"ALL\"", 15000);
        if r.contains("+CMGL:") {
            decoder::parse_text_mode_list(&r, port_name)
        } else {
            Vec::new()
        }
    };
    if !initial.is_empty() {
        log::info!(
            "{}: live initial batch {} msg(s) (pdu={})",
            port_name,
            initial.len(),
            pdu_ok
        );
        on_event(LiveEvent::Batch {
            port: port_name.to_string(),
            items: initial,
        });
    }
    if ch.is_dead() {
        log::warn!("{}: port lost during startup", port_name);
        on_event(LiveEvent::Closed {
            port: port_name.to_string(),
            error: Some("Port lost".into()),
        });
        return;
    }

    on_event(LiveEvent::Ready {
        port: port_name.to_string(),
    });

    let mut queue: VecDeque<i32> = VecDeque::new();
    while !stop.load(Ordering::Relaxed) {
        if let Some(idx) = queue.pop_front() {
            for more in handle_cmgr(&mut ch, idx, port_name, pdu_ok, &mut asm, on_event) {
                queue.push_back(more);
            }
        } else if let Some(note) = ch.wait_notification(500) {
            if let Some(idx) = decoder::parse_cmti_index(note.trim()) {
                log::debug!("{}: +CMTI idx {}", port_name, idx);
                queue.push_back(idx);
            }
        }

        // Release incomplete concat groups after a grace period.
        for msg in asm.flush_stale(crate::core::reassemble::STALE_AFTER) {
            log::info!("{}: flushed incomplete concat SMS", port_name);
            on_event(LiveEvent::Sms {
                port: port_name.to_string(),
                message: msg,
                is_new: true,
            });
        }

        if ch.is_dead() {
            log::warn!("{}: port lost", port_name);
            on_event(LiveEvent::Closed {
                port: port_name.to_string(),
                error: Some("Port lost".into()),
            });
            return;
        }
    }

    ch.send("AT+CNMI=1,0,0,1,0", 1500);
    ch.send("AT+CSCS=\"GSM\"", 1000);
    if pdu_ok {
        ch.send("AT+CMGF=1", 1500);
    }
    on_event(LiveEvent::Closed {
        port: port_name.to_string(),
        error: None,
    });
}

/// Parse a CMGL response and feed every fragment through the reassembler,
/// returning assembled standalone messages (plus best-effort leftovers).
fn collect_parts(resp: &str, port_name: &str, asm: &mut Reassembler) -> Vec<SmsMessage> {
    let parts = decoder::parse_pdu_list(resp, port_name);
    let mut msgs: Vec<SmsMessage> = Vec::new();
    for d in parts {
        match d.concat {
            Some(c) => {
                if let Some(done) = asm.push(&d.message, c) {
                    msgs.push(done);
                }
            }
            None => msgs.push(d.message),
        }
    }
    msgs.extend(asm.flush_stale(Duration::ZERO));
    msgs
}

fn handle_cmgr<F>(
    ch: &mut AtChannel,
    idx: i32,
    port_name: &str,
    pdu_mode: bool,
    asm: &mut Reassembler,
    on_event: &F,
) -> Vec<i32>
where
    F: Fn(LiveEvent) + Send + 'static,
{
    let resp = ch.send(&format!("AT+CMGR={idx}"), 6000);

    let completed: Option<SmsMessage> = if pdu_mode {
        match decoder::parse_pdu_cmgr(&resp, port_name) {
            Some(info) => {
                log::info!(
                    "{}: live SMS read (idx {}){}",
                    port_name,
                    idx,
                    if info.concat.is_some() { " [concat]" } else { "" }
                );
                match info.concat {
                    Some(c) => asm.push(&info.message, c),
                    None => Some(info.message),
                }
            }
            None => {
                log::debug!("{}: CMGR {} -> no message parsed", port_name, idx);
                None
            }
        }
    } else {
        match decoder::parse_cmgr(&resp, port_name) {
            Some(msg) => {
                log::info!("{}: live SMS read (idx {})", port_name, idx);
                Some(msg)
            }
            None => {
                log::debug!("{}: CMGR {} -> no message parsed", port_name, idx);
                None
            }
        }
    };

    if let Some(message) = completed {
        on_event(LiveEvent::Sms {
            port: port_name.to_string(),
            message,
            is_new: true,
        });
    }

    let mut more = Vec::new();
    for note in ch.take_notifications() {
        if let Some(extra_idx) = decoder::parse_cmti_index(note.trim()) {
            more.push(extra_idx);
        }
    }
    more
}
