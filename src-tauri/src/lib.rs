pub mod commands;
pub mod core;
pub mod forwarder;
pub mod logging;
pub mod telegram;

use std::thread;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

/// Show, unminimise and focus the main window. Three calls rather than one
/// because they answer different states: hidden to the tray, minimised to the
/// taskbar, and open but behind something else.
fn reveal_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logging::init();
    log::info!("SIM Bank SMS Reader starting...");
    tauri::Builder::default()
        // First, as upstream requires. Closing the window only hides it, so the
        // process routinely outlives its last window with every serial port
        // still open — and a second launch by an operator who thought they had
        // quit would show a bank where nothing can be opened while the first
        // process is still reading it. Hand the existing window over instead.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            log::info!("Second instance blocked; revealing the running window");
            reveal_main_window(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let state = commands::new_shared_state();
            app.manage(state);

            let show_i = MenuItem::with_id(app, "show", "Open SMS Reader", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &sep, &quit_i])?;

            // Cloned into the menu handler so Quit can report that it is
            // working. The wind-down below can take up to `EXIT_WIND_DOWN`, and
            // a Quit that looks like it did nothing for twenty seconds is how an
            // operator ends up reaching for the power button.
            let quit_item = quit_i.clone();

            // `mut` because the icon and the Windows-only left-click flag are
            // both applied by reassignment below.
            let mut tray_builder = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("SIM Bank SMS Reader")
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "show" => reveal_main_window(app),
                    "quit" => {
                        let _ = quit_item.set_enabled(false);
                        let _ = quit_item.set_text("Quitting — finishing up…");
                        // Exiting the process here would skip the Telegram flush
                        // that the live supervisor performs once its workers are
                        // joined (`Memory/03 T6`). Wind the session down first,
                        // on a worker thread: this closure runs on the main
                        // thread and the wait is up to `EXIT_WIND_DOWN` long.
                        let app = app.clone();
                        thread::spawn(move || {
                            let state = app.state::<commands::SharedState>().inner().clone();
                            commands::wind_down_live(&state, commands::EXIT_WIND_DOWN);
                            app.exit(0);
                        });
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        reveal_main_window(tray.app_handle());
                    }
                });

            #[cfg(target_os = "windows")]
            {
                tray_builder = tray_builder.show_menu_on_left_click(false);
            }

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }

            // Keeping the menu in managed state is what keeps its GTK widget
            // tree alive; nothing ever reads it back. Drop it here and GNOME's
            // DBusMenu asks for labels the widgets no longer have, which draws
            // the tray menu as a blank rectangle (`Memory/03 §27`).
            app.manage(menu);
            let _tray = tray_builder.build(app)?;

            if let Some(window) = app.get_webview_window("main") {
                let win = window.clone();
                let state_handle = app.state::<commands::SharedState>();
                let state = state_handle.inner().clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        let should_hide = {
                            let s = commands::lock_state(&state);
                            // `live_on` overrides the setting on purpose: an
                            // accidental close must not end an unattended
                            // monitoring session. `live_stop.is_some()` is the
                            // shutdown window that follows it — `live_on` is
                            // already clear, but the workers still hold their
                            // ports and the Telegram flush has not run yet, so
                            // closing through it loses exactly what an abrupt
                            // Quit used to lose (`Memory/03 T6`). Quitting from
                            // the tray is the orderly way out of both.
                            s.minimize_to_tray || s.live_on || s.live_stop.is_some()
                        };
                        if should_hide {
                            api.prevent_close();
                            let _ = win.hide();
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::refresh_ports,
            commands::checked_ports,
            commands::toggle_port_checked,
            commands::set_all_ports_checked,
            commands::detect_ports,
            commands::start_scan,
            commands::get_ports,
            commands::get_sim_numbers,
            commands::get_messages,
            commands::get_status_text,
            commands::start_live,
            commands::stop_live,
            commands::delete_selected,
            commands::clear_all,
            commands::export_messages,
            commands::get_logs,
            commands::clear_logs,
            commands::get_log_file_path,
            commands::open_log_folder,
            commands::set_minimize_to_tray,
            commands::purge_expired_messages,
            commands::cleanup_sim_storage,
            commands::telegram::verify_telegram_token,
            commands::telegram::detect_telegram_group,
            commands::telegram::send_telegram_test,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
