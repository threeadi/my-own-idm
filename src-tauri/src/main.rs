// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a.contains("native-messaging") || a.contains("com.myownidm.host")) {
        tauri_app_lib::native_messaging::run_native_messaging_host();
        return;
    }

    tauri_app_lib::run()
}
