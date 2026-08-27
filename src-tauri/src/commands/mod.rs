use crate::core::decoder;
use crate::core::models::*;
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
    pub scan_done: usize,
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
            let path = crate::core::modem::stable_id(&name);
            let sim = settings.sim_of_stable(&path, &name);
            PortInfo {
                name,
                path,
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
        scan_done: 0,
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
                old.path = crate::core::modem::stable_id(&n);
                old.sim_number = st.settings.sim_of_stable(&old.path, &n);
                old.live_ready = false;
                old.live_error = None;
                old
            } else {
                let path = crate::core::modem::stable_id(&n);
                PortInfo {
                    name: n.clone(),
                    path: path.clone(),
                    checked: true,
                    sim_number: st.settings.sim_of_stable(&path, &n),
                    live_ready: false,
                    live_error: None,
                }
            }
        })
        .collect();
    st.ports = ports.clone();
    log::debug!("Ports refreshed: {}", ports.len());
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
pub fn toggle_port_checked(state: tauri::State<'_, SharedState>, port: String, checked: bool) {
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

fn scan_progress_status(done: usize, total: usize, msgs: usize, failed: usize) -> String {
    format!(
        "Reading {}/{}  |  Messages: {}{}",
        done,
        total,
        msgs,
        if failed == 0 {
            String::new()
        } else {
            format!("  |  FAILED: {}", failed)
        }
    )
}

fn scan_done_status(ok: usize, total: usize, msgs: usize, failed_notes: &[String]) -> String {
    format!(
        "Done. Modems OK: {}/{}  |  Total messages: {}{}",
        ok,
        total,
        msgs,
        if failed_notes.is_empty() {
            String::new()
        } else {
            format!("  |  FAILED: {}", failed_notes.join(", "))
        }
    )
}

#[tauri::command]
pub fn start_scan(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
) -> Result<String, String> {
    let ports = checked_ports(state.clone());
    if ports.is_empty() {
        return Err("No COM port selected.".into());
    }
    let initial_status;
    {
        let mut st = state.lock().unwrap();
        if st.scan_busy || st.live_on || st.ussd_busy || st.delete_busy {
            return Err("Busy".into());
        }
        st.scan_busy = true;
        st.scan_done = 0;
        st.messages.clear();
        st.failed_notes.clear();
        st.status_text = scan_progress_status(0, ports.len(), 0, 0);
        initial_status = st.status_text.clone();
    }
    let _ = app.emit("messages:reset", &serde_json::json!({}));
    let _ = app.emit(
        "status:update",
        &serde_json::json!({ "text": initial_status }),
    );

    let total = ports.len();
    log::info!("Scan started on {} port(s)", total);

    // Cap concurrent port reads to protect the USB bridge from contention.
    // With 60+ sticks, firing 64 simultaneous AT probes backs up the modem
    // hardware and the OS USB stack; a bounded worker pool fan-out preserves
    // the shared-state/progress accounting while limiting parallelism.
    const MAX_CONCURRENT_SCAN: usize = 16;
    let work = Arc::new(Mutex::new(ports));
    let workers = total.min(MAX_CONCURRENT_SCAN);
    let mut handles = Vec::with_capacity(workers);
    for _ in 0..workers {
        let work2 = Arc::clone(&work);
        let state_clone = Arc::clone(&state);
        let app2 = app.clone();
        handles.push(thread::spawn(move || loop {
            let port = match work2.lock().unwrap().pop() {
                Some(port) => port,
                None => break,
            };
            let result = crate::core::modem::read_port(&port);
            let crate::core::modem::ReadResult {
                ok,
                messages,
                error,
            } = result;
            let count = messages.len();

            let mut new_items: Vec<SmsItem> = Vec::new();
            let status_text;
            {
                let mut st = state_clone.lock().unwrap();

                if !ok {
                    let err = error.unwrap_or_default();
                    log::warn!("Scan {}: FAILED ({})", port, err);
                    st.failed_notes.push(format!("{} ({})", port, err));
                }
                for m in messages {
                    let id = st.take_next_id();
                    let otp = decoder::extract_otp(&m.text);
                    new_items.push(SmsItem {
                        id,
                        message: m,
                        otp,
                        is_new: false,
                    });
                }
                if !new_items.is_empty() {
                    let _ = app2.emit(
                        "messages:added",
                        &serde_json::json!({ "items": &new_items }),
                    );
                }
                st.messages.append(&mut new_items);

                st.scan_done += 1;

                if st.scan_done >= total {
                    st.scan_busy = false;
                    let ok_ports = total - st.failed_notes.len();
                    st.status_text =
                        scan_done_status(ok_ports, total, st.messages.len(), &st.failed_notes);
                    log::info!("Scan complete: {}", st.status_text);
                    let _ = app2.emit("scan:done", &serde_json::json!({}));
                } else {
                    st.status_text = scan_progress_status(
                        st.scan_done,
                        total,
                        st.messages.len(),
                        st.failed_notes.len(),
                    );
                    if ok && count > 0 {
                        log::info!(
                            "Scan {}/{}: {} -> {} msg(s)",
                            st.scan_done,
                            total,
                            port,
                            count
                        );
                    } else if ok {
                        log::debug!("Scan {}/{}: {} -> no messages", st.scan_done, total, port);
                    }
                }
                status_text = st.status_text.clone();
            }
            let _ = app2.emit("status:update", &serde_json::json!({ "text": status_text }));
        }));
    }
    for h in handles {
        let _ = h.join();
    }

    Ok("Scan started".into())
}

#[tauri::command]
pub fn get_sim_numbers(
    app: tauri::AppHandle,
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
    let app2 = app.clone();
    thread::spawn(move || {
        let total = ports.len();
        let found = Arc::new(AtomicUsize::new(0));
        let done = Arc::new(AtomicUsize::new(0));
        // Same bounded fan-out as scanning: cap simultaneous USSD probes so a
        // large bank of sticks doesn't flood the USB bridge with AT requests.
        const MAX_CONCURRENT_USSD: usize = 16;
        let work = Arc::new(Mutex::new(ports));
        let workers = total.min(MAX_CONCURRENT_USSD);
        let mut handles = Vec::with_capacity(workers);
        for _ in 0..workers {
            let work2 = Arc::clone(&work);
            let st2 = Arc::clone(&state_clone);
            let found2 = Arc::clone(&found);
            let done2 = Arc::clone(&done);
            let app3 = app2.clone();
            handles.push(thread::spawn(move || loop {
                let port = match work2.lock().unwrap().pop() {
                    Some(port) => port,
                    None => break,
                };
                let (num, err) = crate::core::modem::get_sim_number(&port);
                if let Some(ref n) = num {
                    found2.fetch_add(1, Ordering::Relaxed);
                    log::info!("USSD {} -> {}", port, n);
                    let ports_snapshot;
                    {
                        let mut st = st2.lock().unwrap();
                        // Key the cache by the stable path so ttyUSB renumbering
                        // doesn't drop/reassign the SIM number later.
                        let key = st
                            .ports
                            .iter()
                            .find(|p| p.name == *port)
                            .map(|p| p.path.clone())
                            .unwrap_or_else(|| port.clone());
                        st.settings.sim_numbers.insert(key, n.clone());
                        if let Some(p) = st.ports.iter_mut().find(|p| p.name == port) {
                            p.sim_number = n.clone();
                        }
                        ports_snapshot = st.ports.clone();
                    }
                    let _ = app3.emit(
                        "ports:updated",
                        &serde_json::json!({ "ports": ports_snapshot }),
                    );
                } else {
                    log::warn!(
                        "USSD {} -> no number ({})",
                        port,
                        err.as_deref().unwrap_or("unknown")
                    );
                }
                let d = done2.fetch_add(1, Ordering::Relaxed) + 1;
                let text;
                {
                    let mut st = st2.lock().unwrap();
                    st.status_text = format!(
                        "Requesting SIM numbers {}/{}  |  Found: {}",
                        d,
                        total,
                        found2.load(Ordering::Relaxed)
                    );
                    text = st.status_text.clone();
                }
                let _ = app3.emit("status:update", &serde_json::json!({ "text": text }));
            }));
        }
        for h in handles {
            let _ = h.join();
        }
        let f = found.load(Ordering::Relaxed);
        let text;
        {
            let mut st = state_clone.lock().unwrap();
            st.ussd_busy = false;
            st.settings.save_sim_numbers();
            st.settings.save();
            st.status_text = format!("SIM numbers updated. Found: {}/{}   (saved)", f, total);
            log::info!("USSD done: {}", st.status_text);
            text = st.status_text.clone();
        }
        let _ = app2.emit("status:update", &serde_json::json!({ "text": text }));
        let _ = app2.emit("ussd:done", &serde_json::json!({}));
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
        log::info!("Live starting on {} port(s)", ports.len());
    }
    let _ = app.emit("messages:reset", &serde_json::json!({}));
    let _ = app.emit(
        "status:update",
        &serde_json::json!({ "text": format!("Starting live on {} port(s)...", ports.len()) }),
    );
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
                let sender = move |evt: crate::core::live::LiveEvent| match evt {
                    crate::core::live::LiveEvent::Ready { port } => {
                        let mut st = st2.lock().unwrap();
                        if let Some(p) = st.ports.iter_mut().find(|p| p.name == port) {
                            p.live_ready = true;
                            p.live_error = None;
                        }
                        st.live_ports_ready.push(port.clone());
                        st.status_text =
                            format!("Live {} port(s) ready...", st.live_ports_ready.len());
                        drop(st);
                        log::info!("Live ready: {}", port);
                        let _ = app2.emit("sms:ready", &serde_json::json!({ "port": port }));
                    }
                    crate::core::live::LiveEvent::Batch { port: _port, items } => {
                        let n = items.len();
                        log::info!("{} -> initial batch {} msg(s)", _port, n);
                        let mut new_items: Vec<SmsItem> = Vec::new();
                        {
                            let mut st = st2.lock().unwrap();
                            for m in items {
                                let id = st.take_next_id();
                                let otp = crate::core::decoder::extract_otp(&m.text);
                                new_items.push(SmsItem {
                                    id,
                                    message: m,
                                    otp,
                                    is_new: false,
                                });
                            }
                            if !new_items.is_empty() {
                                let _ = app2.emit(
                                    "messages:added",
                                    &serde_json::json!({ "items": &new_items }),
                                );
                            }
                            st.messages.append(&mut new_items);
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
                        log::info!("NEW SMS on {}: from={:?} OTP={:?}", port, message.from, otp);
                        let item = SmsItem {
                            id,
                            message: message.clone(),
                            otp: otp.clone(),
                            is_new,
                        };
                        st.messages.push(item);
                        drop(st);
                        let _ = app2.emit(
                            "sms:new",
                            &serde_json::json!({
                                "id": id,
                                "message": message,
                                "otp": otp,
                                "port": port,
                                "is_new": is_new,
                            }),
                        );
                    }
                    crate::core::live::LiveEvent::Closed { port, error } => {
                        let (text, ports_snapshot);
                        {
                            let mut st = st2.lock().unwrap();
                            if let Some(e) = error {
                                log::warn!("Live {} closed: {}", port, e);
                                if let Some(p) = st.ports.iter_mut().find(|p| p.name == port) {
                                    p.live_ready = false;
                                    p.live_error = Some(e.clone());
                                }
                                st.live_failed.push((port.clone(), e.clone()));
                                st.status_text = format!("{} FAILED: {}", port, e);
                            }
                            text = st.status_text.clone();
                            ports_snapshot = st.ports.clone();
                        }
                        let _ = app2.emit("status:update", &serde_json::json!({ "text": text }));
                        let _ = app2.emit(
                            "ports:updated",
                            &serde_json::json!({ "ports": ports_snapshot }),
                        );
                    }
                };
                crate::core::live::run_live(port, stop, sender);
            }));
        }
        for h in handles {
            let _ = h.join();
        }
        let text;
        {
            let mut st = state_clone.lock().unwrap();
            st.live_on = false;
            st.live_stop = None;
            st.status_text = format!("Live stopped. Messages: {}", st.messages.len());
            text = st.status_text.clone();
        }
        let _ = app.emit("status:update", &serde_json::json!({ "text": text }));
    });
    Ok("Live started".into())
}

#[tauri::command]
pub fn stop_live(app: tauri::AppHandle, state: tauri::State<'_, SharedState>) {
    let mut st = state.lock().unwrap();
    if let Some(ref stop) = st.live_stop {
        stop.store(true, Ordering::Relaxed);
    }
    st.live_on = false;
    for p in &mut st.ports {
        p.live_ready = false;
        p.live_error = None;
    }
    st.status_text = "Stopping live...".into();
    log::info!("Live stop requested");
    let text = st.status_text.clone();
    let ports_snapshot = st.ports.clone();
    drop(st);
    let _ = app.emit("status:update", &serde_json::json!({ "text": text }));
    let _ = app.emit(
        "ports:updated",
        &serde_json::json!({ "ports": ports_snapshot }),
    );
}

#[tauri::command]
pub fn delete_selected(
    app: tauri::AppHandle,
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
                // Assembled long SMS span multiple SIM indices: remove them all.
                let mut idxs = item.message.part_indices.clone();
                if idxs.is_empty()
                    || idxs.len() == 1 && !item.message.part_indices.contains(&item.message.index)
                {
                    idxs = vec![item.message.index];
                }
                map.entry(item.message.port.clone())
                    .or_default()
                    .extend(idxs);
            }
        }
        to_delete = map
            .into_iter()
            .map(|(port, mut idxs)| {
                idxs.sort_unstable();
                idxs.dedup();
                (port, idxs)
            })
            .collect();
    }
    if to_delete.is_empty() {
        return Err("No messages selected.".into());
    }
    let initial_status;
    {
        let mut st = state.lock().unwrap();
        st.delete_busy = true;
        st.status_text = format!("Deleting {} message(s)...", ids.len());
        initial_status = st.status_text.clone();
    }
    let _ = app.emit(
        "status:update",
        &serde_json::json!({ "text": initial_status }),
    );
    let state_clone = Arc::clone(&state);
    let app2 = app.clone();
    thread::spawn(move || {
        let mut ok = 0usize;
        let mut fail = 0usize;
        for (port, indices) in &to_delete {
            let r = crate::core::modem::delete_messages(port, Some(indices.as_slice()));
            if r.ok {
                ok += 1;
                log::info!("Deleted {} msg(s) from {}", indices.len(), port);
            } else {
                fail += 1;
                log::warn!(
                    "Delete failed on {}: {}",
                    port,
                    r.error.as_deref().unwrap_or("unknown")
                );
            }
        }
        let text;
        {
            let mut st = state_clone.lock().unwrap();
            st.delete_busy = false;
            st.messages.retain(|m| !ids.contains(&m.id));
            st.status_text = format!("Deleted: {} ok, {} fail", ok, fail);
            log::info!("{}", st.status_text);
            text = st.status_text.clone();
        }
        let _ = app2.emit("messages:removed", &serde_json::json!({ "ids": &ids }));
        let _ = app2.emit("delete:done", &serde_json::json!({}));
        let _ = app2.emit("status:update", &serde_json::json!({ "text": text }));
    });
    Ok("Delete started".into())
}

#[tauri::command]
pub fn clear_all(app: tauri::AppHandle, state: tauri::State<'_, SharedState>) {
    let mut st = state.lock().unwrap();
    st.messages.clear();
    st.status_text = "Cleared".into();
    let text = st.status_text.clone();
    drop(st);
    let _ = app.emit("messages:reset", &serde_json::json!({}));
    let _ = app.emit("status:update", &serde_json::json!({ "text": text }));
}

/// Write exported SMS content to a user-chosen location.
///
/// The native save dialog runs on the Rust side and the file is written
/// directly with std::fs, which sidesteps the fs-plugin path ACL entirely —
/// the user's chosen path is always writable with no capability gymnastics.
/// Progress/outcome is reported back through the `export:saved` /
/// `export:failed` events so the UI can toast the real result.
#[tauri::command]
pub fn export_messages(app: tauri::AppHandle, contents: String, suggested: String) -> Result<(), String> {
    use tauri_plugin_dialog::DialogExt;
    let handle = app.clone();
    app.dialog()
        .file()
        .add_filter("SMS export", &["csv", "json"])
        .set_file_name(suggested)
        .save_file(move |file_path| {
            let Some(path) = file_path else {
                return; // user cancelled the dialog — stay silent
            };
            match path {
                tauri_plugin_dialog::FilePath::Path(pb) => {
                    match std::fs::write(&pb, &contents) {
                        Ok(_) => {
                            log::info!("Exported SMS to {}", pb.display());
                            let _ = handle.emit(
                                "export:saved",
                                &serde_json::json!({ "path": pb.display().to_string() }),
                            );
                        }
                        Err(e) => {
                            log::error!("Export write failed: {}", e);
                            let _ = handle.emit(
                                "export:failed",
                                &serde_json::json!({ "error": e.to_string() }),
                            );
                        }
                    }
                }
                _ => { /* remote URL path not applicable */ }
            }
        });
    Ok(())
}

#[tauri::command]
pub fn get_logs(limit: Option<usize>, min_level: Option<String>) -> Vec<crate::logging::LogEntry> {
    crate::logging::get_all_logs(limit, min_level)
}

#[tauri::command]
pub fn clear_logs() -> Result<(), String> {
    crate::logging::clear_log_buffer();
    Ok(())
}

#[tauri::command]
pub fn get_log_file_path() -> Result<String, String> {
    crate::logging::get_log_file_path()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "Could not determine log file path".into())
}

#[tauri::command]
pub fn open_log_folder() -> Result<(), String> {
    if let Some(path) = crate::logging::get_log_file_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("explorer").arg(parent).spawn();
            }
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("open").arg(parent).spawn();
            }
            #[cfg(target_os = "linux")]
            {
                let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
            }
            return Ok(());
        }
    }
    Err("Could not locate log folder".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_status_without_failures_has_no_suffix() {
        assert_eq!(
            scan_progress_status(3, 8, 12, 0),
            "Reading 3/8  |  Messages: 12"
        );
    }

    #[test]
    fn progress_status_with_failures_appends_failed_count() {
        assert_eq!(
            scan_progress_status(5, 8, 20, 2),
            "Reading 5/8  |  Messages: 20  |  FAILED: 2"
        );
    }

    #[test]
    fn done_status_lists_failed_ports() {
        let notes = vec!["COM3 (Cannot open port)".to_string()];
        assert_eq!(
            scan_done_status(7, 8, 30, &notes),
            "Done. Modems OK: 7/8  |  Total messages: 30  |  FAILED: COM3 (Cannot open port)"
        );
    }

    #[test]
    fn done_status_clean_when_no_failures() {
        assert_eq!(
            scan_done_status(8, 8, 0, &[],),
            "Done. Modems OK: 8/8  |  Total messages: 0"
        );
    }
}
