use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
#[cfg(target_os = "windows")]
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};

pub const PIPE_NAME: &str = r"\\.\pipe\my_own_idm_ipc";
pub const HTTP_PORT: u16 = 18888;

pub fn start_ipc_server(app_handle: AppHandle) {
    // 1. Start Local HTTP Server for Browser Extensions (Port 18888)
    let http_app = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        let addr = format!("127.0.0.1:{}", HTTP_PORT);
        match TcpListener::bind(&addr).await {
            Ok(listener) => {
                println!(">> My Own IDM IPC HTTP server listening on http://{}", addr);
                loop {
                    if let Ok((mut socket, _)) = listener.accept().await {
                        let app = http_app.clone();
                        tauri::async_runtime::spawn(async move {
                            handle_http_client(&mut socket, app).await;
                        });
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to bind HTTP IPC server on {}: {}", addr, e);
            }
        }
    });

    // 2. Start Windows Named Pipe for Native Messaging Host
    #[cfg(target_os = "windows")]
    {
        tauri::async_runtime::spawn(async move {
            loop {
                let server = match ServerOptions::new()
                    .first_pipe_instance(false)
                    .create(PIPE_NAME)
                {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Named pipe creation error: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        continue;
                    }
                };

                if server.connect().await.is_ok() {
                    let app = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        handle_pipe_client(server, app).await;
                    });
                }
            }
        });
    }
}

pub fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|w| w == b"\r\n\r\n").map(|pos| pos + 4)
}

pub fn parse_content_length(header_str: &str) -> Option<usize> {
    for line in header_str.lines() {
        if line.to_ascii_lowercase().starts_with("content-length:") {
            if let Some(val) = line.split(':').nth(1) {
                if let Ok(len) = val.trim().parse::<usize>() {
                    return Some(len);
                }
            }
        }
    }
    None
}

pub fn parse_http_body(buffer: &[u8], header_end: usize) -> String {
    let req_str = String::from_utf8_lossy(buffer);
    if header_end <= req_str.len() {
        req_str[header_end..].trim_matches(char::from(0)).trim_start_matches('\u{feff}').trim().to_string()
    } else {
        String::new()
    }
}

pub fn build_cors_preflight_response() -> &'static str {
    "HTTP/1.1 204 No Content\r\n\
Access-Control-Allow-Origin: *\r\n\
Access-Control-Allow-Private-Network: true\r\n\
Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
Access-Control-Allow-Headers: Content-Type, Authorization, Range, X-Requested-With, *\r\n\
Access-Control-Max-Age: 86400\r\n\
Content-Length: 0\r\n\
Connection: close\r\n\r\n"
}

pub fn build_json_response(status_code: u16, status_text: &str, body_json: &str) -> String {
    format!(
        "HTTP/1.1 {} {}\r\n\
Access-Control-Allow-Origin: *\r\n\
Access-Control-Allow-Private-Network: true\r\n\
Content-Type: application/json\r\n\
Content-Length: {}\r\n\
Connection: close\r\n\r\n{}",
        status_code,
        status_text,
        body_json.len(),
        body_json
    )
}

pub fn parse_http_path(req_str: &str) -> &str {
    if let Some(first_line) = req_str.lines().next() {
        let mut parts = first_line.split_whitespace();
        let _method = parts.next();
        if let Some(path) = parts.next() {
            return path;
        }
    }
    ""
}

pub async fn process_http_request<S, F>(socket: &mut S, mut on_payload: F)
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
    F: FnMut(&serde_json::Value),
{
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    let mut content_length: Option<usize> = None;
    let mut header_end: Option<usize> = None;

    // Robust loop to read entire HTTP request including headers and body
    loop {
        match socket.read(&mut chunk).await {
            Ok(0) => break,
            Ok(n) => {
                buffer.extend_from_slice(&chunk[..n]);

                // Detect end of headers
                if header_end.is_none() {
                    if let Some(h_end) = find_header_end(&buffer) {
                        header_end = Some(h_end);
                        let header_str = String::from_utf8_lossy(&buffer[..h_end]);
                        content_length = parse_content_length(&header_str);
                    }
                }

                // If headers found, check if full body received
                if let Some(h_end) = header_end {
                    let required_len = h_end + content_length.unwrap_or(0);
                    if buffer.len() >= required_len {
                        break;
                    }
                }
            }
            Err(_) => break,
        }
    }

    let req_str = String::from_utf8_lossy(&buffer);

    // Handle CORS OPTIONS preflight (including Chrome Private Network Access PNA!)
    if req_str.starts_with("OPTIONS") {
        let cors_resp = build_cors_preflight_response();
        let _ = socket.write_all(cors_resp.as_bytes()).await;
        let _ = socket.flush().await;
        let _ = socket.shutdown().await;
        return;
    }

    let path = parse_http_path(&req_str);

    // Parse HTTP Body
    if let Some(h_end) = header_end {
        let body = parse_http_body(&buffer, h_end);
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body) {
            if path.starts_with("/error-report") {
                let error_type = val.get("error_type").and_then(|v| v.as_str()).unwrap_or("unknown");
                let message = val.get("message").and_then(|v| v.as_str()).unwrap_or("Unknown extension error");
                let browser = val.get("browser").and_then(|v| v.as_str()).unwrap_or("Unknown Browser");
                let details = val.get("details");
                let event_id = crate::crash_reporter::report_extension_error(error_type, message, browser, details).unwrap_or_default();
                let reply_body = serde_json::json!({ "status": "ok", "event_id": event_id }).to_string();
                let ok_resp = build_json_response(200, "OK", &reply_body);
                let _ = socket.write_all(ok_resp.as_bytes()).await;
                let _ = socket.flush().await;
                let _ = socket.shutdown().await;
                return;
            }

            on_payload(&val);

            let ok_resp = build_json_response(200, "OK", "{\"status\":\"ok\"}");
            let _ = socket.write_all(ok_resp.as_bytes()).await;
            let _ = socket.flush().await;
            let _ = socket.shutdown().await;
            return;
        }
    }

    let bad_resp = build_json_response(400, "Bad Request", "{\"error\":\"invalid payload\"}");
    let _ = socket.write_all(bad_resp.as_bytes()).await;
    let _ = socket.flush().await;
    let _ = socket.shutdown().await;
}

async fn handle_http_client(socket: &mut tokio::net::TcpStream, app: AppHandle) {
    process_http_request(socket, |val| {
        trigger_download_popup(&app, val);
    })
    .await;
}

pub async fn process_pipe_message<S, F>(server: &mut S, mut on_payload: F)
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
    F: FnMut(&serde_json::Value),
{
    let mut len_buf = [0u8; 4];
    if server.read_exact(&mut len_buf).await.is_err() {
        return;
    }
    let msg_len = u32::from_le_bytes(len_buf) as usize;
    if msg_len == 0 || msg_len > 10 * 1024 * 1024 {
        return;
    }

    let mut msg_buf = vec![0u8; msg_len];
    if server.read_exact(&mut msg_buf).await.is_err() {
        return;
    }

    if let Ok(json_str) = std::str::from_utf8(&msg_buf) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
            on_payload(&val);

            // Reply OK to client
            let reply = serde_json::json!({ "status": "ok" }).to_string();
            let reply_bytes = reply.as_bytes();
            let reply_len = (reply_bytes.len() as u32).to_le_bytes();
            let _ = server.write_all(&reply_len).await;
            let _ = server.write_all(reply_bytes).await;
        }
    }
}

#[cfg(target_os = "windows")]
async fn handle_pipe_client(mut server: NamedPipeServer, app: AppHandle) {
    process_pipe_message(&mut server, |val| {
        if val.get("type").and_then(|v| v.as_str()) == Some("error_report") {
            let error_type = val.get("error_type").and_then(|v| v.as_str()).unwrap_or("unknown");
            let message = val.get("message").and_then(|v| v.as_str()).unwrap_or("Unknown extension error");
            let browser = val.get("browser").and_then(|v| v.as_str()).unwrap_or("Chrome (NamedPipe)");
            let details = val.get("details");
            let _ = crate::crash_reporter::report_extension_error(error_type, message, browser, details);
        } else {
            trigger_download_popup(&app, val);
        }
    })
    .await;
}


fn trigger_download_popup(app: &AppHandle, val: &serde_json::Value) {
    crate::log_info!("ipc", "Received browser download request: {:?}", val);
    
    // 1. Forward payload globally to frontend
    let _ = app.emit("browser-download-requested", val);

    // 2. Focus, unminimize, and bring window to front over Chrome
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.emit("browser-download-requested", val);
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_always_on_top(true);
        let _ = win.set_focus();

        // Release always_on_top after 400ms so the user can interact naturally
        let win_clone = win.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
            let _ = win_clone.set_always_on_top(false);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_header_end() {
        let req = b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\nHello Body";
        let end = find_header_end(req);
        assert_eq!(end, Some(35));
    }

    #[test]
    fn test_parse_content_length() {
        let headers1 = "POST / HTTP/1.1\r\nContent-Length: 42\r\nConnection: close";
        assert_eq!(parse_content_length(headers1), Some(42));

        let headers2 = "content-length: 1024\r\n";
        assert_eq!(parse_content_length(headers2), Some(1024));

        let headers3 = "GET / HTTP/1.1\r\n";
        assert_eq!(parse_content_length(headers3), None);
    }

    #[test]
    fn test_parse_http_body() {
        let raw = b"POST / HTTP/1.1\r\n\r\n{\"url\":\"https://example.com/video.mp4\"}\0\0";
        let end = find_header_end(raw).unwrap();
        let body = parse_http_body(raw, end);
        assert_eq!(body, "{\"url\":\"https://example.com/video.mp4\"}");
    }

    #[test]
    fn test_cors_preflight_response() {
        let resp = build_cors_preflight_response();
        assert!(resp.starts_with("HTTP/1.1 204 No Content"));
        assert!(resp.contains("Access-Control-Allow-Origin: *"));
        assert!(resp.contains("Access-Control-Allow-Private-Network: true"));
        assert!(resp.contains("Access-Control-Allow-Methods: GET, POST, OPTIONS"));
    }

    #[test]
    fn test_json_response() {
        let body = "{\"status\":\"ok\"}";
        let resp = build_json_response(200, "OK", body);
        assert!(resp.starts_with("HTTP/1.1 200 OK"));
        assert!(resp.contains(&format!("Content-Length: {}", body.len())));
        assert!(resp.ends_with(body));
    }

    #[tokio::test]
    async fn test_process_http_request_options() {
        let (mut client, mut server) = tokio::io::duplex(1024);

        let server_task = tokio::spawn(async move {
            process_http_request(&mut server, |_| {}).await;
        });

        client.write_all(b"OPTIONS /download HTTP/1.1\r\nHost: 127.0.0.1:18888\r\n\r\n").await.unwrap();
        let mut response = Vec::new();
        client.read_to_end(&mut response).await.unwrap();
        let _ = server_task.await;

        let resp_str = String::from_utf8_lossy(&response);
        assert!(resp_str.contains("204 No Content"));
        assert!(resp_str.contains("Access-Control-Allow-Origin: *"));
    }

    #[tokio::test]
    async fn test_process_http_request_post_valid_json() {
        let (mut client, mut server) = tokio::io::duplex(2048);

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let server_task = tokio::spawn(async move {
            process_http_request(&mut server, move |val| {
                let _ = tx.send(val.clone());
            }).await;
        });

        let body = r#"{"action":"download","url":"https://example.com/file.zip"}"#;
        let req = format!(
            "POST /download HTTP/1.1\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
            body.len(),
            body
        );
        client.write_all(req.as_bytes()).await.unwrap();

        let mut response = Vec::new();
        client.read_to_end(&mut response).await.unwrap();
        let _ = server_task.await;

        let received = rx.recv().await.unwrap();
        assert_eq!(received["url"], "https://example.com/file.zip");

        let resp_str = String::from_utf8_lossy(&response);
        assert!(resp_str.contains("200 OK"));
        assert!(resp_str.contains(r#"{"status":"ok"}"#));
    }

    #[tokio::test]
    async fn test_process_http_request_invalid_json() {
        let (mut client, mut server) = tokio::io::duplex(2048);

        let server_task = tokio::spawn(async move {
            process_http_request(&mut server, |_| {}).await;
        });

        let bad_body = "not a valid json string";
        let req = format!(
            "POST /download HTTP/1.1\r\nContent-Length: {}\r\n\r\n{}",
            bad_body.len(),
            bad_body
        );
        client.write_all(req.as_bytes()).await.unwrap();

        let mut response = Vec::new();
        client.read_to_end(&mut response).await.unwrap();
        let _ = server_task.await;

        let resp_str = String::from_utf8_lossy(&response);
        assert!(resp_str.contains("400 Bad Request"));
    }

    #[tokio::test]
    async fn test_process_pipe_message_valid() {
        let (mut client, mut server) = tokio::io::duplex(2048);

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let server_task = tokio::spawn(async move {
            process_pipe_message(&mut server, move |val| {
                let _ = tx.send(val.clone());
            }).await;
        });

        let msg = r#"{"url":"https://example.com/video.mp4"}"#;
        let len_bytes = (msg.len() as u32).to_le_bytes();
        client.write_all(&len_bytes).await.unwrap();
        client.write_all(msg.as_bytes()).await.unwrap();

        let mut resp_len_buf = [0u8; 4];
        client.read_exact(&mut resp_len_buf).await.unwrap();
        let r_len = u32::from_le_bytes(resp_len_buf) as usize;
        let mut r_buf = vec![0u8; r_len];
        client.read_exact(&mut r_buf).await.unwrap();
        let _ = server_task.await;

        let received = rx.recv().await.unwrap();
        assert_eq!(received["url"], "https://example.com/video.mp4");
        assert_eq!(String::from_utf8(r_buf).unwrap(), r#"{"status":"ok"}"#);
    }

    #[tokio::test]
    async fn test_process_pipe_message_oversized() {
        let (mut client, mut server) = tokio::io::duplex(1024);

        let server_task = tokio::spawn(async move {
            process_pipe_message(&mut server, |_| {}).await;
        });

        // 20 MB length prefix (greater than 10MB limit)
        let len_bytes = (20 * 1024 * 1024u32).to_le_bytes();
        client.write_all(&len_bytes).await.unwrap();
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn test_process_http_request_chunked_reads() {
        let (mut client, mut server) = tokio::io::duplex(2048);

        let server_task = tokio::spawn(async move {
            process_http_request(&mut server, |_| {}).await;
        });

        // Write first part (headers only)
        client.write_all(b"POST /download HTTP/1.1\r\nContent-Length: 17\r\n\r\n").await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(15)).await;
        // Write second part (body)
        client.write_all(b"{\"action\":\"ping\"}").await.unwrap();

        let mut response = Vec::new();
        client.read_to_end(&mut response).await.unwrap();
        let _ = server_task.await;

        let resp_str = String::from_utf8_lossy(&response);
        assert!(resp_str.contains("200 OK"));
    }

    #[tokio::test]
    async fn test_process_pipe_message_zero_length() {
        let (mut client, mut server) = tokio::io::duplex(1024);

        let server_task = tokio::spawn(async move {
            process_pipe_message(&mut server, |_| {}).await;
        });

        // Send 0 length
        client.write_all(&0u32.to_le_bytes()).await.unwrap();
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn test_process_pipe_message_read_error() {
        let (mut client, mut server) = tokio::io::duplex(1024);

        let server_task = tokio::spawn(async move {
            process_pipe_message(&mut server, |_| {}).await;
        });

        // Write length 100 but drop client immediately (incomplete payload)
        client.write_all(&100u32.to_le_bytes()).await.unwrap();
        drop(client);
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn test_process_pipe_message_invalid_json() {
        let (mut client, mut server) = tokio::io::duplex(1024);

        let server_task = tokio::spawn(async move {
            process_pipe_message(&mut server, |_| {}).await;
        });

        let bad_msg = "not json at all";
        let len_bytes = (bad_msg.len() as u32).to_le_bytes();
        client.write_all(&len_bytes).await.unwrap();
        client.write_all(bad_msg.as_bytes()).await.unwrap();
        drop(client);
        let _ = server_task.await;
    }

    #[test]
    fn test_parse_http_body_oob() {
        assert_eq!(parse_http_body(b"abc", 10), "");
    }

    #[test]
    fn test_parse_http_path() {
        assert_eq!(parse_http_path("POST /error-report HTTP/1.1\r\nHost: 127.0.0.1"), "/error-report");
        assert_eq!(parse_http_path("GET /download?id=123 HTTP/1.1\r\n"), "/download?id=123");
        assert_eq!(parse_http_path("INVALID"), "");
    }

    #[tokio::test]
    async fn test_process_http_request_error_report() {
        let (mut client, mut server) = tokio::io::duplex(2048);

        let server_task = tokio::spawn(async move {
            process_http_request(&mut server, |_| {}).await;
        });

        let body = r#"{"error_type":"unknown_stream","message":"M3U8 master format unhandled","browser":"Firefox"}"#;
        let req = format!(
            "POST /error-report HTTP/1.1\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
            body.len(),
            body
        );
        client.write_all(req.as_bytes()).await.unwrap();

        let mut response = Vec::new();
        client.read_to_end(&mut response).await.unwrap();
        let _ = server_task.await;

        let resp_str = String::from_utf8_lossy(&response);
        assert!(resp_str.contains("200 OK"));
        assert!(resp_str.contains("\"status\":\"ok\""));
    }
}




