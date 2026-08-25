use crate::core::models::SmsMessage;
use crate::core::decoder;
use crate::core::modem;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::thread;

pub enum LiveEvent {
    Ready { port: String },
    Batch { port: String, items: Vec<SmsMessage> },
    Sms { port: String, message: SmsMessage, is_new: bool },
    Closed { port: String, error: Option<String> },
}

pub fn run_live<F>(
    port_name: String,
    stop: Arc<AtomicBool>,
    on_event: F,
) where F: Fn(LiveEvent) + Send + 'static {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_live_inner(&port_name, &stop, &on_event);
    }));
    if let Err(_) = result {
        let _ = on_event(LiveEvent::Closed {
            port: port_name,
            error: Some("Worker crashed".into()),
        });
    }
}

fn run_live_inner<F>(port_name: &str, stop: &Arc<AtomicBool>, on_event: &F)
where F: Fn(LiveEvent) + Send + 'static {
    let mut sp = match modem::open_port(port_name) {
        Ok(sp) => sp,
        Err(e) => {
            let _ = on_event(LiveEvent::Closed { port: port_name.to_string(), error: Some(e) });
            return;
        }
    };
    let _ = sp.write_all(b"ATE0;+CMGF=1;+CSCS=\"UCS2\"\r");
    thread::sleep(Duration::from_millis(200));
    let mut buf = [0u8; 1024];
    let _ = drain(&mut sp, &mut buf);
    let _ = sp.write_all(b"AT+CNMI=2,1,0,0,0\r");
    thread::sleep(Duration::from_millis(200));
    let _ = drain(&mut sp, &mut buf);
    let _ = sp.write_all(b"AT+CMGL=\"ALL\"\r");
    thread::sleep(Duration::from_millis(200));
    let mut r = String::new();
    let end = Instant::now() + Duration::from_secs(15);
    while Instant::now() < end {
        let mut chunk = [0u8; 1024];
        if let Ok(n) = sp.read(&mut chunk) {
            if n > 0 { r.push_str(&String::from_utf8_lossy(&chunk[..n])); }
        }
        if r.contains("OK") || r.contains("ERROR") { break; }
        thread::sleep(Duration::from_millis(15));
    }
    if r.contains("+CMGL:") {
        let initial = decoder::parse_text_mode_list(&r, port_name);
        if !initial.is_empty() {
            let _ = on_event(LiveEvent::Batch { port: port_name.to_string(), items: initial });
        }
    }
    let _ = on_event(LiveEvent::Ready { port: port_name.to_string() });
    let mut ibuf = String::new();
    let mut cmti_queue: Vec<i32> = Vec::new();
    while !stop.load(Ordering::Relaxed) {
        let mut chunk = [0u8; 1024];
        match sp.read(&mut chunk) {
            Ok(n) if n > 0 => {
                ibuf.push_str(&String::from_utf8_lossy(&chunk[..n]));
                while let Some(pos) = ibuf.find('\n') {
                    let line: String = ibuf.drain(..=pos).collect();
                    if let Some(idx) = decoder::parse_cmti_index(line.trim()) {
                        cmti_queue.push(idx);
                    }
                }
                if ibuf.len() > 8192 {
                    let keep = ibuf.len() - 512;
                    ibuf.drain(..keep);
                }
            }
            _ => {}
        }
        if let Some(idx) = cmti_queue.first().copied() {
            cmti_queue.remove(0);
            let more = handle_cmti(&mut sp, idx, port_name, on_event);
            cmti_queue.extend(more);
        } else {
            thread::sleep(Duration::from_millis(150));
        }
    }
    let _ = sp.write_all(b"AT+CNMI=1,0,0,1,0\r");
    let _ = sp.write_all(b"AT+CSCS=\"GSM\"\r");
    let _ = on_event(LiveEvent::Closed { port: port_name.to_string(), error: None });
}

fn drain(sp: &mut Box<dyn serialport::SerialPort>, buf: &mut [u8; 1024]) -> String {
    let mut result = String::new();
    loop {
        match sp.read(buf) {
            Ok(n) if n > 0 => result.push_str(&String::from_utf8_lossy(&buf[..n])),
            _ => break,
        }
    }
    result
}

fn handle_cmti<F>(
    sp: &mut Box<dyn serialport::SerialPort>,
    idx: i32,
    port_name: &str,
    on_event: &F,
) -> Vec<i32>
where F: Fn(LiveEvent) + Send + 'static {
    let _ = sp.write_all(format!("AT+CMGR={idx}\r").as_bytes());
    let mut sb = String::new();
    let end = Instant::now() + Duration::from_secs(6);
    while Instant::now() < end {
        let mut chunk = [0u8; 1024];
        if let Ok(n) = sp.read(&mut chunk) {
            sb.push_str(&String::from_utf8_lossy(&chunk[..n]));
        }
        if sb.contains("\r\nOK") || sb.contains("ERROR") {
            let end2 = Instant::now() + Duration::from_millis(400);
            while Instant::now() < end2 {
                let mut extra = [0u8; 1024];
                if let Ok(n) = sp.read(&mut extra) {
                    sb.push_str(&String::from_utf8_lossy(&extra[..n]));
                }
                thread::sleep(Duration::from_millis(30));
            }
            break;
        }
        thread::sleep(Duration::from_millis(40));
    }
    let mut more = Vec::new();
    for line in decoder::find_cmti(&sb) {
        if let Some(more_idx) = decoder::parse_cmti_index(&line) {
            more.push(more_idx);
        }
    }
    if let Some(msg) = decoder::parse_cmgr(&sb, port_name) {
        let _ = on_event(LiveEvent::Sms {
            port: port_name.to_string(),
            message: msg,
            is_new: true,
        });
    }
    more
}