#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    sms_tauri_lib::run();
}