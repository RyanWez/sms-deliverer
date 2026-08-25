pub mod core;
pub mod commands;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
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
            commands::get_sim_numbers,
            commands::get_messages,
            commands::get_status_text,
            commands::start_live,
            commands::stop_live,
            commands::delete_selected,
            commands::clear_all,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}