pub mod commands;
pub mod core;
pub mod logging;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logging::init();
    log::info!("SIM Bank SMS Reader starting...");
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            logging::set_app_handle(app.handle().clone());
            let state = commands::new_shared_state();
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::refresh_ports,
            commands::checked_ports,
            commands::toggle_port_checked,
            commands::set_all_ports_checked,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
