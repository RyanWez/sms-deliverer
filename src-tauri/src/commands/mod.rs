use crate::core::decoder;
use crate::core::models::*;
use crate::core::sim_directory::SimDirectory;
use std::collections::{HashMap, HashSet};
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
    ///
    /// `live_stop` is part of the answer even though `live_on` is already here:
    /// `stop_live` clears `live_on` the instant the user asks, but the live
    /// workers keep their ports open until the supervisor has joined them — and a
    /// worker parked in `AT+CMGL=4` holds its port for up to 15 s more. Reporting
    /// idle during that window let scan / SIM numbers / delete / SIM cleanup
    /// through the gate (including the unattended 10-minute cleanup timer), and
    /// every port came back "device or resource busy". The flag means "the ports
    /// are held", not "an operation is nominally running".
    fn port_busy(&self) -> bool {
        self.scan_busy
            || self.live_on
            || self.live_stop.is_some()
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

/// Releases a busy flag when the thread that owns an operation goes away —
/// including by unwinding.
///
/// Every flag in `port_busy()` used to be released by an ordinary statement on
/// the happy path, so a panic anywhere outside the `catch_unwind` that wraps the
/// per-port work left it set for the rest of the process. `delete_selected` was
/// the sharpest case: its `catch_unwind` covers only the modem loop, so a panic
/// in the bookkeeping after it wedged `delete_busy` and with it every
/// port-touching command until restart. `start_live` was worse — its supervisor
/// thread had no `catch_unwind` at all, and it owns both `live_on` and
/// `live_stop`.
///
/// The happy path is deliberately unchanged. Each command still clears its own
/// flag inline, because several of them do more work under that same lock
/// (`sim_dir.save()`, building the status line) and reordering that is a
/// behaviour change this guard has no business making. `clear` is therefore
/// written to be idempotent and in practice the guard only ever fires on an
/// unwind. `lock_state` recovers from poisoning, so it can still take the lock
/// after a panic.
///
/// Construct it in the command and `move` it into the spawned closure: if
/// `thread::spawn` itself panics, the closure — and with it the guard — is
/// dropped during that unwind and the flag is still released.
struct BusyGuard {
    state: SharedState,
    clear: fn(&mut AppStateInner),
}

impl BusyGuard {
    fn new(state: &SharedState, clear: fn(&mut AppStateInner)) -> Self {
        BusyGuard {
            state: Arc::clone(state),
            clear,
        }
    }
}

impl Drop for BusyGuard {
    fn drop(&mut self) {
        let mut st = lock_state(&self.state);
        (self.clear)(&mut st);
    }
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
            // Nothing has been probed yet, so the number shown is the one filed
            // against whichever card was last seen in this slot. Detect Modems
            // confirms or clears it.
            let iccid = sim_dir.iccid_of(&path);
            let sim = sim_dir.number_of(&path, iccid.as_deref());
            PortInfo {
                name,
                path,
                checked: true,
                sim_number: sim,
                iccid,
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

/// Rebuild the port list from a fresh enumeration, carrying session state over
/// from `old`.
///
/// `enumerated` is `(tty name, stable path)` per port, resolved by the caller so
/// this can be tested without a real `/dev/serial/by-path`. `live_session` says
/// whether live mode still has workers running — the same "the ports are held"
/// window `port_busy()` covers, not just `live_on`.
fn merge_ports(
    enumerated: Vec<(String, String)>,
    old: Vec<PortInfo>,
    sim_dir: &SimDirectory,
    live_session: bool,
) -> Vec<PortInfo> {
    // Carry state over by stable path, never by tty name: the name is precisely
    // what a replug reshuffles, so matching on it moves one stick's liveness and
    // card onto a different stick.
    let old_map: HashMap<String, PortInfo> =
        old.into_iter().map(|p| (p.path.clone(), p)).collect();
    enumerated
        .into_iter()
        .map(|(name, path)| {
            let old = old_map.get(&path);
            let iccid = old
                .and_then(|p| p.iccid.clone())
                .or_else(|| sim_dir.iccid_of(&path));
            // A LIVE badge means "a worker is sitting on this port right now", so
            // it survives a refresh only while that is still true. Re-enumerating
            // does not touch the workers, and clearing the flag unconditionally
            // made the badges of a running bank go dark on every Refresh — worse
            // now that a background timer refreshes on its own.
            //
            // Three things have to hold, because a worker is bound to the tty
            // *name* it was spawned with for its whole life (`live::run_live`
            // reopens that exact name after an outage):
            //   - a live session is still running, else there is no worker at all;
            //   - the port is still enumerated (a vanished port has no entry here,
            //     so when it comes back it starts from a fresh `false`);
            //   - the name behind the stable path has not changed. If it has, the
            //     stick was replugged and renumbered: the old worker is stuck
            //     retrying a name that no longer exists and will never turn this
            //     entry green again, so a carried-over badge would be a lie.
            let live_ready = live_session
                && old.map(|p| p.live_ready && p.name == name).unwrap_or(false);
            // The error text survives a refresh, gated on the tty name for the
            // same reason as the badge above. Clearing it unconditionally meant a
            // port stuck in "Reconnecting: Port lost: EIO" or "Serial I/O failed:
            // …" went back to looking like an ordinary idle row within one
            // refresh interval — and it never came back, because `OutageLatch`
            // emits one event per outage rather than repeating. `alive` was
            // already carried over, so silence was the one failure the operator
            // could still see and the two that need explaining were the two that
            // got erased.
            //
            // Nothing here has to expire it: `start_live` and `stop_live` both
            // clear every `live_error` at their boundaries, and `detect_ports`
            // overwrites it per port with its own verdict.
            let live_error = old
                .filter(|p| p.name == name)
                .and_then(|p| p.live_error.clone());
            PortInfo {
                name,
                sim_number: sim_dir.number_of(&path, iccid.as_deref()),
                checked: old.map(|p| p.checked).unwrap_or(true),
                alive: old.and_then(|p| p.alive),
                iccid,
                path,
                live_ready,
                live_error,
            }
        })
        .collect()
}

#[tauri::command]
pub fn refresh_ports(state: tauri::State<'_, SharedState>) -> Vec<PortInfo> {
    let enumerated: Vec<(String, String)> = crate::core::modem::get_port_names()
        .into_iter()
        .map(|n| {
            let path = crate::core::modem::stable_id(&n);
            (n, path)
        })
        .collect();
    let mut st = lock_state(&state);
    let live_session = st.live_on || st.live_stop.is_some();
    let old = std::mem::take(&mut st.ports);
    let ports = merge_ports(enumerated, old, &st.sim_dir, live_session);
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

/// What a detect probe concluded about one port.
///
/// The distinction the two-state version lost: "nothing is in this slot" and "I
/// could not find out" are different answers, and only the first is evidence.
/// `detect_ports` folded an unopenable port, a host-side serial failure and a
/// panicking probe all into `alive = false`, which then deselected the port,
/// cleared its `iccid` and erased its slot→ICCID mapping from disk. A port held
/// by ModemManager for a moment, or one that hit EBUSY, was labelled `NO MODEM`
/// and lost its card's number permanently.
enum ProbeVerdict {
    /// A modem answered, with its ICCID if it would give one up.
    Alive(Option<String>),
    /// The probe's own silence — the only verdict allowed to set
    /// `PortInfo::alive = Some(false)`.
    Empty,
    /// The probe never got an answer either way. Nothing about the port's
    /// recorded state may change; the reason is shown so the row is not silently
    /// indistinguishable from a healthy one.
    Inconclusive(String),
}

impl ProbeVerdict {
    fn of(
        port: &str,
        probed: std::thread::Result<Result<crate::core::modem::ProbeResult, String>>,
    ) -> Self {
        match probed {
            Ok(Ok(r)) if r.alive => ProbeVerdict::Alive(r.iccid),
            Ok(Ok(r)) if r.proved_empty() => ProbeVerdict::Empty,
            Ok(Ok(r)) => ProbeVerdict::Inconclusive(
                r.failure
                    .unwrap_or_else(|| "Probe failed for an unknown reason".into()),
            ),
            Ok(Err(e)) => ProbeVerdict::Inconclusive(format!("Cannot open port: {e}")),
            Err(_) => {
                log::error!("Detect probe panicked on {}", port);
                ProbeVerdict::Inconclusive("Probe crashed — see the log".into())
            }
        }
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
    let busy = BusyGuard::new(&state, |st| st.detect_busy = false);
    thread::spawn(move || {
        let _busy = busy;
        let work = Arc::new(Mutex::new(names));
        let done = Arc::new(AtomicUsize::new(0));
        let alive_count = Arc::new(AtomicUsize::new(0));
        let unknown_count = Arc::new(AtomicUsize::new(0));
        let workers = total.min(MAX_CONCURRENT_PROBES);
        let mut handles = Vec::with_capacity(workers);
        for _ in 0..workers {
            let work2 = Arc::clone(&work);
            let st2 = Arc::clone(&state_clone);
            let app3 = app2.clone();
            let done2 = Arc::clone(&done);
            let alive2 = Arc::clone(&alive_count);
            let unknown2 = Arc::clone(&unknown_count);
            handles.push(thread::spawn(move || {
                while let Some(port) = take_port(&work2) {
                    let probed = catch_unwind(AssertUnwindSafe(|| {
                        crate::core::modem::probe_port(&port)
                    }));
                    let verdict = ProbeVerdict::of(&port, probed);
                    if matches!(verdict, ProbeVerdict::Alive(_)) {
                        alive2.fetch_add(1, Ordering::Relaxed);
                    }
                    if let ProbeVerdict::Inconclusive(_) = &verdict {
                        unknown2.fetch_add(1, Ordering::Relaxed);
                    }
                    let n = done2.fetch_add(1, Ordering::Relaxed) + 1;
                    let text;
                    {
                        let mut st = lock_state(&st2);
                        // Which card is in which slot is settled here, while the
                        // port is open and has just identified itself. Nothing
                        // downstream has to guess from tty numbering again.
                        let path = st
                            .ports
                            .iter()
                            .find(|p| p.name == port)
                            .map(|p| p.path.clone());
                        if let Some(path) = path {
                            match &verdict {
                                ProbeVerdict::Alive(Some(id)) => st.sim_dir.set_slot(&path, id),
                                // Answered but would not give up its ICCID —
                                // a transient refusal shouldn't erase what we
                                // already know about the slot.
                                ProbeVerdict::Alive(None) => {}
                                // Silence proved the slot is empty. The card's
                                // number stays on file under its own ICCID for
                                // whenever it shows up again.
                                ProbeVerdict::Empty => st.sim_dir.clear_slot(&path),
                                // We never got to ask. Erasing the mapping here
                                // is what lost a bank's slot→card hints to one
                                // momentary EBUSY.
                                ProbeVerdict::Inconclusive(_) => {}
                            }
                        }
                        if let Some(p) = st.ports.iter_mut().find(|p| p.name == port) {
                            p.live_ready = false;
                            match &verdict {
                                ProbeVerdict::Alive(iccid) => {
                                    p.alive = Some(true);
                                    // Leaving dead ports selected is what made
                                    // every later action pay their timeouts, so
                                    // the sweep owns the selection.
                                    p.checked = true;
                                    p.live_error = None;
                                    if iccid.is_some() {
                                        p.iccid = iccid.clone();
                                    }
                                }
                                ProbeVerdict::Empty => {
                                    p.alive = Some(false);
                                    p.checked = false;
                                    p.live_error = None;
                                    p.iccid = None;
                                }
                                // Selection, liveness and ICCID all keep their
                                // previous values: the probe failed, the port
                                // did not. The reason is surfaced so the row
                                // says why instead of quietly looking normal.
                                ProbeVerdict::Inconclusive(reason) => {
                                    p.live_error = Some(reason.clone());
                                }
                            }
                        }
                        let resolved = st
                            .ports
                            .iter()
                            .find(|p| p.name == port)
                            .map(|p| st.sim_dir.number_of(&p.path, p.iccid.as_deref()));
                        if let (Some(num), Some(p)) =
                            (resolved, st.ports.iter_mut().find(|p| p.name == port))
                        {
                            p.sim_number = num;
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
        let unknown = unknown_count.load(Ordering::Relaxed);
        let (text, ports_snapshot);
        {
            let mut st = lock_state(&state_clone);
            st.detect_busy = false;
            // The slot→card map changed, so persist it: the next launch can then
            // show numbers before anything has been probed.
            st.sim_dir.save();
            st.status_text = detect_done_status(found, total, unknown);
            text = st.status_text.clone();
            ports_snapshot = st.ports.clone();
        }
        log::info!("{}", text);
        let _ = app2.emit("status:update", &serde_json::json!({ "text": text }));
        let _ = app2.emit("ports:updated", &serde_json::json!({ "ports": ports_snapshot }));
        let _ = app2.emit(
            "detect:done",
            &serde_json::json!({ "found": found, "total": total, "unknown": unknown }),
        );
    });

    Ok("Detect started".into())
}

/// Detect's closing line. Ports the probe could not reach are counted
/// separately: they were left selected and keep whatever liveness they had, so
/// reporting them as "no modem, deselected" would be two lies at once.
fn detect_done_status(found: usize, total: usize, unknown: usize) -> String {
    let empty = total.saturating_sub(found).saturating_sub(unknown);
    let mut s = format!(
        "Detect done. Modems found: {}/{}  |  {} port(s) with no modem deselected",
        found, total, empty
    );
    if unknown > 0 {
        s.push_str(&format!(
            "  |  {} port(s) could not be probed — left as they were",
            unknown
        ));
    }
    s
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
    let busy = BusyGuard::new(&state, |st| st.scan_busy = false);
    thread::spawn(move || {
        let _busy = busy;
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
    let busy = BusyGuard::new(&state, |st| st.ussd_busy = false);
    thread::spawn(move || {
        let _busy = busy;
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
    let found_id = crate::core::modem::get_sim_number(port);
    let (num, err, iccid) = (found_id.number, found_id.error, found_id.iccid);
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
    {
        let mut st = lock_state(state);
        // Record the card even when the number lookup failed: knowing which SIM
        // sits in the slot is what keeps a previously-learned number attached to
        // the right port.
        if let Some(ref id) = iccid {
            let path = st
                .ports
                .iter()
                .find(|p| p.name == port)
                .map(|p| p.path.clone());
            if let Some(path) = path {
                st.sim_dir.set_slot(&path, id);
            }
        }
        if let Some(p) = st.ports.iter_mut().find(|p| p.name == port) {
            if let Some(alive) = observed_alive {
                p.alive = Some(alive);
            }
            if iccid.is_some() {
                p.iccid = iccid.clone();
            }
        }
    }
    if let Some(ref n) = num {
        found.fetch_add(1, Ordering::Relaxed);
        log::info!("USSD {} -> {}", port, crate::logging::mask_number(n));
        let ports_snapshot;
        {
            let mut st = lock_state(state);
            match iccid.as_deref() {
                // Filed against the card, so it survives renumbering and follows
                // the SIM into another slot.
                Some(id) => st.sim_dir.set_number(id, n),
                // No ICCID means no durable key. Show the number for this
                // session but do not write a guess to disk — that is how one
                // number ended up on two ports.
                None => log::warn!(
                    "{}: number {} not saved — the modem would not report its ICCID",
                    port,
                    crate::logging::mask_number(n)
                ),
            }
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

/// Retention windows past this point are indistinguishable from "keep
/// everything", so they are treated as off rather than converted. Ten years is
/// far beyond any operational window for a SIM that holds 20-50 messages, and
/// stopping here is what keeps `Duration::from_secs_f64` and the `chrono`
/// arithmetic in `retention_cutoff_ms` inside their domains.
const MAX_RETENTION_HOURS: f64 = 87_600.0;

/// Convert the UI's retention setting into a duration, or `None` when the
/// operator turned auto-cleanup off (0 or a nonsensical value).
///
/// The value arrives from `localStorage`, where an older profile or a hand edit
/// can leave anything at all, so the bounds are checked here and not in the
/// frontend. Out of range means "keep everything", which is the same answer as
/// off — the alternative was a panicking `Duration::from_secs_f64`, and the
/// worst caller is the live worker, where one panic per port turns into
/// `Worker crashed` on the whole bank.
fn retention_from_hours(hours: Option<f64>) -> Option<Duration> {
    let h = hours?;
    if !h.is_finite() || h <= 0.0 || h > MAX_RETENTION_HOURS {
        return None;
    }
    Duration::try_from_secs_f64(h * 3600.0).ok()
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
        // `live_stop` still set with `live_on` already cleared is the shutdown
        // window: port_busy() covers it now, but a bare "Busy" right after the
        // user pressed Stop reads as a bug rather than as "wait a moment".
        if st.live_stop.is_some() && !st.live_on {
            return Err("Live mode is still stopping — try again in a moment.".into());
        }
        if st.port_busy() {
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
    // The supervisor owns both live flags. It had no `catch_unwind`, so a panic
    // in it left `live_on` and `live_stop` set and `port_busy()` permanently
    // true — the app reports "Busy" to every port command until restart.
    let busy = BusyGuard::new(&state, |st| {
        st.live_on = false;
        st.live_stop = None;
    });
    thread::spawn(move || {
        let _busy = busy;
        let port_count = ports.len();
        let mut handles = Vec::with_capacity(port_count);
        for port in ports {
            let stop = Arc::clone(&shared_stop);
            let st2 = Arc::clone(&state_clone);
            let st3 = Arc::clone(&state_clone);
            let app2 = app.clone();
            let app3 = app.clone();
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
                        // Same rule as the scan and USSD paths: only the probe's
                        // own silence means "no modem". A host-side I/O failure
                        // proves nothing about the slot, and marking it empty
                        // would style a working stick as an unused one.
                        let silent = error == crate::core::modem::NOT_RESPONDING;
                        let (text, ports_snapshot);
                        {
                            let mut st = lock_state(&st2);
                            if let Some(p) = st.ports.iter_mut().find(|p| p.name == port) {
                                if silent {
                                    p.alive = Some(false);
                                }
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
                        // Sender masked and the OTP reduced to "found (n digits)":
                        // the log file rotates on size only and the ring buffer is
                        // shown verbatim on the Logs page, so a code written here
                        // would outlive the inbox retention window.
                        log::info!(
                            "NEW SMS on {}: from={} otp={}",
                            port,
                            crate::logging::mask_number(&message.from),
                            crate::logging::otp_summary(otp.as_deref())
                        );

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
                // The live workers were the one per-port pool with no
                // `catch_unwind`. A panic in `run_live` killed that port's
                // monitoring silently: `join()` swallowed it, nothing marked the
                // port failed, and the LIVE badge stayed green on a stick that
                // had stopped delivering. Report it the same way a transport
                // failure is reported so the row turns red and says why.
                let port_name = port.clone();
                let outcome = catch_unwind(AssertUnwindSafe(|| {
                    crate::core::live::run_live(port, stop, retention, sender);
                }));
                if outcome.is_err() {
                    log::error!("Live worker panicked on {}", port_name);
                    let reason = crate::core::live::WORKER_PANIC;
                    let (text, ports_snapshot);
                    {
                        let mut st = lock_state(&st3);
                        if let Some(p) = st.ports.iter_mut().find(|p| p.name == port_name) {
                            p.live_ready = false;
                            p.live_error = Some(reason.into());
                        }
                        st.live_failed.push((port_name.clone(), reason.into()));
                        st.status_text = format!("{} FAILED: {}", port_name, reason);
                        text = st.status_text.clone();
                        ports_snapshot = st.ports.clone();
                    }
                    let _ = app3.emit("status:update", &serde_json::json!({ "text": text }));
                    let _ = app3.emit(
                        "ports:updated",
                        &serde_json::json!({ "ports": ports_snapshot }),
                    );
                }
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

/// SIM slots one inbox row occupies. An assembled long SMS spans one slot per
/// fragment; a single-part message only has `index`.
///
/// Slot numbering starts at 1, so a non-positive index is not a slot — it is a
/// row whose origin never recorded one. Returning nothing for it is what makes
/// `confirmed_removals` keep the row: a bogus `AT+CMGD=0` is refused by the
/// modem, and slot 0 is then missing from the confirming `AT+CMGL` for the
/// trivial reason that it cannot be there, which would otherwise be read as
/// proof the message was deleted.
fn message_slots(m: &SmsMessage) -> Vec<i32> {
    let mut idxs = m.part_indices.clone();
    if idxs.is_empty() || idxs.len() == 1 && !idxs.contains(&m.index) {
        idxs = vec![m.index];
    }
    idxs.retain(|i| *i > 0);
    idxs
}

/// Which requested rows may actually leave the inbox.
///
/// `slots` maps each requested message id to the port and SIM slots it occupies
/// (captured before the delete, since the row is what we are deciding about);
/// `freed` is what each port's modem *confirmed* is gone, i.e. `OpResult.indices`
/// from `confirm_delete`, which re-reads the SIM rather than trusting per-command
/// replies. Everything else stays.
///
/// A row is removed only when every slot it occupies is confirmed gone: a
/// concatenated SMS holds one slot per fragment, and dropping the row while
/// fragments are still on the card made them come back at the next scan or live
/// reconnect looking like duplicates — the status line said "Deleted 2/6" while
/// all six rows vanished.
fn confirmed_removals(
    slots: &HashMap<u64, (String, Vec<i32>)>,
    freed: &HashMap<String, HashSet<i32>>,
) -> HashSet<u64> {
    let none = HashSet::new();
    slots
        .iter()
        .filter(|(_, (port, idxs))| {
            let gone = freed.get(port).unwrap_or(&none);
            // No slots recorded means no evidence at all — keep the row.
            !idxs.is_empty() && idxs.iter().all(|i| gone.contains(i))
        })
        .map(|(id, _)| *id)
        .collect()
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
    // Slots per port to hand the modems, plus the per-id slot map the removal
    // decision needs once the modems have answered.
    let to_delete: Vec<(String, Vec<i32>)>;
    let slots: HashMap<u64, (String, Vec<i32>)>;
    {
        let st = lock_state(&state);
        let requested: HashSet<u64> = ids.iter().copied().collect();
        let mut map: HashMap<String, Vec<i32>> = HashMap::new();
        let mut per_id: HashMap<u64, (String, Vec<i32>)> = HashMap::new();
        // One pass over the inbox against a set, rather than a linear find per
        // requested id — a bulk delete of a full inbox was quadratic.
        for item in st.messages.iter().filter(|m| requested.contains(&m.id)) {
            let idxs = message_slots(&item.message);
            map.entry(item.message.port.clone())
                .or_default()
                .extend(idxs.iter().copied());
            per_id.insert(item.id, (item.message.port.clone(), idxs));
        }
        slots = per_id;
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
    let busy = BusyGuard::new(&state, |st| st.delete_busy = false);
    thread::spawn(move || {
        let _busy = busy;
        let mut fail = 0usize;
        let mut wanted = 0usize;
        let mut gone = 0usize;
        // SIM slots the modems confirmed are gone, per port. This — not the
        // request — decides which rows leave the inbox.
        let mut freed: HashMap<String, HashSet<i32>> = HashMap::new();
        // A panic mid-delete would otherwise leave delete_busy set and the
        // toolbar spinner stuck; the flag is cleared below either way. Whatever
        // was confirmed before the panic still counts.
        let counted = catch_unwind(AssertUnwindSafe(|| {
            for (port, indices) in &to_delete {
                let r = crate::core::modem::delete_messages(port, Some(indices.as_slice()));
                wanted += indices.len();
                gone += r.deleted;
                freed
                    .entry(port.clone())
                    .or_default()
                    .extend(r.indices.iter().copied());
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
        let removed = confirmed_removals(&slots, &freed);
        let kept = slots.len().saturating_sub(removed.len());
        let text;
        {
            let mut st = lock_state(&state_clone);
            st.delete_busy = false;
            st.messages.retain(|m| !removed.contains(&m.id));
            // Count SIM slots, not ports. One port can free some of its indices
            // and leave others behind, and a "1 ok" per port would hide it. The
            // slot count can exceed the message count: a concatenated SMS
            // occupies one slot per part. Rows whose slots survived stay in the
            // list, so the message count is reported separately.
            st.status_text = if fail == 0 && kept == 0 && gone >= wanted {
                format!(
                    "Deleted {} message(s) ({} SIM slot(s) freed)",
                    removed.len(),
                    gone
                )
            } else {
                format!(
                    "Deleted {}/{} SIM slot(s)  |  {} message(s) removed, {} kept  |  FAILED: {} port(s)",
                    gone,
                    wanted,
                    removed.len(),
                    kept,
                    fail
                )
            };
            log::info!("{}", st.status_text);
            text = st.status_text.clone();
        }
        // Only the ids that really went away — the frontend drops exactly these
        // rows, so anything extra here is the data loss we are preventing.
        let mut removed_ids: Vec<u64> = removed.into_iter().collect();
        removed_ids.sort_unstable();
        let _ = app2.emit(
            "messages:removed",
            &serde_json::json!({ "ids": removed_ids }),
        );
        // The verdict as numbers, not as the display string. A partial delete is
        // the outcome the operator most needs to hear about, and the status text
        // it used to be carried in is rendered on the Ports page only — so
        // deleting from the Inbox looked identical whether 10 of 10 slots were
        // freed or 2. `kept` counts rows still on the card, `failed_ports` the
        // ports whose modem refused outright.
        let _ = app2.emit(
            "delete:done",
            &serde_json::json!({
                "requested": wanted,
                "freed": gone,
                "removed": removed_ids.len(),
                "kept": kept,
                "failed_ports": fail,
            }),
        );
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
    let busy = BusyGuard::new(&state, |st| st.cleanup_busy = false);
    thread::spawn(move || {
        let _busy = busy;
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
    use crate::core::modem::{ProbeResult, NOT_RESPONDING};

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
        // The largest window still treated as a real retention period.
        assert_eq!(
            retention_from_hours(Some(MAX_RETENTION_HOURS)),
            Some(Duration::from_secs(87_600 * 3600))
        );
    }

    /// The setting is rehydrated from `localStorage`, so an out-of-range value
    /// is normal input, not an exceptional one. It used to panic:
    /// `Duration::from_secs_f64` overflows past ~5.1e15 hours, and anything
    /// surviving that panicked in `retention_cutoff_ms` once
    /// `chrono::Duration::seconds` went out of bounds. The live worker calls
    /// this per port, so one bad value meant `Worker crashed` on the whole bank.
    #[test]
    fn an_absurd_retention_window_is_off_not_a_panic() {
        for h in [
            1e13,
            1e16,
            f64::MAX,
            MAX_RETENTION_HOURS + 1.0,
            i64::MAX as f64,
        ] {
            assert!(
                retention_from_hours(Some(h)).is_none(),
                "{h} hours should read as \"keep everything\""
            );
        }
    }

    /// A state where nothing owns a port.
    fn idle_state() -> AppStateInner {
        AppStateInner {
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
        }
    }

    #[test]
    fn port_busy_covers_every_operation_that_owns_a_port() {
        assert!(!idle_state().port_busy());

        let setters: [fn(&mut AppStateInner); 6] = [
            |st| st.scan_busy = true,
            |st| st.live_on = true,
            |st| st.ussd_busy = true,
            |st| st.delete_busy = true,
            |st| st.cleanup_busy = true,
            |st| st.detect_busy = true,
        ];
        for set in setters {
            let mut st = idle_state();
            set(&mut st);
            assert!(st.port_busy());
        }
    }

    #[test]
    fn port_busy_still_holds_while_live_workers_wind_down() {
        // stop_live clears live_on immediately, but the workers own their ports
        // until the supervisor joins them (up to 15 s in AT+CMGL=4). Every busy
        // boolean is false here and the ports are still held.
        let mut st = idle_state();
        assert!(!st.port_busy());
        st.live_stop = Some(Arc::new(AtomicBool::new(true)));
        assert!(st.port_busy());
        // Only the supervisor's final clear releases the gate.
        st.live_stop = None;
        assert!(!st.port_busy());
    }

    /// The whole point of the guard: the flag has to come back down even when the
    /// thread that set it never reaches its own clear.
    #[test]
    fn busy_guard_releases_the_gate_on_an_unwind() {
        let state: SharedState = Arc::new(Mutex::new(idle_state()));
        lock_state(&state).delete_busy = true;
        assert!(lock_state(&state).port_busy());

        let panicked = catch_unwind(AssertUnwindSafe(|| {
            let _busy = BusyGuard::new(&state, |st| st.delete_busy = false);
            panic!("worker died mid-operation");
        }));

        assert!(panicked.is_err());
        assert!(
            !lock_state(&state).port_busy(),
            "a panicking worker must not leave the app reporting Busy forever"
        );
    }

    #[test]
    fn busy_guard_releases_the_gate_on_a_normal_return() {
        let state: SharedState = Arc::new(Mutex::new(idle_state()));
        lock_state(&state).scan_busy = true;
        {
            let _busy = BusyGuard::new(&state, |st| st.scan_busy = false);
            assert!(lock_state(&state).port_busy());
        }
        assert!(!lock_state(&state).port_busy());
    }

    /// The commands still clear their own flag inline, because several of them do
    /// more work under that same lock. The guard has to tolerate running after
    /// that, and must not disturb a different operation that started since.
    #[test]
    fn busy_guard_is_a_no_op_once_the_flag_is_already_clear() {
        let state: SharedState = Arc::new(Mutex::new(idle_state()));
        {
            let _busy = BusyGuard::new(&state, |st| st.cleanup_busy = false);
            lock_state(&state).cleanup_busy = true;
            // The inline clear the command performs on its happy path.
            lock_state(&state).cleanup_busy = false;
            // A later operation, already running by the time the guard drops.
            lock_state(&state).scan_busy = true;
        }
        let st = lock_state(&state);
        assert!(!st.cleanup_busy);
        assert!(st.scan_busy, "the guard must only touch its own flag");
    }

    /// `start_live`'s supervisor owns both flags, and `live_stop` is what keeps
    /// `port_busy()` true through the shutdown window.
    #[test]
    fn busy_guard_releases_both_live_flags() {
        let state: SharedState = Arc::new(Mutex::new(idle_state()));
        {
            let mut st = lock_state(&state);
            st.live_on = true;
            st.live_stop = Some(Arc::new(AtomicBool::new(false)));
        }
        let panicked = catch_unwind(AssertUnwindSafe(|| {
            let _busy = BusyGuard::new(&state, |st| {
                st.live_on = false;
                st.live_stop = None;
            });
            panic!("supervisor died before joining its workers");
        }));
        assert!(panicked.is_err());
        let st = lock_state(&state);
        assert!(!st.live_on);
        assert!(st.live_stop.is_none());
        assert!(!st.port_busy());
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

    /// One enumerated port. `path` is the stable by-path id, `name` the tty name.
    fn port_row(name: &str, path: &str, live_ready: bool) -> PortInfo {
        PortInfo {
            name: name.into(),
            path: path.into(),
            checked: true,
            sim_number: "09651995803".into(),
            iccid: Some("8995010912345678901".into()),
            alive: Some(true),
            live_ready,
            live_error: None,
        }
    }

    const SLOT_A: &str = "pci-0000:03:00.3-usb-0:4.1:1.0-port0";
    const SLOT_B: &str = "pci-0000:03:00.3-usb-0:4.2:1.0-port0";

    #[test]
    fn refresh_keeps_the_live_badge_of_a_port_that_is_still_there() {
        // The Refresh-blanks-every-badge bug: re-enumerating does not stop the
        // workers, so a port that is still present stays LIVE. Its selection,
        // liveness and card have to survive too.
        let old = vec![port_row("/dev/ttyUSB3", SLOT_A, true)];
        let merged = merge_ports(
            vec![("/dev/ttyUSB3".into(), SLOT_A.into())],
            old,
            &SimDirectory::default(),
            true,
        );
        assert_eq!(merged.len(), 1);
        assert!(merged[0].live_ready);
        assert!(merged[0].checked);
        assert_eq!(merged[0].alive, Some(true));
        assert_eq!(merged[0].iccid.as_deref(), Some("8995010912345678901"));
    }

    #[test]
    fn no_live_session_means_no_live_badge_can_survive() {
        // Without a worker the flag cannot be true, whatever the old list said —
        // this is the invariant that keeps a background refresh from resurrecting
        // badges after live mode has wound down.
        let enumerated = vec![("/dev/ttyUSB3".to_string(), SLOT_A.to_string())];
        let merged = merge_ports(
            enumerated,
            vec![port_row("/dev/ttyUSB3", SLOT_A, true)],
            &SimDirectory::default(),
            false,
        );
        assert!(!merged[0].live_ready);
    }

    #[test]
    fn a_port_that_disappeared_and_came_back_is_not_live_again() {
        // Unplugged: the port drops out of the enumeration entirely.
        let gone = merge_ports(
            vec![("/dev/ttyUSB4".into(), SLOT_B.into())],
            vec![
                port_row("/dev/ttyUSB3", SLOT_A, true),
                port_row("/dev/ttyUSB4", SLOT_B, true),
            ],
            &SimDirectory::default(),
            true,
        );
        assert_eq!(gone.len(), 1);
        assert_eq!(gone[0].path, SLOT_B);
        // Replugged into the same slot under the same name: there is no longer an
        // old entry to carry anything from, and no worker was spawned for it, so
        // it must come back dark rather than green.
        let back = merge_ports(
            vec![
                ("/dev/ttyUSB3".into(), SLOT_A.into()),
                ("/dev/ttyUSB4".into(), SLOT_B.into()),
            ],
            gone,
            &SimDirectory::default(),
            true,
        );
        let revived = back.iter().find(|p| p.path == SLOT_A).unwrap();
        assert!(!revived.live_ready);
        assert!(revived.alive.is_none());
    }

    #[test]
    fn a_stick_renumbered_in_the_same_slot_does_not_inherit_the_badge() {
        // Same USB slot, new tty name — a replug within one refresh interval, or a
        // different stick pushed into that slot. The worker is bound to the old
        // name for life, so nothing will ever refresh this entry's badge.
        let merged = merge_ports(
            vec![("/dev/ttyUSB70".into(), SLOT_A.into())],
            vec![port_row("/dev/ttyUSB3", SLOT_A, true)],
            &SimDirectory::default(),
            true,
        );
        assert!(!merged[0].live_ready);
        // Everything the old code already carried by path still carries — this
        // change is about the badge only.
        assert!(merged[0].checked);
        assert_eq!(merged[0].alive, Some(true));
    }

    /// The two failures that most need explaining were the two a refresh erased.
    /// A background timer runs `refresh_ports` on its own, and `OutageLatch` emits
    /// one event per outage rather than repeating, so a blanked message never came
    /// back: the port spent the rest of the outage looking like an ordinary idle
    /// row.
    #[test]
    fn a_refresh_keeps_the_reason_a_port_is_failing() {
        let mut old = port_row("/dev/ttyUSB3", SLOT_A, false);
        old.live_error = Some("Reconnecting: Port lost: EIO".into());
        let merged = merge_ports(
            vec![("/dev/ttyUSB3".into(), SLOT_A.into())],
            vec![old],
            &SimDirectory::default(),
            true,
        );
        assert_eq!(
            merged[0].live_error.as_deref(),
            Some("Reconnecting: Port lost: EIO")
        );
    }

    #[test]
    fn a_renumbered_stick_does_not_inherit_the_previous_error() {
        // Gated on the tty name for the same reason as the badge: the message
        // describes a worker bound to a name that no longer exists, so carrying it
        // onto whatever is in that slot now would be misleading.
        let mut old = port_row("/dev/ttyUSB3", SLOT_A, false);
        old.live_error = Some("Serial I/O failed: Input/output error".into());
        let merged = merge_ports(
            vec![("/dev/ttyUSB70".into(), SLOT_A.into())],
            vec![old],
            &SimDirectory::default(),
            true,
        );
        assert!(merged[0].live_error.is_none());
    }

    #[test]
    fn a_healthy_port_still_refreshes_with_no_error() {
        let merged = merge_ports(
            vec![("/dev/ttyUSB3".into(), SLOT_A.into())],
            vec![port_row("/dev/ttyUSB3", SLOT_A, true)],
            &SimDirectory::default(),
            true,
        );
        assert!(merged[0].live_error.is_none());
    }

    #[test]
    fn detect_status_only_mentions_unreachable_ports_when_there_are_some() {
        assert_eq!(
            detect_done_status(7, 64, 0),
            "Detect done. Modems found: 7/64  |  57 port(s) with no modem deselected"
        );
        // The three that could not be probed are not counted as empty, because
        // they were left selected and kept whatever liveness they had.
        assert_eq!(
            detect_done_status(7, 64, 3),
            "Detect done. Modems found: 7/64  |  54 port(s) with no modem deselected  \
             |  3 port(s) could not be probed — left as they were"
        );
    }

    fn probe_ok(alive: bool, failure: Option<&str>, iccid: Option<&str>) -> ProbeResult {
        ProbeResult {            alive,
            failure: failure.map(Into::into),
            iccid: iccid.map(Into::into),
        }
    }

    /// The classification that used to be a two-way `match` collapsing every
    /// failure into `alive = false`. Only the probe's own silence is evidence
    /// about the slot; everything else is evidence about this machine.
    #[test]
    fn only_probe_silence_counts_as_an_empty_slot() {
        let silent = ProbeVerdict::of(
            "ttyUSB0",
            Ok(Ok(probe_ok(false, Some(NOT_RESPONDING), None))),
        );
        assert!(matches!(silent, ProbeVerdict::Empty));

        // A host-side read/write failure proves nothing about the slot.
        let broken = ProbeVerdict::of(
            "ttyUSB0",
            Ok(Ok(probe_ok(
                false,
                Some("Serial I/O failed: Input/output error"),
                None,
            ))),
        );
        assert!(matches!(broken, ProbeVerdict::Inconclusive(_)));

        // Held by ModemManager, unplugged, or a permissions problem.
        let unopenable: ProbeVerdict =
            ProbeVerdict::of("ttyUSB0", Ok(Err("Permission denied".into())));
        match unopenable {
            ProbeVerdict::Inconclusive(reason) => assert!(reason.contains("Permission denied")),
            _ => panic!("an unopenable port is not an empty slot"),
        }
    }

    #[test]
    fn a_live_port_carries_its_iccid_through_the_verdict() {
        let v = ProbeVerdict::of(
            "ttyUSB0",
            Ok(Ok(probe_ok(true, None, Some("8995010912345678901")))),
        );
        match v {
            ProbeVerdict::Alive(iccid) => {
                assert_eq!(iccid.as_deref(), Some("8995010912345678901"))
            }
            _ => panic!("a modem answered"),
        }
        // Answered but would not give up its ICCID — still alive, no ICCID to file.
        assert!(matches!(
            ProbeVerdict::of("ttyUSB0", Ok(Ok(probe_ok(true, None, None)))),
            ProbeVerdict::Alive(None)
        ));
    }


    #[test]
    fn a_slot_hint_filling_in_an_unknown_iccid_leaves_the_badge_alone() {
        // refresh_ports never probes, so the only ICCID change it can see at a
        // path is the directory hint filling in a card it had not recorded. That
        // hint is "last seen here", possibly from an earlier session, so it is no
        // evidence that the stick under the running worker changed — the tty-name
        // check is what covers a real swap.
        let mut dir = SimDirectory::default();
        dir.set_slot(SLOT_A, "8995010999999999999");
        let mut old = port_row("/dev/ttyUSB3", SLOT_A, true);
        old.iccid = None;
        let merged = merge_ports(
            vec![("/dev/ttyUSB3".into(), SLOT_A.into())],
            vec![old],
            &dir,
            true,
        );
        assert_eq!(merged[0].iccid.as_deref(), Some("8995010999999999999"));
        assert!(merged[0].live_ready);
    }

    #[test]
    fn a_new_port_defaults_to_selected_with_nothing_known_about_it() {
        let merged = merge_ports(
            vec![("/dev/ttyUSB9".into(), SLOT_B.into())],
            Vec::new(),
            &SimDirectory::default(),
            true,
        );
        assert!(merged[0].checked);
        assert!(!merged[0].live_ready);
        assert!(merged[0].alive.is_none());
        assert!(merged[0].iccid.is_none());
        assert_eq!(merged[0].sim_number, "");
    }

    fn row(id: u64, port: &str, index: i32, parts: &[i32]) -> SmsItem {
        SmsItem {
            id,
            message: SmsMessage {
                port: port.into(),
                index,
                from: "MYTEL".into(),
                received: chrono::Utc::now(),
                status: "REC READ".into(),
                text: "x".into(),
                part_indices: parts.to_vec(),
            },
            otp: None,
            is_new: false,
        }
    }

    fn slot_map(rows: &[SmsItem]) -> HashMap<u64, (String, Vec<i32>)> {
        rows.iter()
            .map(|r| (r.id, (r.message.port.clone(), message_slots(&r.message))))
            .collect()
    }

    fn freed(entries: &[(&str, &[i32])]) -> HashMap<String, HashSet<i32>> {
        entries
            .iter()
            .map(|(port, idxs)| (port.to_string(), idxs.iter().copied().collect()))
            .collect()
    }

    #[test]
    fn single_part_slots_fall_back_to_the_row_index() {
        assert_eq!(message_slots(&row(1, "ttyUSB0", 4, &[]).message), vec![4]);
        assert_eq!(
            message_slots(&row(1, "ttyUSB0", 4, &[2, 3, 4]).message),
            vec![2, 3, 4]
        );
    }

    /// SIM slots are numbered from 1. A row carrying 0 never learned its slot,
    /// and deleting it would be reported as a success it cannot have had: the
    /// modem refuses `AT+CMGD=0`, and the confirming `AT+CMGL` cannot list a
    /// slot that does not exist, so its absence proves nothing. No slots means
    /// no evidence, and `confirmed_removals` keeps the row.
    #[test]
    fn slot_zero_is_never_a_real_sim_slot() {
        let bare = row(1, "ttyUSB0", 0, &[]);
        assert!(message_slots(&bare.message).is_empty());
        assert!(confirmed_removals(&slot_map(&[bare]), &freed(&[("ttyUSB0", &[0])])).is_empty());

        // A concat whose fragments never recorded slots either.
        let concat = row(2, "ttyUSB0", 0, &[0]);
        assert!(message_slots(&concat.message).is_empty());

        // Real slots alongside a bogus one still delete the real ones.
        assert_eq!(
            message_slots(&row(3, "ttyUSB0", 0, &[0, 7, 8]).message),
            vec![7, 8]
        );
    }

    #[test]
    fn full_success_removes_every_requested_row() {
        let rows = vec![row(1, "ttyUSB0", 1, &[]), row(2, "ttyUSB0", 2, &[])];
        let removed = confirmed_removals(&slot_map(&rows), &freed(&[("ttyUSB0", &[1, 2])]));
        assert_eq!(removed, HashSet::from([1, 2]));
    }

    #[test]
    fn partial_failure_removes_only_the_confirmed_rows() {
        // The SIM refused 4 of 6 slots. The rows behind those slots have to stay,
        // or they reappear at the next scan looking like duplicates.
        let rows = vec![
            row(1, "ttyUSB0", 1, &[]),
            row(2, "ttyUSB0", 2, &[]),
            row(3, "ttyUSB0", 3, &[]),
            row(4, "ttyUSB1", 4, &[]),
        ];
        let removed = confirmed_removals(&slot_map(&rows), &freed(&[("ttyUSB0", &[1, 3])]));
        assert_eq!(removed, HashSet::from([1, 3]));
    }

    #[test]
    fn total_failure_removes_nothing() {
        let rows = vec![row(1, "ttyUSB0", 1, &[]), row(2, "ttyUSB1", 2, &[])];
        assert!(confirmed_removals(&slot_map(&rows), &HashMap::new()).is_empty());
        // A port that answered but confirmed no slot is the same story.
        assert!(
            confirmed_removals(&slot_map(&rows), &freed(&[("ttyUSB0", &[])])).is_empty()
        );
    }

    #[test]
    fn a_concat_row_stays_until_every_fragment_is_confirmed_gone() {
        let rows = vec![row(9, "ttyUSB0", 3, &[3, 4, 5])];
        // Two of three fragments freed: the message is still readable from the
        // SIM, so the row must not disappear.
        assert!(confirmed_removals(&slot_map(&rows), &freed(&[("ttyUSB0", &[3, 4])])).is_empty());
        assert_eq!(
            confirmed_removals(&slot_map(&rows), &freed(&[("ttyUSB0", &[3, 4, 5])])),
            HashSet::from([9])
        );
    }

    #[test]
    fn slots_are_matched_per_port_not_globally() {
        // Same slot numbers on two sticks: freeing index 1 on ttyUSB0 says
        // nothing about index 1 on ttyUSB1.
        let rows = vec![row(1, "ttyUSB0", 1, &[]), row(2, "ttyUSB1", 1, &[])];
        assert_eq!(
            confirmed_removals(&slot_map(&rows), &freed(&[("ttyUSB0", &[1])])),
            HashSet::from([1])
        );
    }
}
