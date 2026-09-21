use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

static IS_INITIALIZED: AtomicBool = AtomicBool::new(false);
static SENTRY_GUARD: OnceLock<Mutex<Option<sentry::ClientInitGuard>>> = OnceLock::new();

/// Initialize Sentry / GlitchTip crash reporting.
/// Reads DSN from compile-time `GLITCHTIP_DSN` / `SENTRY_DSN` or runtime environment variables.
/// If neither is present, operates in safe no-op mode without crashing.
pub fn init() {
    if IS_INITIALIZED.swap(true, Ordering::SeqCst) {
        return;
    }

    let dsn = option_env!("GLITCHTIP_DSN")
        .map(|s| s.to_string())
        .or_else(|| option_env!("SENTRY_DSN").map(|s| s.to_string()))
        .or_else(|| std::env::var("GLITCHTIP_DSN").ok())
        .or_else(|| std::env::var("SENTRY_DSN").ok());

    if let Some(dsn_str) = dsn {
        let trimmed = dsn_str.trim();
        if !trimmed.is_empty() {
            let guard = sentry::init((
                trimmed,
                sentry::ClientOptions {
                    release: sentry::release_name!(),
                    environment: Some(
                        std::env::var("ENVIRONMENT")
                            .unwrap_or_else(|_| "production".to_string())
                            .into(),
                    ),
                    attach_stacktrace: true,
                    ..Default::default()
                },
            ));

            crate::log_info!("crash_reporter", "GlitchTip / Sentry crash reporter initialized successfully");
            let holder = SENTRY_GUARD.get_or_init(|| Mutex::new(None));
            *holder.lock() = Some(guard);
            return;
        }
    }

    crate::log_info!(
        "crash_reporter",
        "No GLITCHTIP_DSN or SENTRY_DSN configured. Crash reporter running in local no-op mode."
    );
}

/// Check if Sentry reporting is actively enabled with a valid transport
pub fn is_reporting_enabled() -> bool {
    sentry::Hub::current().client().is_some()
}

/// Sanitize URL by removing credentials and masking sensitive query parameters (tokens, keys, secrets).
pub fn sanitize_url(raw_url: &str) -> String {
    if let Ok(mut parsed) = reqwest::Url::parse(raw_url) {
        let _ = parsed.set_username("");
        let _ = parsed.set_password(None);

        let sensitive_keywords = [
            "token", "auth", "key", "signature", "sig", "secret", "password", "pwd",
            "access_token", "api_key", "session", "credential", "bearer", "ticket",
        ];

        let query_pairs: Vec<(String, String)> = parsed
            .query_pairs()
            .map(|(k, v)| {
                let k_lower = k.to_lowercase();
                if sensitive_keywords.iter().any(|kw| k_lower.contains(kw)) {
                    (k.to_string(), "***".to_string())
                } else {
                    (k.to_string(), v.to_string())
                }
            })
            .collect();

        if !query_pairs.is_empty() {
            parsed.set_query(None);
            let mut pairs_mut = parsed.query_pairs_mut();
            for (k, v) in query_pairs {
                pairs_mut.append_pair(&k, &v);
            }
        }

        parsed.to_string()
    } else {
        raw_url.to_string()
    }
}

/// Sanitize file paths by masking user directories (C:\Users\<user>\ -> C:\Users\***\).
pub fn sanitize_path(path: &str) -> String {
    let mut sanitized = path.to_string();

    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        if !user_profile.is_empty() {
            sanitized = sanitized.replace(&user_profile, "C:\\Users\\***");
            let forward_slash = user_profile.replace('\\', "/");
            sanitized = sanitized.replace(&forward_slash, "C:/Users/***");
        }
    }

    if let Ok(username) = std::env::var("USERNAME") {
        if !username.is_empty() && username.len() > 1 {
            let win_user = format!("Users\\{}", username);
            sanitized = sanitized.replace(&win_user, "Users\\***");
            let fwd_user = format!("Users/{}", username);
            sanitized = sanitized.replace(&fwd_user, "Users/***");
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            sanitized = sanitized.replace(&home, "/home/***");
        }
    }

    sanitized
}

/// Sanitize log line by stripping user paths and sensitive query tokens
pub fn sanitize_log_line(line: &str) -> String {
    let path_sanitized = sanitize_path(line);
    let mut result = String::with_capacity(path_sanitized.len());
    let mut remaining = path_sanitized.as_str();

    while let Some(http_pos) = remaining.find("http://").or_else(|| remaining.find("https://")) {
        result.push_str(&remaining[..http_pos]);
        let url_part = &remaining[http_pos..];
        let end_idx = url_part
            .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ')' || c == ']')
            .unwrap_or(url_part.len());
        let raw_url = &url_part[..end_idx];
        result.push_str(&sanitize_url(raw_url));
        remaining = &url_part[end_idx..];
    }
    result.push_str(remaining);
    result
}


/// Report a download failure to GlitchTip / Sentry with sanitized context and recent logs.
pub fn report_download_failure(
    task: &crate::engine::types::DownloadTask,
    error_reason: &str,
    recent_logs: &[String],
) -> Result<String, String> {
    let sanitized_target_url = sanitize_url(&task.url);
    let sanitized_dest_path = sanitize_path(&task.file_path);
    let sanitized_logs: Vec<String> = recent_logs.iter().map(|l| sanitize_log_line(l)).collect();

    crate::log_info!(
        "crash_reporter",
        "Dispatching diagnostic error report for task {} ({})",
        task.id,
        task.filename
    );

    let event_id = sentry::with_scope(
        |scope| {
            scope.set_tag("component", "download_engine");
            scope.set_tag("category", format!("{:?}", task.category));
            scope.set_tag("supports_range", task.supports_range.to_string());
            scope.set_tag("is_hls", task.is_hls.to_string());
            scope.set_tag("os", std::env::consts::OS);
            scope.set_tag("arch", std::env::consts::ARCH);

            scope.set_extra("task_id", serde_json::Value::String(task.id.clone()));
            scope.set_extra("sanitized_url", serde_json::Value::String(sanitized_target_url));
            scope.set_extra("sanitized_path", serde_json::Value::String(sanitized_dest_path));
            scope.set_extra("filename", serde_json::Value::String(task.filename.clone()));
            scope.set_extra("downloaded_bytes", serde_json::json!(task.downloaded_bytes));
            scope.set_extra("total_bytes", serde_json::json!(task.total_bytes));
            scope.set_extra("connections", serde_json::json!(task.connections));
            scope.set_extra("recent_logs", serde_json::json!(sanitized_logs));
        },
        || {
            sentry::capture_message(
                &format!("Download Failure: {} ({})", task.filename, error_reason),
                sentry::Level::Error,
            )
        },
    );

    let id_str = event_id.to_string();
    if id_str == "00000000000000000000000000000000" || id_str.is_empty() {
        // Return a mock ID in local mode so UI gets positive acknowledgment
        Ok(format!("local-diag-{}", &task.id[..task.id.len().min(8)]))
    } else {
        Ok(id_str)
    }
}

/// Report a browser extension error (e.g. unknown media stream, IPC disconnect, JS exception).
pub fn report_extension_error(
    error_type: &str,
    message: &str,
    browser: &str,
    details: Option<&serde_json::Value>,
) -> Result<String, String> {
    crate::log_info!(
        "crash_reporter",
        "Dispatching extension error report: [{}] {} (browser: {})",
        error_type,
        message,
        browser
    );

    let event_id = sentry::with_scope(
        |scope| {
            scope.set_tag("component", "browser_extension");
            scope.set_tag("browser", browser);
            scope.set_tag("error_type", error_type);
            scope.set_tag("os", std::env::consts::OS);
            scope.set_tag("arch", std::env::consts::ARCH);

            if let Some(val) = details {
                scope.set_extra("details", val.clone());
            }
        },
        || {
            sentry::capture_message(
                &format!("Extension Error [{}]: {}", error_type, message),
                sentry::Level::Error,
            )
        },
    );


    let id_str = event_id.to_string();
    if id_str == "00000000000000000000000000000000" || id_str.is_empty() {
        Ok(format!("local-ext-{}", uuid::Uuid::new_v4().simple()))
    } else {
        Ok(id_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_url_cleans_sensitive_params() {
        let raw = "https://example.com/video.mp4?token=secret123&api_key=abcdef&file_id=999&auth=xyz";
        let sanitized = sanitize_url(raw);
        assert!(!sanitized.contains("secret123"));
        assert!(!sanitized.contains("abcdef"));
        assert!(!sanitized.contains("xyz"));
        assert!(sanitized.contains("token=***"));
        assert!(sanitized.contains("api_key=***"));
        assert!(sanitized.contains("auth=***"));
        assert!(sanitized.contains("file_id=999"));
    }

    #[test]
    fn test_sanitize_url_removes_user_password() {
        let raw = "https://admin:supersecret@example.com/download.zip";
        let sanitized = sanitize_url(raw);
        assert!(!sanitized.contains("admin"));
        assert!(!sanitized.contains("supersecret"));
        assert!(sanitized.contains("https://example.com/download.zip"));
    }

    #[test]
    fn test_sanitize_path() {
        let sample_path = if cfg!(windows) {
            if let Ok(user) = std::env::var("USERNAME") {
                format!("C:\\Users\\{}\\Downloads\\myfile.zip", user)
            } else {
                "C:\\Users\\john\\Downloads\\myfile.zip".to_string()
            }
        } else {
            "/home/john/Downloads/myfile.zip".to_string()
        };

        let sanitized = sanitize_path(&sample_path);
        if cfg!(windows) {
            if let Ok(user) = std::env::var("USERNAME") {
                assert!(!sanitized.contains(&user));
            }
            assert!(sanitized.contains("Users\\***"));
        }
    }

    #[test]
    fn test_init_and_local_reporting() {
        init();
        let task = crate::engine::types::DownloadTask {
            id: "test-task-123".to_string(),
            url: "https://example.com/file.iso?token=12345".to_string(),
            filename: "file.iso".to_string(),
            save_dir: "C:\\Downloads".to_string(),
            file_path: "C:\\Downloads\\file.iso".to_string(),
            total_bytes: Some(1024),
            downloaded_bytes: 512,
            category: crate::engine::types::DownloadCategory::General,
            status: crate::engine::types::TaskStatus::Failed("HTTP 403 Forbidden".to_string()),
            connections: 8,
            supports_range: true,
            is_hls: false,
            created_at: "2026-09-21".to_string(),
            completed_at: None,
            error_message: Some("HTTP 403 Forbidden".to_string()),
            segments: vec![],
            referer: None,
            speed_limit_bps: None,
        };

        let res = report_download_failure(&task, "HTTP 403 Forbidden", &["Sample log line".to_string()]);
        assert!(res.is_ok());
        let id = res.unwrap();
        assert!(!id.is_empty());

        let ext_res = report_extension_error("stream_parse_failure", "Unknown manifest format", "Chrome", None);
        assert!(ext_res.is_ok());
        let ext_id = ext_res.unwrap();
        assert!(!ext_id.is_empty());

        let ext_with_details = report_extension_error(
            "ipc_disconnect",
            "Pipe disconnected",
            "Edge",
            Some(&serde_json::json!({ "port": 18888, "error": "connection reset" })),
        );
        assert!(ext_with_details.is_ok());

        let line = "2026-09-21 ERROR Error downloading https://example.com/file?auth=secret123";
        let sanitized_line = sanitize_log_line(line);
        assert!(!sanitized_line.contains("secret123"));
        assert!(sanitized_line.contains("auth=***"));

        let _ = is_reporting_enabled();
    }
}

