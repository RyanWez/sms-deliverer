pub mod commands;
pub mod core;
pub mod forwarder;
pub mod logging;
pub mod telegram;

use tauri::Manager;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logging::init();
    log::info!("SIM Bank SMS Reader starting...");
    tauri::Builder::default()
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

            #[allow(unused_mut)]
            let mut tray_builder = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("SIM Bank SMS Reader")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
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
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                });

            #[cfg(target_os = "windows")]
            {
                tray_builder = tray_builder.show_menu_on_left_click(false);
            }

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }

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
                            s.minimize_to_tray || s.live_on
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
