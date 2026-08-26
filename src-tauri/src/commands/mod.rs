use crate::core::models::*;
use crate::core::decoder;
use crate::core::settings::Settings;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::Emitter;

pub struct AppStateInner {
    pub settings: Settings,
    pub next_id: u64,
    pub ports: Vec<PortInfo>,
    pub messages: Vec<SmsItem>,
    pub scan_busy: bool,
    pub live_on: bool,
    pub live_ports_ready: Vec<String>,
    pub live_failed: Vec<(String, String)>,
    pub live_stop: Option<Arc<AtomicBool>>,
    pub ussd_busy: bool,
    pub delete_busy: bool,
    pub status_text: String,
    pub failed_notes: Vec<String>,
}

impl AppStateInner {
    pub fn take_next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

pub type SharedState = Arc<Mutex<AppStateInner>>;

pub fn new_shared_state() -> SharedState {
    let settings = Settings::load();
    let ports: Vec<PortInfo> = crate::core::modem::get_port_names()
        .into_iter()
        .map(|name| {
            let sim = settings.sim_of(&name).to_string();
            PortInfo {
                name,
                checked: true,
                sim_number: sim,
                live_ready: false,
                live_error: None,
            }
        })
        .collect();
    Arc::new(Mutex::new(AppStateInner {
        settings,
        next_id: 1,
        ports,
        messages: Vec::new(),
        scan_busy: false,
        live_on: false,
        live_ports_ready: Vec::new(),
        live_failed: Vec::new(),
        live_stop: None,
        ussd_busy: false,
        delete_busy: false,
        status_text: String::new(),
        failed_notes: Vec::new(),
    }))
}

#[tauri::command]
pub fn refresh_ports(state: tauri::State<'_, SharedState>) -> Vec<PortInfo> {
    let names = crate::core::modem::get_port_names();
    let mut st = state.lock().unwrap();
    let old_map: std::collections::HashMap<String, PortInfo> =
        st.ports.drain(..).map(|p| (p.name.clone(), p)).collect();
    let ports: Vec<PortInfo> = names
        .into_iter()
        .map(|n| {
            if let Some(mut old) = old_map.get(&n).cloned() {
                old.sim_number = st.settings.sim_of(&n).to_string();
                old
            } else {
                PortInfo {
                    name: n.clone(),
                    checked: true,
                    sim_number: st.settings.sim_of(&n).to_string(),
                    live_ready: false,
                    live_error: None,
                }
            }
        })
        .collect();
    st.ports = ports.clone();
    ports
}

#[tauri::command]
pub fn checked_ports(state: tauri::State<'_, SharedState>) -> Vec<String> {
    let st = state.lock().unwrap();
    st.ports
        .iter()
        .filter(|p| p.checked)
        .map(|p| p.name.clone())
        .collect()
}

#[tauri::command]
pub fn toggle_port_checked(
    state: tauri::State<'_, SharedState>,
    port: String,
    checked: bool,
) {
    let mut st = state.lock().unwrap();
    if let Some(p) = st.ports.iter_mut().find(|p| p.name == port) {
        p.checked = checked;
    }
}

#[tauri::command]
pub fn set_all_ports_checked(state: tauri::State<'_, SharedState>, checked: bool) {
    let mut st = state.lock().unwrap();
    for p in &mut st.ports {
        p.checked = checked;
    }
}

#[tauri::command]
pub fn start_scan(
    state: tauri::State<'_, SharedState>,
) -> Result<String, String> {
    let ports = checked_ports(state.clone());
    if ports.is_empty() {
        return Err("No COM port selected.".into());
    }
    {
        let mut st = state.lock().unwrap();
        if st.scan_busy || st.live_on || st.ussd_busy || st.delete_busy {
            return Err("Busy".into());
        }
        st.scan_busy = true;
        st.messages.clear();
    }
    let state_clone = Arc::clone(&state);
    thread::spawn(move || {
        let mut all_msgs: Vec<SmsItem> = Vec::new();
        let mut failures: Vec<String> = Vec::new();
        let total = ports.len();
        for (done, port) in ports.iter().enumerate() {
            let r = crate::core::modem::read_port(port);
            if r.ok {
                for m in r.messages {
                    let id = {
                        let mut st = state_clone.lock().unwrap();
                        st.take_next_id()
                    };
                    let otp = decoder::extract_otp(&m.text);
                    all_msgs.push(SmsItem {
                        id,
                        message: m,
                        otp,
                        is_new: false,
                    });
                }
            } else {
                failures.push(format!("{} ({})", port, r.error.unwrap_or_default()));
            }
            let mut st = state_clone.lock().unwrap();
            st.messages = all_msgs.clone();
            st.status_text = format!(
                "Reading {}/{}  |  Messages: {}{}",
                done + 1,
                total,
                all_msgs.len(),
                if failures.is_empty() {
                    String::new()
                } else {
                    format!("  |  FAILED: {}", failures.len())
                }
            );
        }
        let mut st = state_clone.lock().unwrap();
        st.scan_busy = false;
        st.failed_notes = failures.clone();
        st.status_text = format!(
            "Done. Modems OK: {}/{}  |  Total messages: {}{}",
            total - failures.len(),
            total,
            all_msgs.len(),
            if failures.is_empty() {
                String::new()
            } else {
                format!("  |  FAILED: {}", failures.join(", "))
            }
        );
    });
    Ok("Scan started".into())
}

#[tauri::command]
pub fn get_sim_numbers(
    state: tauri::State<'_, SharedState>,
) -> Result<String, String> {
    let ports = checked_ports(state.clone());
    if ports.is_empty() {
        return Err("No COM port selected.".into());
    }
    {
        let mut st = state.lock().unwrap();
        if st.ussd_busy || st.scan_busy || st.live_on || st.delete_busy {
            return Err("Busy".into());
        }
        st.ussd_busy = true;
        st.status_text = format!("Requesting SIM numbers 0/{}...", ports.len());
    }
    let state_clone = Arc::clone(&state);
    thread::spawn(move || {
        let total = ports.len();
        let found = Arc::new(AtomicUsize::new(0));
        let done = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::with_capacity(total);
        for port in ports {
            let st2 = Arc::clone(&state_clone);
            let found2 = Arc::clone(&found);
            let done2 = Arc::clone(&done);
            handles.push(thread::spawn(move || {
                let (num, _err) = crate::core::modem::get_sim_number(&port);
                if let Some(ref n) = num {
                    found2.fetch_add(1, Ordering::Relaxed);
                    let mut st = st2.lock().unwrap();
                    st.settings.sim_numbers.insert(port.clone(), n.clone());
                    if let Some(p) = st.ports.iter_mut().find(|p| p.name == port) {
                        p.sim_number = n.clone();
                    }
                }
                let d = done2.fetch_add(1, Ordering::Relaxed) + 1;
                let mut st = st2.lock().unwrap();
                st.status_text = format!(
                    "Requesting SIM numbers {}/{}  |  Found: {}",
                    d,
                    total,
                    found2.load(Ordering::Relaxed)
                );
            }));
        }
        for h in handles {
            let _ = h.join();
        }
        let f = found.load(Ordering::Relaxed);
        let mut st = state_clone.lock().unwrap();
        st.ussd_busy = false;
        st.settings.save_sim_numbers();
        st.settings.save();
        st.status_text = format!(
            "SIM numbers updated. Found: {}/{}   (saved)",
            f, total
        );
    });
    Ok("USSD started".into())
}

#[tauri::command]
pub fn get_messages(state: tauri::State<'_, SharedState>) -> Vec<SmsItem> {
    state.lock().unwrap().messages.clone()
}

#[tauri::command]
pub fn get_ports(state: tauri::State<'_, SharedState>) -> Vec<PortInfo> {
    state.lock().unwrap().ports.clone()
}

#[tauri::command]
pub fn get_status_text(state: tauri::State<'_, SharedState>) -> String {
    state.lock().unwrap().status_text.clone()
}

#[tauri::command]
pub fn start_live(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
) -> Result<String, String> {
    let ports = {
        let mut st = state.lock().unwrap();
        if st.live_on || st.live_stop.is_some() || st.scan_busy || st.ussd_busy || st.delete_busy {
            return Err("Busy".into());
        }
        for p in &mut st.ports {
            p.live_ready = false;
            p.live_error = None;
        }
        st.ports
            .iter()
            .filter(|p| p.checked)
            .map(|p| p.name.clone())
            .collect::<Vec<_>>()
    };
    if ports.is_empty() {
        return Err("No COM port selected.".into());
    }
    {
        let mut st = state.lock().unwrap();
        st.live_on = true;
        st.live_ports_ready.clear();
        st.live_failed.clear();
        st.messages.clear();
        let stop = Arc::new(AtomicBool::new(false));
        st.live_stop = Some(Arc::clone(&stop));
        st.status_text = format!("Starting live on {} port(s)...", ports.len());
    }
    let state_clone = Arc::clone(&state);
    thread::spawn(move || {
        let shared_stop = {
            let st = state_clone.lock().unwrap();
            st.live_stop.as_ref().unwrap().clone()
        };
        let port_count = ports.len();
        let mut handles = Vec::with_capacity(port_count);
        for port in ports {
            let stop = Arc::clone(&shared_stop);
            let st2 = Arc::clone(&state_clone);
            let app2 = app.clone();
            handles.push(thread::spawn(move || {
                let sender = move |evt: crate::core::live::LiveEvent| {
                    match evt {
                        crate::core::live::LiveEvent::Ready { port } => {
                            let mut st = st2.lock().unwrap();
                            if let Some(p) = st.ports.iter_mut().find(|p| p.name == port) {
                                p.live_ready = true;
                                p.live_error = None;
                            }
                            st.live_ports_ready.push(port.clone());
                            st.status_text = format!(
                                "Live {} port(s) ready...",
                                st.live_ports_ready.len()
                            );
                            drop(st);
                            let _ = app2.emit("sms:ready", &serde_json::json!({ "port": port }));
                        }
                        crate::core::live::LiveEvent::Batch { port: _port, items } => {
                            let mut st = st2.lock().unwrap();
                            for m in items {
                                let id = st.take_next_id();
                                let otp = crate::core::decoder::extract_otp(&m.text);
                                st.messages.push(SmsItem {
                                    id,
                                    message: m,
                                    otp,
                                    is_new: false,
                                });
                            }
                        }
                        crate::core::live::LiveEvent::Sms {
                            port,
                            message,
                            is_new,
                        } => {
                            let mut st = st2.lock().unwrap();
                            let id = st.take_next_id();
                            let otp = crate::core::decoder::extract_otp(&message.text);
                            let item = SmsItem {
                                id,
                                message: message.clone(),
                                otp: otp.clone(),
                                is_new,
                            };
                            st.messages.push(item);
                            drop(st);
                            let _ = app2.emit("sms:new", &serde_json::json!({
                                "id": id,
                                "message": message,
                                "otp": otp,
                                "port": port,
                                "is_new": is_new,
                            }));
                        }
                        crate::core::live::LiveEvent::Closed { port, error } => {
                            let mut st = st2.lock().unwrap();
                            if let Some(e) = error {
                                if let Some(p) = st.ports.iter_mut().find(|p| p.name == port) {
                                    p.live_ready = false;
                                    p.live_error = Some(e.clone());
                                }
                                st.live_failed.push((port.clone(), e.clone()));
                                st.status_text = format!("{} FAILED: {}", port, e);
                            }
                        }
                    }
                };
                crate::core::live::run_live(port, stop, sender);
            }));
        }
        for h in handles {
            let _ = h.join();
        }
        let mut st = state_clone.lock().unwrap();
        st.live_on = false;
        st.live_stop = None;
        st.status_text = format!(
            "Live stopped. Messages: {}",
            st.messages.len()
        );
    });
    Ok("Live started".into())
}

#[tauri::command]
pub fn stop_live(state: tauri::State<'_, SharedState>) {
    let mut st = state.lock().unwrap();
    if let Some(ref stop) = st.live_stop {
        stop.store(true, Ordering::Relaxed);
    }
    st.live_on = false;
    st.status_text = "Stopping live...".into();
}

#[tauri::command]
pub fn delete_selected(
    state: tauri::State<'_, SharedState>,
    ids: Vec<u64>,
) -> Result<String, String> {
    {
        let st = state.lock().unwrap();
        if st.scan_busy || st.live_on || st.ussd_busy || st.delete_busy {
            return Err("Busy".into());
        }
    }
    let to_delete: Vec<(String, Vec<i32>)>;
    {
        let st = state.lock().unwrap();
        let mut map: std::collections::HashMap<String, Vec<i32>> = std::collections::HashMap::new();
        for id in &ids {
            if let Some(item) = st.messages.iter().find(|m| m.id == *id) {
                map.entry(item.message.port.clone())
                    .or_default()
                    .push(item.message.index);
            }
        }
        to_delete = map.into_iter().collect();
    }
    if to_delete.is_empty() {
        return Err("No messages selected.".into());
    }
    {
        let mut st = state.lock().unwrap();
        st.delete_busy = true;
    }
    let state_clone = Arc::clone(&state);
    thread::spawn(move || {
        let mut ok = 0usize;
        let mut fail = 0usize;
        for (port, indices) in &to_delete {
            let r = crate::core::modem::delete_messages(port, Some(indices.as_slice()));
            if r.ok { ok += 1; } else { fail += 1; }
        }
        let mut st = state_clone.lock().unwrap();
        st.delete_busy = false;
        st.messages.retain(|m| !ids.contains(&m.id));
        st.status_text = format!("Deleted: {} ok, {} fail", ok, fail);
    });
    Ok("Delete started".into())
}

#[tauri::command]
pub fn clear_all(state: tauri::State<'_, SharedState>) {
    let mut st = state.lock().unwrap();
    st.messages.clear();
    st.status_text = "Cleared".into();
}