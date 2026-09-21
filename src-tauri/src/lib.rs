pub mod commands;
pub mod crash_reporter;
pub mod db;
pub mod engine;
pub mod ipc_server;
pub mod logger;
pub mod native_messaging;
pub mod tray;

use std::sync::Arc;
use tauri::{Manager, WindowEvent};

use commands::AppState;
use db::Database;
use engine::manager::DownloadManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // 1. Initialize robust file and console logger
            logger::AppLogger::init();
            log_info!("init", "My Own IDM application starting up...");

            // 2. Initialize GlitchTip / Sentry Crash Reporter & Panic Integration
            crash_reporter::init();

            // Setup SQLite DB in AppData
            let app_data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let db_path = app_data_dir.join("idm.db");
            let db = Database::new(db_path).expect("Failed to initialize SQLite database");
            let db_arc = Arc::new(db);

            let manager = Arc::new(DownloadManager::new(db_arc));

            app.manage(AppState { manager });

            // Start Windows Named Pipe IPC Server for Browser Extensions
            ipc_server::start_ipc_server(app.handle().clone());

            // Auto-register browser Native Messaging Host manifests
            if let Ok(exe_path) = std::env::current_exe() {
                let _ = native_messaging::register_native_messaging_manifests(&exe_path);
            }

            // Setup tray
            if let Err(e) = tray::setup_tray(app.handle()) {
                eprintln!("Warning: Failed to setup tray icon: {}", e);
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            // Minimize to tray on close for main window only
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::probe_url,
            commands::check_duplicate_download,
            commands::start_download,
            commands::pause_download,
            commands::resume_download,
            commands::cancel_download,
            commands::get_all_tasks,
            commands::get_default_download_dir,
            commands::open_file_in_folder,
            commands::open_file,
            commands::open_log_folder,
            commands::get_recent_logs,
            commands::move_downloaded_file,
            commands::open_external_url,
            commands::refresh_download_url,
            commands::set_global_speed_limit,
            commands::get_global_speed_limit,
            commands::set_task_speed_limit,
            commands::open_transfer_window,
            commands::close_current_window,
            commands::minimize_current_window,
            commands::start_dragging_window,
            commands::get_app_settings,
            commands::save_app_settings,
            commands::register_native_host_manifest,
            commands::report_diagnostic_error
        ])

        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
