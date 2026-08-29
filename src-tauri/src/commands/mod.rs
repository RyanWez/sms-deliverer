use crate::core::decoder;
use crate::core::models::*;
use crate::core::sim_directory::SimDirectory;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;
use tauri::Emitter;

pub struct AppStateInner {
    pub sim_dir: SimDirectory,
    pub next_id: u64,
    pub ports: Vec<PortInfo>,
    pub messages: Vec<SmsItem>,
    pub scan_busy: bool,
    pub scan_done: usize,
    pub live_on: bool,
    pub live_ports_ready: Vec<String>,
    pub live_failed: Vec<(String, String)>,
    /// Ports live mode is holding open but where no modem answers. Tracked so
    /// the status line can say "7/64 ready | 57 no modem" instead of implying
    /// the whole bank is monitored.
    pub live_offline: Vec<String>,
    pub live_stop: Option<Arc<AtomicBool>>,
    pub ussd_busy: bool,
    pub delete_busy: bool,
    pub cleanup_busy: bool,
    pub detect_busy: bool,
    pub status_text: String,
    pub failed_notes: Vec<String>,
}

impl AppStateInner {
    pub fn take_next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// True while any operation owns the serial ports. Every port-touching
    /// command checks this, so the list must stay in one place.
    fn port_busy(&self) -> bool {
        self.scan_busy
            || self.live_on
            || self.ussd_busy
            || self.delete_busy
            || self.cleanup_busy
            || self.detect_busy
    }
}

pub type SharedState = Arc<Mutex<AppStateInner>>;

/// Lock the shared state, recovering from poisoning.
///
/// A worker that panics while holding this lock would otherwise poison it and
/// turn every later `lock().unwrap()` into a second panic — the app would look
/// permanently wedged. The state is a plain snapshot of ports/messages, so
/// continuing with the last-written value is strictly better than dying.
pub fn lock_state(state: &Mutex<AppStateInner>) -> MutexGuard<'_, AppStateInner> {
    state.lock().unwrap_or_else(|e| e.into_inner())
}

fn take_port<T>(queue: &Mutex<Vec<T>>) -> Option<T> {
    queue.lock().unwrap_or_else(|e| e.into_inner()).pop()
}

/// Cap concurrent port access to protect the USB bridge from contention. With
/// 60+ sticks, firing 64 simultaneous AT probes backs up the modem hardware and
/// the OS USB stack; a bounded worker pool preserves the shared-state/progress
/// accounting while limiting parallelism.
const MAX_CONCURRENT_PORTS: usize = 16;

/// Worker count for the liveness sweep. Higher than `MAX_CONCURRENT_PORTS`
/// because a probe is a single `AT` with a short timeout rather than a full
/// SMS conversation: it moves almost no bytes, so the USB bridge tolerates more
/// of them at once and the whole bank resolves in one pass instead of four.
const MAX_CONCURRENT_PROBES: usize = 32;

pub fn new_shared_state() -> SharedState {
    let sim_dir = SimDirectory::load();
    let ports: Vec<PortInfo> = crate::core::modem::get_port_names()
        .into_iter()
        .map(|name| {
            let path = crate::core::modem::stable_id(&name);
            let sim = sim_dir.number_of(&path, &name);
            PortInfo {
                name,
                path,
                checked: true,
                sim_number: sim,
                alive: None,
                live_ready: false,
                live_error: None,
            }
        })
        .collect();
    Arc::new(Mutex::new(AppStateInner {
        sim_dir,
        next_id: 1,
        ports,
        messages: Vec::new(),
        scan_busy: false,
        scan_done: 0,
        live_on: false,
        live_ports_ready: Vec::new(),
        live_failed: Vec::new(),
        live_offline: Vec::new(),
        live_stop: None,
        ussd_busy: false,
        delete_busy: false,
        cleanup_busy: false,
        detect_busy: false,
        status_text: String::new(),
        failed_notes: Vec::new(),
    }))
}

#[tauri::command]
pub fn refresh_ports(state: tauri::State<'_, SharedState>) -> Vec<PortInfo> {
    let names = crate::core::modem::get_port_names();
    let mut st = lock_state(&state);
    let old_map: std::collections::HashMap<String, PortInfo> =
        st.ports.drain(..).map(|p| (p.name.clone(), p)).collect();
    let ports: Vec<PortInfo> = names
        .into_iter()
        .map(|n| {
            if let Some(mut old) = old_map.get(&n).cloned() {
                old.path = crate::core::modem::stable_id(&n);
                old.sim_number = st.sim_dir.number_of(&old.path, &n);
                old.live_ready = false;
                old.live_error = None;
                old
            } else {
                let path = crate::core::modem::stable_id(&n);
                PortInfo {
                    name: n.clone(),
                    path: path.clone(),
                    checked: true,
                    sim_number: st.sim_dir.number_of(&path, &n),
                    alive: None,
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
    let st = lock_state(&state);
    st.ports
        .iter()
        .filter(|p| p.checked)
        .map(|p| p.name.clone())
        .collect()
}

#[tauri::command]
pub fn toggle_port_checked(state: tauri::State<'_, SharedState>, port: String, checked: bool) {
    let mut st = lock_state(&state);
    if let Some(p) = st.ports.iter_mut().find(|p| p.name == port) {
        p.checked = checked;
    }
}

#[tauri::command]
pub fn set_all_ports_checked(state: tauri::State<'_, SharedState>, checked: bool) {
    let mut st = lock_state(&state);
    for p in &mut st.ports {
        p.checked = checked;
    }
}

/// Probe every port once and record which ones actually have a modem behind
/// them, then leave only those selected.
///
/// This exists because a SIM bank publishes one tty per channel regardless of
/// whether a SIM is inserted, so port *count* says nothing about how many
/// modems are reachable. Running this first turns every later operation from
/// "64 ports × full timeout chain" into "7 ports that answer instantly".
#[tauri::command]
pub fn detect_ports(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
) -> Result<String, String> {
    let names: Vec<String> = {
        let st = lock_state(&state);
        if st.port_busy() {
            return Err("Busy".into());
        }
        st.ports.iter().map(|p| p.name.clone()).collect()
    };
    if names.is_empty() {
        return Err("No serial ports found.".into());
    }
    let total = names.len();
    {
        let mut st = lock_state(&state);
        st.detect_busy = true;
        st.status_text = format!("Detecting modems on {} port(s)...", total);
    }
    let _ = app.emit(
        "status:update",
        &serde_json::json!({ "text": format!("Detecting modems on {} port(s)...", total) }),
    );
    log::info!("Detect started on {} port(s)", total);

    let state_clone = Arc::clone(&state);
    let app2 = app.clone();
    thread::spawn(move || {
        let work = Arc::new(Mutex::new(names));
        let done = Arc::new(AtomicUsize::new(0));
        let alive_count = Arc::new(AtomicUsize::new(0));
        let workers = total.min(MAX_CONCURRENT_PROBES);
        let mut handles = Vec::with_capacity(workers);
        for _ in 0..workers {
            let work2 = Arc::clone(&work);
            let st2 = Arc::clone(&state_clone);
            let app3 = app2.clone();
            let done2 = Arc::clone(&done);
            let alive2 = Arc::clone(&alive_count);
            handles.push(thread::spawn(move || {
                while let Some(port) = take_port(&work2) {
                    let probed = catch_unwind(AssertUnwindSafe(|| {
                        crate::core::modem::probe_port(&port)
                    }));
                    // An unopenable port is not the same as a silent one, but
                    // either way there is nothing to talk to right now.
                    let alive = matches!(probed, Ok(Ok(true)));
                    if alive {
                        alive2.fetch_add(1, Ordering::Relaxed);
                    }
                    let n = done2.fetch_add(1, Ordering::Relaxed) + 1;
                    let text;
                    {
                        let mut st = lock_state(&st2);
                        if let Some(p) = st.ports.iter_mut().find(|p| p.name == port) {
                            p.alive = Some(alive);
                            // Leaving dead ports selected is what made every
                            // later action pay their timeouts, so the sweep
                            // owns the selection: alive ports on, others off.
                            p.checked = alive;
                            p.live_error = None;
                            p.live_ready = false;
                        }
                        st.status_text = format!(
                            "Detecting {}/{}  |  Modems found: {}",
                            n,
                            total,
                            alive2.load(Ordering::Relaxed)
                        );
                        text = st.status_text.clone();
                    }
                    let _ = app3.emit("status:update", &serde_json::json!({ "text": text }));
                }
            }));
        }
        for h in handles {
            let _ = h.join();
        }
        let found = alive_count.load(Ordering::Relaxed);
        let (text, ports_snapshot);
        {
            let mut st = lock_state(&state_clone);
            st.detect_busy = false;
            st.status_text = format!(
                "Detect done. Modems found: {}/{}  |  {} port(s) with no modem deselected",
                found,
                total,
                total - found
            );
            text = st.status_text.clone();
            ports_snapshot = st.ports.clone();
        }
        log::info!("{}", text);
        let _ = app2.emit("status:update", &serde_json::json!({ "text": text }));
        let _ = app2.emit("ports:updated", &serde_json::json!({ "ports": ports_snapshot }));
        let _ = app2.emit(
            "detect:done",
            &serde_json::json!({ "found": found, "total": total }),
        );
    });

    Ok("Detect started".into())
}

/// Status line for live mode. Reports ready, offline and still-connecting
/// counts separately so a bank of mostly-empty slots reads honestly instead of
/// looking like every port is being monitored.
fn live_status(st: &AppStateInner, total: usize) -> String {
    let ready = st.live_ports_ready.len();
    let offline = st.live_offline.len();
    let mut s = format!("Live {}/{} ready", ready, total);
    if offline > 0 {
        s.push_str(&format!("  |  {} no modem", offline));
    }
    let pending = total.saturating_sub(ready + offline);
    if pending > 0 {
        s.push_str(&format!("  |  {} connecting…", pending));
    }
    s
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
        let mut st = lock_state(&state);
        if st.port_busy() {
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

    // Spawn-and-return, matching every other long-running command. The scan
    // used to join its workers inline, so the IPC call blocked for the whole
    // sweep — minutes on a full bank — and only then answered "Scan started".
    let state_clone = Arc::clone(&state);
    let app2 = app.clone();
    thread::spawn(move || {
        let work = Arc::new(Mutex::new(ports));
        let workers = total.min(MAX_CONCURRENT_PORTS);
        let mut handles = Vec::with_capacity(workers);
        for _ in 0..workers {
            let work2 = Arc::clone(&work);
            let st2 = Arc::clone(&state_clone);
            let app3 = app2.clone();
            handles.push(thread::spawn(move || {
                while let Some(port) = take_port(&work2) {
                    // Without this guard a single panic (malformed PDU, a bad
                    // index) killed the worker with its share of the queue
                    // unread, so scan_done never reached total and scan_busy
                    // stayed set — the app read "Busy" until restart.
                    let outcome =
                        catch_unwind(AssertUnwindSafe(|| scan_one_port(&port, &st2, &app3, total)));
                    if outcome.is_err() {
                        log::error!("Scan worker panicked on {} — continuing", port);
                        let mut st = lock_state(&st2);
                        st.failed_notes.push(format!("{} (worker panicked)", port));
                        st.scan_done += 1;
                    }
                }
            }));
        }
        for h in handles {
            let _ = h.join();
        }
        // The supervisor owns the exit from "busy", so it happens even when
        // every worker died mid-flight.
        let (text, ports_snapshot);
        {
            let mut st = lock_state(&state_clone);
            st.scan_busy = false;
            let ok_ports = total.saturating_sub(st.failed_notes.len());
            st.status_text = scan_done_status(ok_ports, total, st.messages.len(), &st.failed_notes);
            text = st.status_text.clone();
            ports_snapshot = st.ports.clone();
        }
        log::info!("Scan complete: {}", text);
        let _ = app2.emit("status:update", &serde_json::json!({ "text": text }));
        // Push the liveness verdicts the sweep collected so the Ports page
        // reflects which slots are populated.
        let _ = app2.emit("ports:updated", &serde_json::json!({ "ports": ports_snapshot }));
        let _ = app2.emit("scan:done", &serde_json::json!({}));
    });

    Ok("Scan started".into())
}

/// Read one port and fold the result into shared state and progress events.
fn scan_one_port(port: &str, state: &SharedState, app: &tauri::AppHandle, total: usize) {
    let crate::core::modem::ReadResult { ok, messages, error } =
        crate::core::modem::read_port(port);
    let count = messages.len();
    // A scan is itself a liveness observation, so record it — the port list
    // learns which slots are populated without a separate Detect pass. Only the
    // probe's own verdict marks a port dead: a modem that answered `AT` and then
    // failed mid-read is present but wedged, which is a different problem and
    // must not be labelled "no modem".
    let observed_alive = if ok {
        Some(true)
    } else if error.as_deref() == Some(crate::core::modem::NOT_RESPONDING) {
        Some(false)
    } else {
        None
    };

    let (added, status_text);
    {
        let mut st = lock_state(state);
        if let Some(alive) = observed_alive {
            if let Some(p) = st.ports.iter_mut().find(|p| p.name == port) {
                p.alive = Some(alive);
            }
        }
        if !ok {
            let err = error.unwrap_or_default();
            log::warn!("Scan {}: FAILED ({})", port, err);
            st.failed_notes.push(format!("{} ({})", port, err));
        }
        let mut new_items: Vec<SmsItem> = Vec::new();
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
        added = (!new_items.is_empty()).then(|| serde_json::json!({ "items": &new_items }));
        st.messages.append(&mut new_items);
        st.scan_done += 1;
        st.status_text =
            scan_progress_status(st.scan_done, total, st.messages.len(), st.failed_notes.len());
        status_text = st.status_text.clone();
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
    // Emitting outside the lock: a slow/blocked webview must not hold the state
    // mutex the other workers are queueing on.
    if let Some(payload) = added {
        let _ = app.emit("messages:added", &payload);
    }
    let _ = app.emit("status:update", &serde_json::json!({ "text": status_text }));
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
        let mut st = lock_state(&state);
        if st.port_busy() {
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
        let work = Arc::new(Mutex::new(ports));
        let workers = total.min(MAX_CONCURRENT_PORTS);
        let mut handles = Vec::with_capacity(workers);
        for _ in 0..workers {
            let work2 = Arc::clone(&work);
            let st2 = Arc::clone(&state_clone);
            let found2 = Arc::clone(&found);
            let done2 = Arc::clone(&done);
            let app3 = app2.clone();
            handles.push(thread::spawn(move || {
                while let Some(port) = take_port(&work2) {
                    let outcome = catch_unwind(AssertUnwindSafe(|| {
                        ussd_one_port(&port, &st2, &app3, &found2, &done2, total)
                    }));
                    if outcome.is_err() {
                        log::error!("USSD worker panicked on {} — continuing", port);
                        done2.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }));
        }
        for h in handles {
            let _ = h.join();
        }
        let f = found.load(Ordering::Relaxed);
        let text;
        {
            let mut st = lock_state(&state_clone);
            st.ussd_busy = false;
            st.sim_dir.save();
            st.status_text = format!("SIM numbers updated. Found: {}/{}   (saved)", f, total);
            log::info!("USSD done: {}", st.status_text);
            text = st.status_text.clone();
        }
        let _ = app2.emit("status:update", &serde_json::json!({ "text": text }));
        let _ = app2.emit("ussd:done", &serde_json::json!({}));
    });
    Ok("USSD started".into())
}

/// Query one port's own number over USSD and record it against its stable path.
fn ussd_one_port(
    port: &str,
    state: &SharedState,
    app: &tauri::AppHandle,
    found: &AtomicUsize,
    done: &AtomicUsize,
    total: usize,
) {
    let (num, err) = crate::core::modem::get_sim_number(port);
    // Same liveness bookkeeping as the scan: only the probe's own verdict marks
    // a port dead, so a registered modem that simply had no USSD answer keeps
    // its "alive" status.
    let observed_alive = if num.is_some() {
        Some(true)
    } else if err.as_deref() == Some(crate::core::modem::NOT_RESPONDING) {
        Some(false)
    } else {
        None
    };
    if let Some(alive) = observed_alive {
        let mut st = lock_state(state);
        if let Some(p) = st.ports.iter_mut().find(|p| p.name == port) {
            p.alive = Some(alive);
        }
    }
    if let Some(ref n) = num {
        found.fetch_add(1, Ordering::Relaxed);
        log::info!("USSD {} -> {}", port, n);
        let ports_snapshot;
        {
            let mut st = lock_state(state);
            // Key the cache by the stable path so ttyUSB renumbering doesn't
            // drop or reassign the SIM number later.
            let key = st
                .ports
                .iter()
                .find(|p| p.name == port)
                .map(|p| p.path.clone())
                .unwrap_or_else(|| port.to_string());
            st.sim_dir.numbers.insert(key, n.clone());
            if let Some(p) = st.ports.iter_mut().find(|p| p.name == port) {
                p.sim_number = n.clone();
            }
            ports_snapshot = st.ports.clone();
        }
        let _ = app.emit(
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
    let d = done.fetch_add(1, Ordering::Relaxed) + 1;
    let (text, ports_snapshot);
    {
        let mut st = lock_state(state);
        st.status_text = format!(
            "Requesting SIM numbers {}/{}  |  Found: {}",
            d,
            total,
            found.load(Ordering::Relaxed)
        );
        text = st.status_text.clone();
        ports_snapshot = st.ports.clone();
    }
    let _ = app.emit("status:update", &serde_json::json!({ "text": text }));
    let _ = app.emit(
        "ports:updated",
        &serde_json::json!({ "ports": ports_snapshot }),
    );
}

#[tauri::command]
pub fn get_messages(state: tauri::State<'_, SharedState>) -> Vec<SmsItem> {
    lock_state(&state).messages.clone()
}

#[tauri::command]
pub fn get_ports(state: tauri::State<'_, SharedState>) -> Vec<PortInfo> {
    lock_state(&state).ports.clone()
}

#[tauri::command]
pub fn get_status_text(state: tauri::State<'_, SharedState>) -> String {
    lock_state(&state).status_text.clone()
}

/// Convert the UI's retention setting into a duration, or `None` when the
/// operator turned auto-cleanup off (0 or a nonsensical value).
fn retention_from_hours(hours: Option<f64>) -> Option<Duration> {
    let h = hours?;
    if !h.is_finite() || h <= 0.0 {
        return None;
    }
    Some(Duration::from_secs_f64(h * 3600.0))
}

#[tauri::command]
pub fn start_live(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    retention_hours: Option<f64>,
) -> Result<String, String> {
    let retention = retention_from_hours(retention_hours);
    let ports = {
        let mut st = lock_state(&state);
        if st.live_stop.is_some() || st.port_busy() {
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
    let shared_stop = Arc::new(AtomicBool::new(false));
    {
        let mut st = lock_state(&state);
        st.live_on = true;
        st.live_ports_ready.clear();
        st.live_failed.clear();
        st.live_offline.clear();
        st.messages.clear();
        st.live_stop = Some(Arc::clone(&shared_stop));
        st.status_text = format!("Starting live on {} port(s)...", ports.len());
        log::info!(
            "Live starting on {} port(s) (SIM retention: {})",
            ports.len(),
            match retention {
                Some(d) => format!("{}h", d.as_secs() / 3600),
                None => "off".into(),
            }
        );
    }
    let _ = app.emit("messages:reset", &serde_json::json!({}));
    let _ = app.emit(
        "status:update",
        &serde_json::json!({ "text": format!("Starting live on {} port(s)...", ports.len()) }),
    );
    let state_clone = Arc::clone(&state);
    thread::spawn(move || {
        let port_count = ports.len();
        let mut handles = Vec::with_capacity(port_count);
        for port in ports {
            let stop = Arc::clone(&shared_stop);
            let st2 = Arc::clone(&state_clone);
            let app2 = app.clone();
            handles.push(thread::spawn(move || {
                let sender = move |evt: crate::core::live::LiveEvent| match evt {
                    crate::core::live::LiveEvent::Ready { port } => {
                        let (text, ports_snapshot);
                        {
                            let mut st = lock_state(&st2);
                            if let Some(p) = st.ports.iter_mut().find(|p| p.name == port) {
                                p.alive = Some(true);
                                p.live_ready = true;
                                p.live_error = None;
                            }
                            // Reconnects re-emit Ready for a port already counted —
                            // keep the ready list deduplicated or the "Live x/y"
                            // badge overcounts.
                            if !st.live_ports_ready.contains(&port) {
                                st.live_ports_ready.push(port.clone());
                            }
                            st.live_offline.retain(|p| p != &port);
                            st.status_text = live_status(&st, port_count);
                            text = st.status_text.clone();
                            ports_snapshot = st.ports.clone();
                        }
                        log::info!("Live ready: {}", port);
                        let _ = app2.emit("status:update", &serde_json::json!({ "text": text }));
                        let _ = app2
                            .emit("ports:updated", &serde_json::json!({ "ports": ports_snapshot }));
                        let _ = app2.emit("sms:ready", &serde_json::json!({ "port": port }));
                    }
                    crate::core::live::LiveEvent::Offline { port, error } => {
                        let (text, ports_snapshot);
                        {
                            let mut st = lock_state(&st2);
                            if let Some(p) = st.ports.iter_mut().find(|p| p.name == port) {
                                p.alive = Some(false);
                                p.live_ready = false;
                                p.live_error = Some(error.clone());
                            }
                            st.live_ports_ready.retain(|p| p != &port);
                            if !st.live_offline.contains(&port) {
                                st.live_offline.push(port.clone());
                            }
                            st.status_text = live_status(&st, port_count);
                            text = st.status_text.clone();
                            ports_snapshot = st.ports.clone();
                        }
                        let _ = app2.emit("status:update", &serde_json::json!({ "text": text }));
                        let _ = app2
                            .emit("ports:updated", &serde_json::json!({ "ports": ports_snapshot }));
                        let _ = app2.emit(
                            "live:offline",
                            &serde_json::json!({ "port": port, "error": error }),
                        );
                    }
                    crate::core::live::LiveEvent::Reconnecting { port, error } => {
                        let (text, ports_snapshot);
                        {
                            let mut st = lock_state(&st2);
                            log::warn!("Live {} reconnecting: {}", port, error);
                            if let Some(p) = st.ports.iter_mut().find(|p| p.name == port) {
                                p.live_ready = false;
                                p.live_error = Some(format!("Reconnecting: {}", error));
                            }
                            st.status_text = format!("{} reconnecting…", port);
                            text = st.status_text.clone();
                            ports_snapshot = st.ports.clone();
                        }
                        let _ = app2.emit("status:update", &serde_json::json!({ "text": text }));
                        let _ = app2.emit("ports:updated", &serde_json::json!({ "ports": ports_snapshot }));
                        let _ = app2.emit(
                            "live:reconnecting",
                            &serde_json::json!({ "port": port, "error": error }),
                        );
                    }
                    crate::core::live::LiveEvent::Batch { port: _port, items, is_new } => {
                        let n = items.len();
                        log::info!("{} -> initial batch {} msg(s)", _port, n);
                        let mut new_items: Vec<SmsItem> = Vec::new();
                        {
                            let mut st = lock_state(&st2);
                            for m in items {
                                let id = st.take_next_id();
                                let otp = crate::core::decoder::extract_otp(&m.text);
                                new_items.push(SmsItem {
                                    id,
                                    message: m,
                                    otp,
                                    is_new,
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
                        let mut st = lock_state(&st2);
                        let otp = crate::core::decoder::extract_otp(&message.text);
                        log::info!("NEW SMS on {}: from={:?} OTP={:?}", port, message.from, otp);

                        // A completed concatenated message often supersedes a
                        // partial fragment row already shown from the initial
                        // backfill (same sender + same receive timestamp, and
                        // the partial's text is a prefix of the full text).
                        // Swap that row in place instead of appending a
                        // duplicate — this is what makes long messages appear
                        // whole instead of "half + full" pairs.
                        let existing = st
                            .messages
                            .iter()
                            .position(|it| {
                                it.message.port == message.port
                                    && it.message.from == message.from
                                    && it.message.received == message.received
                                    && message.text.starts_with(&it.message.text)
                            })
                            .map(|idx| st.messages[idx].id);
                        let item = SmsItem {
                            id: existing.unwrap_or_else(|| st.take_next_id()),
                            message: message.clone(),
                            otp: otp.clone(),
                            is_new,
                        };
                        match existing {
                            Some(_) => {
                                if let Some(idx) = st
                                    .messages
                                    .iter()
                                    .position(|it| it.id == item.id)
                                {
                                    st.messages[idx] = item.clone();
                                }
                                drop(st);
                                let _ = app2.emit(
                                    "messages:updated",
                                    &serde_json::json!({ "item": &item }),
                                );
                            }
                            None => {
                                st.messages.push(item.clone());
                                drop(st);
                                let _ = app2.emit(
                                    "sms:new",
                                    &serde_json::json!({
                                        "id": item.id,
                                        "message": message,
                                        "otp": otp,
                                        "port": port,
                                        "is_new": is_new,
                                    }),
                                );
                            }
                        }
                    }
                    crate::core::live::LiveEvent::Closed { port, error } => {
                        let (text, ports_snapshot);
                        {
                            let mut st = lock_state(&st2);
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
                crate::core::live::run_live(port, stop, retention, sender);
            }));
        }
        for h in handles {
            let _ = h.join();
        }
        let text;
        {
            let mut st = lock_state(&state_clone);
            st.live_on = false;
            st.live_stop = None;
            st.status_text = format!("Live stopped. Messages: {}", st.messages.len());
            text = st.status_text.clone();
        }
        let _ = app.emit("status:update", &serde_json::json!({ "text": text }));
        // Explicit end-of-live signal: the frontend's "Live" badge is driven by
        // its own optimistic state, so it needs this to learn the backend side
        // actually wound down (all workers exited).
        let _ = app.emit("live:stopped", &serde_json::json!({}));
    });
    Ok("Live started".into())
}

#[tauri::command]
pub fn stop_live(app: tauri::AppHandle, state: tauri::State<'_, SharedState>) {
    let mut st = lock_state(&state);
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
        let st = lock_state(&state);
        if st.port_busy() {
            return Err("Busy".into());
        }
    }
    let to_delete: Vec<(String, Vec<i32>)>;
    {
        let st = lock_state(&state);
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
        let mut st = lock_state(&state);
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
        let mut fail = 0usize;
        let mut wanted = 0usize;
        let mut gone = 0usize;
        // A panic mid-delete would otherwise leave delete_busy set and the
        // toolbar spinner stuck; the flag is cleared below either way.
        let counted = catch_unwind(AssertUnwindSafe(|| {
            for (port, indices) in &to_delete {
                let r = crate::core::modem::delete_messages(port, Some(indices.as_slice()));
                wanted += indices.len();
                gone += r.deleted;
                if r.ok {
                    log::info!("Deleted {} msg(s) from {}", r.deleted, port);
                } else {
                    fail += 1;
                    log::warn!(
                        "Delete failed on {}: {}",
                        port,
                        r.error.as_deref().unwrap_or("unknown")
                    );
                }
            }
        }));
        if counted.is_err() {
            log::error!("Delete worker panicked");
        }
        let text;
        {
            let mut st = lock_state(&state_clone);
            st.delete_busy = false;
            st.messages.retain(|m| !ids.contains(&m.id));
            // Count SIM slots, not ports. A port can confirm some of its indices
            // and refuse the rest, and "1 ok" would hide that the messages are
            // still occupying SIM storage.
            st.status_text = if fail == 0 && gone == wanted {
                format!("Deleted {} message(s) from SIM", gone)
            } else {
                format!(
                    "Deleted {}/{} from SIM  |  FAILED: {} port(s)",
                    gone, wanted, fail
                )
            };
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
    let mut st = lock_state(&state);
    st.messages.clear();
    st.status_text = "Cleared".into();
    let text = st.status_text.clone();
    drop(st);
    let _ = app.emit("messages:reset", &serde_json::json!({}));
    let _ = app.emit("status:update", &serde_json::json!({ "text": text }));
}

/// Drop messages from the in-app inbox once they pass the retention period.
/// Uses the same expiry rule as SIM cleanup, so the list and the SIM agree on
/// what "expired" means — including never expiring an undated message.
#[tauri::command]
pub fn purge_expired_messages(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    max_age_hours: f64,
) -> Result<usize, String> {
    let Some(retention) = retention_from_hours(Some(max_age_hours)) else {
        return Ok(0);
    };
    let cutoff = retention_cutoff_ms(retention);
    let purged_ids: Vec<u64>;
    {
        let mut st = lock_state(&state);
        purged_ids = st
            .messages
            .iter()
            .filter(|it| is_expired(&it.message, cutoff))
            .map(|it| it.id)
            .collect();
        if !purged_ids.is_empty() {
            st.messages.retain(|it| !is_expired(&it.message, cutoff));
        }
    }
    if !purged_ids.is_empty() {
        let _ = app.emit("messages:removed", &serde_json::json!({ "ids": &purged_ids }));
        log::info!(
            "Auto-purged {} expired message(s) older than {} hour(s)",
            purged_ids.len(),
            max_age_hours
        );
    }
    Ok(purged_ids.len())
}

/// Delete expired messages from SIM storage on every selected port.
///
/// SIM memory holds only ~20–50 messages and nothing ever pruned it, so a bank
/// left running eventually filled up and the modems started silently rejecting
/// new SMS. This is the idle half of the fix: it opens each port itself, so it
/// refuses to run while scan/live/USSD/delete owns them — live mode prunes on
/// its own already-open channel instead.
#[tauri::command]
pub fn cleanup_sim_storage(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    retention_hours: Option<f64>,
) -> Result<String, String> {
    let Some(retention) = retention_from_hours(retention_hours) else {
        return Err("Retention is off.".into());
    };
    let ports = checked_ports(state.clone());
    if ports.is_empty() {
        return Err("No COM port selected.".into());
    }
    {
        let mut st = lock_state(&state);
        if st.port_busy() {
            return Err("Busy".into());
        }
        st.cleanup_busy = true;
    }
    let total = ports.len();
    let cutoff = retention_cutoff_ms(retention);
    log::info!(
        "SIM cleanup starting on {} port(s), retention {}h",
        total,
        retention.as_secs() / 3600
    );

    let state_clone = Arc::clone(&state);
    let app2 = app.clone();
    thread::spawn(move || {
        let deleted = Arc::new(AtomicUsize::new(0));
        let failed = Arc::new(AtomicUsize::new(0));
        let work = Arc::new(Mutex::new(ports));
        let workers = total.min(MAX_CONCURRENT_PORTS);
        let mut handles = Vec::with_capacity(workers);
        for _ in 0..workers {
            let work2 = Arc::clone(&work);
            let deleted2 = Arc::clone(&deleted);
            let failed2 = Arc::clone(&failed);
            handles.push(thread::spawn(move || {
                while let Some(port) = take_port(&work2) {
                    let outcome = catch_unwind(AssertUnwindSafe(|| {
                        crate::core::modem::expire_old(&port, cutoff)
                    }));
                    match outcome {
                        Ok(r) if r.ok => {
                            if r.deleted > 0 {
                                log::info!("SIM cleanup {}: deleted {} message(s)", port, r.deleted);
                            }
                            deleted2.fetch_add(r.deleted, Ordering::Relaxed);
                        }
                        Ok(r) => {
                            failed2.fetch_add(1, Ordering::Relaxed);
                            log::warn!(
                                "SIM cleanup {} failed: {}",
                                port,
                                r.error.as_deref().unwrap_or("unknown")
                            );
                        }
                        Err(_) => {
                            failed2.fetch_add(1, Ordering::Relaxed);
                            log::error!("SIM cleanup worker panicked on {} — continuing", port);
                        }
                    }
                }
            }));
        }
        for h in handles {
            let _ = h.join();
        }
        let d = deleted.load(Ordering::Relaxed);
        let f = failed.load(Ordering::Relaxed);
        let text;
        {
            // Supervisor clears the flag first, so a failure anywhere above
            // still leaves the app usable.
            let mut st = lock_state(&state_clone);
            st.cleanup_busy = false;
            st.status_text = if f == 0 {
                format!("SIM cleanup done. Deleted {} expired message(s)", d)
            } else {
                format!("SIM cleanup done. Deleted {}  |  FAILED: {}/{}", d, f, total)
            };
            text = st.status_text.clone();
        }
        log::info!("{}", text);
        let _ = app2.emit("status:update", &serde_json::json!({ "text": text }));
        let _ = app2.emit(
            "sim_cleanup:done",
            &serde_json::json!({ "deleted": d, "failed": f }),
        );
    });
    Ok("SIM cleanup started".into())
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

    #[test]
    fn retention_of_zero_or_less_means_cleanup_is_off() {
        assert!(retention_from_hours(None).is_none());
        assert!(retention_from_hours(Some(0.0)).is_none());
        assert!(retention_from_hours(Some(-4.0)).is_none());
        assert!(retention_from_hours(Some(f64::NAN)).is_none());
        assert!(retention_from_hours(Some(f64::INFINITY)).is_none());
    }

    #[test]
    fn retention_hours_convert_to_a_duration() {
        assert_eq!(
            retention_from_hours(Some(2.0)),
            Some(Duration::from_secs(7200))
        );
        assert_eq!(
            retention_from_hours(Some(0.5)),
            Some(Duration::from_secs(1800))
        );
    }

    #[test]
    fn port_busy_covers_every_operation_that_owns_a_port() {
        let idle = || AppStateInner {
            sim_dir: SimDirectory::default(),
            next_id: 1,
            ports: Vec::new(),
            messages: Vec::new(),
            scan_busy: false,
            scan_done: 0,
            live_on: false,
            live_ports_ready: Vec::new(),
            live_failed: Vec::new(),
            live_offline: Vec::new(),
            live_stop: None,
            ussd_busy: false,
            delete_busy: false,
            cleanup_busy: false,
            detect_busy: false,
            status_text: String::new(),
            failed_notes: Vec::new(),
        };
        assert!(!idle().port_busy());

        let setters: [fn(&mut AppStateInner); 6] = [
            |st| st.scan_busy = true,
            |st| st.live_on = true,
            |st| st.ussd_busy = true,
            |st| st.delete_busy = true,
            |st| st.cleanup_busy = true,
            |st| st.detect_busy = true,
        ];
        for set in setters {
            let mut st = idle();
            set(&mut st);
            assert!(st.port_busy());
        }
    }

    fn live_state(ready: usize, offline: usize) -> AppStateInner {
        let names = |prefix: &str, n: usize| (0..n).map(|i| format!("{prefix}{i}")).collect();
        AppStateInner {
            sim_dir: SimDirectory::default(),
            next_id: 1,
            ports: Vec::new(),
            messages: Vec::new(),
            scan_busy: false,
            scan_done: 0,
            live_on: true,
            live_ports_ready: names("ready", ready),
            live_failed: Vec::new(),
            live_offline: names("dead", offline),
            live_stop: None,
            ussd_busy: false,
            delete_busy: false,
            cleanup_busy: false,
            detect_busy: false,
            status_text: String::new(),
            failed_notes: Vec::new(),
        }
    }

    #[test]
    fn live_status_separates_ready_from_ports_with_no_modem() {
        // The shape that used to read "Live 64 port(s) ready..." on a bank
        // holding 7 SIMs.
        assert_eq!(
            live_status(&live_state(7, 57), 64),
            "Live 7/64 ready  |  57 no modem"
        );
    }

    #[test]
    fn live_status_counts_ports_still_connecting() {
        assert_eq!(
            live_status(&live_state(2, 1), 8),
            "Live 2/8 ready  |  1 no modem  |  5 connecting…"
        );
    }

    #[test]
    fn live_status_is_clean_when_every_port_is_ready() {
        assert_eq!(live_status(&live_state(3, 0), 3), "Live 3/3 ready");
    }
}
