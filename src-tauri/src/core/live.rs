use crate::core::at::AtChannel;
use crate::core::decoder;
use crate::core::models::SmsMessage;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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

    ch.send("ATE0;+CMGF=1;+CSCS=\"UCS2\"", 4000);
    ch.send("AT+CNMI=2,1,0,0,0", 3000);
    let stale = ch.take_notifications();
    if !stale.is_empty() {
        log::debug!(
            "{}: dropped {} stale notification(s)",
            port_name,
            stale.len()
        );
    }

    let r = ch.send("AT+CMGL=\"ALL\"", 15000);
    if r.contains("+CMGL:") {
        let initial = decoder::parse_text_mode_list(&r, port_name);
        if !initial.is_empty() {
            log::info!("{}: live initial batch {} msg(s)", port_name, initial.len());
            on_event(LiveEvent::Batch {
                port: port_name.to_string(),
                items: initial,
            });
        }
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
            for more in handle_cmgr(&mut ch, idx, port_name, on_event) {
                queue.push_back(more);
            }
        } else if let Some(note) = ch.wait_notification(500) {
            if let Some(idx) = decoder::parse_cmti_index(note.trim()) {
                log::debug!("{}: +CMTI idx {}", port_name, idx);
                queue.push_back(idx);
            }
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
    on_event(LiveEvent::Closed {
        port: port_name.to_string(),
        error: None,
    });
}

fn handle_cmgr<F>(ch: &mut AtChannel, idx: i32, port_name: &str, on_event: &F) -> Vec<i32>
where
    F: Fn(LiveEvent) + Send + 'static,
{
    let resp = ch.send(&format!("AT+CMGR={idx}"), 6000);

    if let Some(msg) = decoder::parse_cmgr(&resp, port_name) {
        log::info!("{}: live SMS read (idx {})", port_name, idx);
        on_event(LiveEvent::Sms {
            port: port_name.to_string(),
            message: msg,
            is_new: true,
        });
    } else {
        log::debug!("{}: CMGR {} -> no message parsed", port_name, idx);
    }

    let mut more = Vec::new();
    for note in ch.take_notifications() {
        if let Some(extra_idx) = decoder::parse_cmti_index(note.trim()) {
            more.push(extra_idx);
        }
    }
    more
}
