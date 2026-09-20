use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tauri::State;

use crate::engine::manager::DownloadManager;
use crate::engine::probe::Prober;
use crate::engine::types::{DownloadTask, ProbeResult};

pub struct AppState {
    pub manager: Arc<DownloadManager>,
}

#[tauri::command]
pub async fn probe_url(
    state: State<'_, AppState>,
    url: String,
    headers: Option<HashMap<String, String>>,
) -> Result<ProbeResult, String> {
    Prober::probe(&state.manager.client, &url, headers).await
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
) -> Result<DownloadTask, String> {
    state
        .manager
        .start_download(app, url, filename, save_dir, connections, headers)
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
        let mut cmd = std::process::Command::new("cmd");
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
        let mut cmd = std::process::Command::new("cmd");
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
        let state = AppState { manager: manager.clone() };

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
}
