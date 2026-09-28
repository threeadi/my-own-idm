use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tauri::State;

use crate::engine::manager::DownloadManager;
use crate::engine::probe::Prober;
use crate::engine::types::{DownloadTask, DuplicateCheckResult, ProbeResult};

pub struct AppState {
    pub manager: Arc<DownloadManager>,
    pub telegram: Arc<crate::engine::telegram::TelegramManager>,
}

#[tauri::command]
pub async fn probe_url(
    state: State<'_, AppState>,
    url: String,
    headers: Option<HashMap<String, String>>,
) -> Result<ProbeResult, String> {
    Prober::probe(&state.manager.get_client(), &url, headers).await
}

#[tauri::command]
pub async fn check_duplicate_download(
    state: State<'_, AppState>,
    url: String,
    filename: String,
    save_dir: String,
) -> Result<DuplicateCheckResult, String> {
    Ok(state.manager.check_duplicate_task(&url, &filename, &save_dir).await)
}

#[tauri::command]
pub async fn start_download(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    url: String,
    filename: String,
    save_dir: String,
    connections: usize,
    headers: Option<HashMap<String, String>>,
    quality: Option<String>,
) -> Result<DownloadTask, String> {
    state
        .manager
        .start_download(app, url, filename, save_dir, connections, headers, quality)
        .await
}

#[tauri::command]
pub async fn pause_download(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<(), String> {
    state.manager.pause_download(&task_id).await
}

#[tauri::command]
pub async fn resume_download(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    task_id: String,
) -> Result<(), String> {
    state.manager.resume_download(app, &task_id).await
}

#[tauri::command]
pub async fn cancel_download(
    state: State<'_, AppState>,
    task_id: String,
    delete_file: bool,
) -> Result<(), String> {
    state.manager.cancel_download(&task_id, delete_file).await
}

#[tauri::command]
pub async fn get_all_tasks(state: State<'_, AppState>) -> Result<Vec<DownloadTask>, String> {
    Ok(state.manager.get_all_tasks().await)
}

#[tauri::command]
pub async fn get_default_download_dir() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        if let Some(user_profile) = std::env::var_os("USERPROFILE") {
            let path = Path::new(&user_profile).join("Downloads");
            return Ok(path.to_string_lossy().to_string());
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let path = Path::new(&home).join("Downloads");
            return Ok(path.to_string_lossy().to_string());
        }
    }
    Ok("C:\\Downloads".to_string())
}

pub fn build_open_in_folder_command(path: &str) -> std::process::Command {
    #[cfg(target_os = "windows")]
    {
        let mut cmd = std::process::Command::new("explorer");
        cmd.arg("/select,").arg(path);
        cmd
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = std::process::Command::new("xdg-open");
        if let Some(parent) = Path::new(path).parent() {
            cmd.arg(parent);
        }
        cmd
    }
}

pub fn build_open_file_command(path: &str) -> std::process::Command {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let mut cmd = std::process::Command::new("cmd");
        cmd.creation_flags(0x08000000);
        cmd.args(["/C", "start", "", path]);
        cmd
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = std::process::Command::new("xdg-open");
        cmd.arg(path);
        cmd
    }
}

#[tauri::command]
pub fn open_file_in_folder(path: String) -> Result<(), String> {
    build_open_in_folder_command(&path)
        .spawn()
        .map_err(|e| format!("Failed to open folder: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn open_file(path: String) -> Result<(), String> {
    build_open_file_command(&path)
        .spawn()
        .map_err(|e| format!("Failed to open file: {}", e))?;
    Ok(())
}

pub fn build_open_folder_command(path: &str) -> std::process::Command {
    #[cfg(target_os = "windows")]
    {
        let mut cmd = std::process::Command::new("explorer");
        cmd.arg(path);
        cmd
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = std::process::Command::new("xdg-open");
        cmd.arg(path);
        cmd
    }
}

#[tauri::command]
pub fn open_log_folder() -> Result<(), String> {
    let log_dir = crate::logger::get_log_dir();
    build_open_folder_command(&log_dir.to_string_lossy())
        .spawn()
        .map_err(|e| format!("Failed to open log folder: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn get_recent_logs(max_lines: Option<usize>) -> Vec<String> {
    crate::logger::AppLogger::get().get_recent_lines(max_lines.unwrap_or(100))
}

#[tauri::command]
pub async fn move_downloaded_file(
    state: State<'_, AppState>,
    task_id: String,
    new_dir: String,
) -> Result<DownloadTask, String> {
    let tasks = state.manager.tasks.read().await;
    let task = tasks.get(&task_id).ok_or("Task not found")?.clone();
    drop(tasks);

    let old_path = Path::new(&task.file_path);
    if !old_path.exists() {
        return Err(format!("File not found at: {}", task.file_path));
    }

    let new_dir_path = Path::new(&new_dir);
    if !new_dir_path.exists() {
        std::fs::create_dir_all(new_dir_path)
            .map_err(|e| format!("Failed to create destination directory: {}", e))?;
    }

    let filename = old_path
        .file_name()
        .ok_or("Invalid file name")?
        .to_string_lossy();
    let new_file_path = new_dir_path.join(filename.as_ref());

    if let Err(_) = std::fs::rename(old_path, &new_file_path) {
        std::fs::copy(old_path, &new_file_path)
            .map_err(|e| format!("Failed to copy file to new directory: {}", e))?;
        let _ = std::fs::remove_file(old_path);
    }

    let new_path_str = new_file_path.to_string_lossy().to_string();
    state
        .manager
        .update_task_path(&task_id, &new_dir, &new_path_str)
        .await
}

pub fn build_open_url_command(url: &str) -> std::process::Command {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let mut cmd = std::process::Command::new("cmd");
        cmd.creation_flags(0x08000000);
        cmd.args(["/C", "start", "", url]);
        cmd
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = std::process::Command::new("xdg-open");
        cmd.arg(url);
        cmd
    }
}

#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL harus diawali dengan http:// atau https://".to_string());
    }
    build_open_url_command(&url)
        .spawn()
        .map_err(|e| format!("Failed to open browser URL: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn refresh_download_url(
    state: State<'_, AppState>,
    task_id: String,
    new_url: String,
) -> Result<ProbeResult, String> {
    state.manager.refresh_task_url(&task_id, &new_url).await
}

#[tauri::command]
pub async fn set_global_speed_limit(
    state: State<'_, AppState>,
    enabled: bool,
    limit_bps: Option<u64>,
) -> Result<(), String> {
    state.manager.set_global_speed_limit(enabled, limit_bps).await
}

#[tauri::command]
pub async fn get_global_speed_limit(
    state: State<'_, AppState>,
) -> Result<crate::engine::types::GlobalSpeedLimitConfig, String> {
    state.manager.get_global_speed_limit().await
}

#[tauri::command]
pub async fn set_task_speed_limit(
    state: State<'_, AppState>,
    task_id: String,
    limit_bps: Option<u64>,
) -> Result<(), String> {
    state.manager.set_task_speed_limit(&task_id, limit_bps).await
}

#[tauri::command]
pub async fn get_app_settings(
    state: State<'_, AppState>,
) -> Result<HashMap<String, String>, String> {
    state.manager.db.get_all_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_app_settings(
    state: State<'_, AppState>,
    settings: HashMap<String, String>,
) -> Result<(), String> {
    state.manager.db.set_multiple_settings(&settings).map_err(|e| e.to_string())?;
    state.manager.reload_client();
    Ok(())
}

#[tauri::command]
pub async fn test_proxy_connection(
    proxy_type: String,
    proxy_host: String,
    proxy_port: u16,
    proxy_auth: bool,
    proxy_user: Option<String>,
    proxy_pass: Option<String>,
) -> Result<String, String> {
    let start = std::time::Instant::now();
    let proxy_url_str = match proxy_type.to_lowercase().as_str() {
        "socks5" => format!("socks5h://{}:{}", proxy_host.trim(), proxy_port),
        "https" => format!("https://{}:{}", proxy_host.trim(), proxy_port),
        _ => format!("http://{}:{}", proxy_host.trim(), proxy_port),
    };

    let mut proxy = reqwest::Proxy::all(&proxy_url_str)
        .map_err(|e| format!("URL Proxy tidak valid: {}", e))?;

    if proxy_auth {
        if let (Some(user), Some(pass)) = (proxy_user, proxy_pass) {
            if !user.is_empty() {
                proxy = proxy.basic_auth(&user, &pass);
            }
        }
    }

    let client = reqwest::Client::builder()
        .proxy(proxy)
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| format!("Gagal menginisialisasi client proxy: {}", e))?;

    let res = client
        .get("https://httpbin.org/ip")
        .send()
        .await
        .map_err(|e| format!("Koneksi ke proxy gagal: {}", e))?;

    let elapsed = start.elapsed().as_millis();
    if res.status().is_success() {
        let text = res.text().await.unwrap_or_default();
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(origin) = json.get("origin").and_then(|o| o.as_str()) {
                return Ok(format!("Terhubung! IP Proxy: {} (Latensi: {} ms)", origin, elapsed));
            }
        }
        Ok(format!("Terhubung ke Proxy! (Latensi: {} ms)", elapsed))
    } else {
        Err(format!("Proxy merespons dengan status HTTP {}", res.status()))
    }
}

#[tauri::command]
pub fn register_native_host_manifest() -> Result<(), String> {
    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    crate::native_messaging::register_native_messaging_manifests(&current_exe)
}

pub fn sanitize_window_label(task_id: &str) -> String {
    let sanitized: String = task_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    format!("transfer-{}", sanitized)
}

#[tauri::command]
pub async fn open_transfer_window(
    app: tauri::AppHandle,
    task_id: String,
) -> Result<(), String> {
    use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

    let label = sanitize_window_label(&task_id);

    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
        return Ok(());
    }

    let url_path = format!("transfer?id={}", task_id);
    let builder = WebviewWindowBuilder::new(&app, &label, WebviewUrl::App(url_path.into()))
        .title("Transfer Unduhan")
        .inner_size(560.0, 420.0)
        .min_inner_size(480.0, 340.0)
        .resizable(true)
        .decorations(false)
        .shadow(true)
        .center();

    builder.build().map_err(|e| format!("Failed to create transfer window: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn close_current_window(window: tauri::Window) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn minimize_current_window(window: tauri::Window) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_dragging_window(window: tauri::Window) -> Result<(), String> {
    window.start_dragging().map_err(|e| e.to_string())
}

pub async fn report_diagnostic_error_inner(
    manager: &DownloadManager,
    task_id: &str,
    error_message: Option<String>,
) -> Result<String, String> {
    let task = {
        let tasks = manager.tasks.read().await;
        tasks.get(task_id).cloned()
    };

    let task = match task {
        Some(t) => t,
        None => {
            let all = manager.db.load_all_tasks().map_err(|e| e.to_string())?;
            all.into_iter()
                .find(|t| t.id == task_id)
                .ok_or_else(|| format!("Task {} not found", task_id))?
        }
    };

    let recent_logs = crate::logger::AppLogger::get().get_recent_lines(30);
    let error_reason = error_message
        .or_else(|| task.error_message.clone())
        .unwrap_or_else(|| "Unknown failure".to_string());

    crate::crash_reporter::report_download_failure(&task, &error_reason, &recent_logs)
}

#[tauri::command]
pub async fn report_diagnostic_error(
    state: State<'_, AppState>,
    task_id: String,
    error_message: Option<String>,
) -> Result<String, String> {
    report_diagnostic_error_inner(&state.manager, &task_id, error_message).await
}

#[tauri::command]
pub async fn telegram_set_credentials(
    state: State<'_, AppState>,
    api_id: String,
    api_hash: String,
) -> Result<(), String> {
    state.telegram.set_credentials(&api_id, &api_hash)
}

#[tauri::command]
pub async fn telegram_request_otp(
    state: State<'_, AppState>,
    phone_number: String,
) -> Result<String, String> {
    state.telegram.request_otp(&phone_number)
}

#[tauri::command]
pub async fn telegram_verify_otp(
    state: State<'_, AppState>,
    phone_number: String,
    code: String,
    phone_code_hash: String,
) -> Result<crate::engine::telegram::TelegramAuthStatus, String> {
    state.telegram.verify_otp(&phone_number, &code, &phone_code_hash)
}

#[tauri::command]
pub async fn telegram_login_bot(
    state: State<'_, AppState>,
    bot_token: String,
) -> Result<crate::engine::telegram::TelegramAuthStatus, String> {
    state.telegram.login_with_bot(&bot_token)
}

#[tauri::command]
pub async fn telegram_get_auth_status(
    state: State<'_, AppState>,
) -> Result<crate::engine::telegram::TelegramAuthStatus, String> {
    Ok(state.telegram.get_auth_status())
}

#[tauri::command]
pub async fn telegram_logout(
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.telegram.logout()
}

#[tauri::command]
pub async fn telegram_list_dialogs(
    state: State<'_, AppState>,
) -> Result<Vec<crate::engine::telegram::TelegramChannelInfo>, String> {
    state.telegram.list_dialogs()
}

#[tauri::command]
pub async fn telegram_delete_dialog(
    state: State<'_, AppState>,
    dialog_id: String,
) -> Result<Vec<crate::engine::telegram::TelegramChannelInfo>, String> {
    state.telegram.delete_dialog(&dialog_id)
}

#[tauri::command]
pub async fn telegram_scan_media(
    state: State<'_, AppState>,
    chat_input: String,
    media_filter: Option<String>,
) -> Result<Vec<crate::engine::telegram::TelegramMediaItem>, String> {
    state.telegram.scan_chat_media(&chat_input, media_filter.as_deref())
}

#[tauri::command]
pub async fn telegram_list_accounts(
    state: State<'_, AppState>,
) -> Result<Vec<crate::engine::telegram::TelegramAccountInfo>, String> {
    Ok(state.telegram.list_accounts())
}

#[tauri::command]
pub async fn telegram_switch_account(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<crate::engine::telegram::TelegramAuthStatus, String> {
    state.telegram.switch_account(&account_id)
}

#[tauri::command]
pub async fn telegram_remove_account(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Vec<crate::engine::telegram::TelegramAccountInfo>, String> {
    state.telegram.remove_account(&account_id)
}



#[cfg(test)]

mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_default_download_dir() {
        let res = get_default_download_dir().await;
        assert!(res.is_ok());
        let dir = res.unwrap();
        assert!(!dir.is_empty());
        assert!(dir.contains("Downloads"));
    }

    #[test]
    fn test_get_recent_logs_command() {
        let logs = get_recent_logs(Some(10));
        assert!(logs.len() <= 10);

        let default_logs = get_recent_logs(None);
        assert!(default_logs.len() <= 100);
    }

    #[tokio::test]
    async fn test_app_state_and_manager_methods() {
        let db = Arc::new(crate::db::Database::open_in_memory().unwrap());
        let manager = Arc::new(DownloadManager::new(db));
        let telegram = Arc::new(crate::engine::telegram::TelegramManager::new());
        let state = AppState { manager: manager.clone(), telegram };

        let all = state.manager.get_all_tasks().await;
        assert_eq!(all.len(), 0);

        let pause_res = state.manager.pause_download("non-existent-task").await;
        assert!(pause_res.is_ok());

        let cancel_res = state.manager.cancel_download("non-existent-task", false).await;
        assert!(cancel_res.is_ok());
    }

    #[test]
    fn test_build_open_in_folder_command() {
        let cmd = build_open_in_folder_command("C:\\Downloads\\file.mp4");
        let program = cmd.get_program().to_string_lossy();
        assert!(program.contains("explorer") || program.contains("xdg-open"));
    }

    #[test]
    fn test_build_open_file_command() {
        let cmd = build_open_file_command("C:\\Downloads\\file.mp4");
        let program = cmd.get_program().to_string_lossy();
        assert!(program.contains("cmd") || program.contains("xdg-open"));
    }

    #[test]
    fn test_build_open_folder_command() {
        let cmd = build_open_folder_command("C:\\Downloads");
        let program = cmd.get_program().to_string_lossy();
        assert!(program.contains("explorer") || program.contains("xdg-open"));
    }

    #[test]
    fn test_build_open_url_command() {
        let cmd = build_open_url_command("https://example.com/page");
        let program = cmd.get_program().to_string_lossy();
        assert!(program.contains("cmd") || program.contains("xdg-open"));
    }

    #[test]
    fn test_open_external_url_invalid_protocol() {
        let res = open_external_url("javascript:alert(1)".to_string());
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("http://"));
    }

    #[tokio::test]
    async fn test_speed_limit_commands_flow() {
        let db = Arc::new(crate::db::Database::open_in_memory().unwrap());
        let manager = Arc::new(DownloadManager::new(db));

        // Test global speed limit
        let res_set = manager.set_global_speed_limit(true, Some(2_000_000)).await;
        assert!(res_set.is_ok());

        let cfg = manager.get_global_speed_limit().await.unwrap();
        assert!(cfg.enabled);
        assert_eq!(cfg.limit_bps, Some(2_000_000));

        // Test task speed limit
        let res_task = manager.set_task_speed_limit("task-1", Some(1_000_000)).await;
        assert!(res_task.is_ok());
    }

    #[test]
    fn test_sanitize_window_label() {
        assert_eq!(sanitize_window_label("abc-123"), "transfer-abc-123");
        assert_eq!(sanitize_window_label("task with spaces!@#"), "transfer-task_with_spaces___");
        assert_eq!(sanitize_window_label("uuid-v4_1234"), "transfer-uuid-v4_1234");
    }

    #[tokio::test]
    async fn test_settings_commands_flow() {
        let db = Arc::new(crate::db::Database::open_in_memory().unwrap());
        let _manager = Arc::new(DownloadManager::new(db.clone()));

        let mut sample = HashMap::new();
        sample.insert("default_connections".to_string(), "16".to_string());
        sample.insert("notify_on_complete".to_string(), "true".to_string());

        let res_save = db.set_multiple_settings(&sample);
        assert!(res_save.is_ok());

        let loaded = db.get_all_settings().unwrap();
        assert_eq!(loaded.get("default_connections"), Some(&"16".to_string()));
        assert_eq!(loaded.get("notify_on_complete"), Some(&"true".to_string()));
    }

    #[tokio::test]
    async fn test_check_duplicate_download_command() {
        let db = Arc::new(crate::db::Database::open_in_memory().unwrap());
        let manager = Arc::new(DownloadManager::new(db.clone()));

        let res = manager.check_duplicate_task("https://example.com/file.zip", "file.zip", "C:\\Downloads").await;
        assert!(!res.is_duplicate);
    }

    #[tokio::test]
    async fn test_update_task_path_and_logs() {
        use crate::engine::types::{DownloadCategory, TaskStatus};
        let db = Arc::new(crate::db::Database::open_in_memory().unwrap());
        let manager = Arc::new(DownloadManager::new(db.clone()));

        let task = DownloadTask {
            id: "move-task-1".to_string(),
            url: "https://example.com/f.zip".to_string(),
            filename: "f.zip".to_string(),
            save_dir: "C:\\Downloads".to_string(),
            file_path: "C:\\Downloads\\f.zip".to_string(),
            total_bytes: Some(1024),
            downloaded_bytes: 1024,
            category: DownloadCategory::General,
            status: TaskStatus::Completed,
            connections: 4,
            supports_range: true,
            is_hls: false,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            completed_at: None,
            error_message: None,
            segments: vec![],
            referer: None,
            speed_limit_bps: None,
            quality: None,
        };
        db.insert_task(&task).unwrap();
        manager.tasks.write().await.insert(task.id.clone(), task);

        let updated = manager.update_task_path("move-task-1", "D:\\NewDir", "D:\\NewDir\\f.zip").await;
        assert!(updated.is_ok());
        let t = updated.unwrap();
        assert_eq!(t.save_dir, "D:\\NewDir");
        assert_eq!(t.file_path, "D:\\NewDir\\f.zip");

        let logs = get_recent_logs(Some(10));
        assert!(logs.len() <= 10);
    }

    #[tokio::test]
    async fn test_report_diagnostic_error_flow() {
        use crate::engine::types::{DownloadCategory, TaskStatus};
        crate::crash_reporter::init();

        let db = Arc::new(crate::db::Database::open_in_memory().unwrap());
        let manager = Arc::new(DownloadManager::new(db.clone()));

        let task = DownloadTask {
            id: "diag-task-1".to_string(),
            url: "https://example.com/test.zip?token=secret".to_string(),
            filename: "test.zip".to_string(),
            save_dir: "C:\\Downloads".to_string(),
            file_path: "C:\\Downloads\\test.zip".to_string(),
            total_bytes: Some(2048),
            downloaded_bytes: 512,
            category: DownloadCategory::General,
            status: TaskStatus::Failed("Connection reset".to_string()),
            connections: 4,
            supports_range: true,
            is_hls: false,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            completed_at: None,
            error_message: Some("Connection reset".to_string()),
            segments: vec![],
            referer: None,
            speed_limit_bps: None,
            quality: None,
        };
        db.insert_task(&task).unwrap();
        manager.tasks.write().await.insert(task.id.clone(), task.clone());

        // 1. In-memory task branch
        let res_mem = report_diagnostic_error_inner(&manager, "diag-task-1", Some("Explicit timeout".to_string())).await;
        assert!(res_mem.is_ok());
        assert!(!res_mem.unwrap().is_empty());

        // 2. Fallback DB task branch (remove from in-memory map)
        manager.tasks.write().await.remove("diag-task-1");
        let res_db = report_diagnostic_error_inner(&manager, "diag-task-1", None).await;
        assert!(res_db.is_ok());
        assert!(!res_db.unwrap().is_empty());

        // 3. Non-existent task branch (returns Err)
        let res_missing = report_diagnostic_error_inner(&manager, "non-existent-task-999", None).await;
        assert!(res_missing.is_err());
        assert!(res_missing.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn test_proxy_configuration_and_manager_reload() {
        let db = Arc::new(crate::db::Database::open_in_memory().unwrap());
        let manager = Arc::new(DownloadManager::new(db.clone()));

        // Disabled by default
        assert_eq!(DownloadManager::get_proxy_url_from_db(&db), None);

        // Enable HTTP proxy
        db.set_setting("proxyEnabled", "true").unwrap();
        db.set_setting("proxyType", "http").unwrap();
        db.set_setting("proxyHost", "127.0.0.1").unwrap();
        db.set_setting("proxyPort", "8080").unwrap();

        let proxy_url = DownloadManager::get_proxy_url_from_db(&db);
        assert_eq!(proxy_url, Some("http://127.0.0.1:8080".to_string()));

        // Reload client with new proxy
        manager.reload_client();
        let _ = manager.get_client();

        // Enable SOCKS5 with authentication
        db.set_setting("proxyType", "socks5").unwrap();
        db.set_setting("proxyHost", "10.0.0.1").unwrap();
        db.set_setting("proxyPort", "1080").unwrap();
        db.set_setting("proxyAuth", "true").unwrap();
        db.set_setting("proxyUser", "alice").unwrap();
        db.set_setting("proxyPass", "secret").unwrap();

        let socks_url = DownloadManager::get_proxy_url_from_db(&db);
        assert_eq!(socks_url, Some("socks5://alice:secret@10.0.0.1:1080".to_string()));

        manager.reload_client();
    }

    #[tokio::test]
    async fn test_test_proxy_connection_command_flow() {
        // Test with unreachable port/host - should return Err gracefully without panic
        let res = test_proxy_connection(
            "http".to_string(),
            "127.0.0.1".to_string(),
            59999,
            false,
            None,
            None,
        ).await;

        assert!(res.is_err());
    }
}



