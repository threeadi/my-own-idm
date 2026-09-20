use std::io::{self, Read, Write};
use std::path::Path;

pub const HOST_NAME: &str = "com.myownidm.host";

pub fn run_native_messaging_host() {
    run_native_messaging_loop(io::stdin(), io::stdout(), forward_to_main_app);
}

pub fn run_native_messaging_loop<R: Read, W: Write, F>(mut stdin: R, mut stdout: W, forward_fn: F)
where
    F: Fn(&[u8]) -> String,
{
    loop {
        // 1. Read 4 bytes length
        let mut len_buf = [0u8; 4];
        if stdin.read_exact(&mut len_buf).is_err() {
            break;
        }
        let msg_len = u32::from_ne_bytes(len_buf) as usize;
        if msg_len == 0 || msg_len > 10 * 1024 * 1024 {
            break;
        }

        // 2. Read JSON message
        let mut msg_buf = vec![0u8; msg_len];
        if stdin.read_exact(&mut msg_buf).is_err() {
            break;
        }

        // 3. Forward to Main App
        let response = forward_fn(&msg_buf);

        // 4. Send response back to browser via stdout
        let resp_bytes = response.as_bytes();
        let resp_len = (resp_bytes.len() as u32).to_ne_bytes();
        let _ = stdout.write_all(&resp_len);
        let _ = stdout.write_all(resp_bytes);
        let _ = stdout.flush();
    }
}

pub fn forward_to_pipe<P: Read + Write>(mut pipe: P, msg_bytes: &[u8]) -> Result<String, String> {
    let len_bytes = (msg_bytes.len() as u32).to_le_bytes();
    pipe.write_all(&len_bytes).map_err(|e| e.to_string())?;
    pipe.write_all(msg_bytes).map_err(|e| e.to_string())?;

    let mut resp_len_buf = [0u8; 4];
    pipe.read_exact(&mut resp_len_buf).map_err(|e| e.to_string())?;
    let r_len = u32::from_le_bytes(resp_len_buf) as usize;
    let mut r_buf = vec![0u8; r_len];
    pipe.read_exact(&mut r_buf).map_err(|e| e.to_string())?;
    Ok(String::from_utf8(r_buf).unwrap_or_else(|_| r#"{"status":"ok"}"#.to_string()))
}

pub fn forward_to_main_app(msg_bytes: &[u8]) -> String {
    #[cfg(target_os = "windows")]
    {
        use std::fs::OpenOptions;

        // Try opening pipe
        if let Ok(pipe) = OpenOptions::new()
            .read(true)
            .write(true)
            .open(crate::ipc_server::PIPE_NAME)
        {
            if let Ok(resp) = forward_to_pipe(pipe, msg_bytes) {
                return resp;
            }
        }
    }

    r#"{"status":"error","message":"My Own IDM application is not currently running"}"#.to_string()
}


pub fn build_chrome_manifest(exe_path: &Path) -> serde_json::Value {
    serde_json::json!({
        "name": HOST_NAME,
        "description": "My Own IDM Native Messaging Host for Chromium",
        "path": exe_path.to_string_lossy(),
        "type": "stdio",
        "allowed_origins": [
            "chrome-extension://*/"
        ]
    })
}

pub fn build_firefox_manifest(exe_path: &Path) -> serde_json::Value {
    serde_json::json!({
        "name": HOST_NAME,
        "description": "My Own IDM Native Messaging Host for Firefox",
        "path": exe_path.to_string_lossy(),
        "type": "stdio",
        "allowed_extensions": [
            "myownidm@extension"
        ]
    })
}

pub fn write_manifest_files(manifest_dir: &Path, exe_path: &Path) -> Result<(std::path::PathBuf, std::path::PathBuf), String> {
    // 1. Chrome / Edge Manifest
    let chrome_manifest_path = manifest_dir.join("com.myownidm.host.chrome.json");
    let chrome_manifest = build_chrome_manifest(exe_path);
    std::fs::write(&chrome_manifest_path, chrome_manifest.to_string())
        .map_err(|e| format!("Failed to write Chrome manifest: {}", e))?;

    // 2. Firefox Manifest
    let firefox_manifest_path = manifest_dir.join("com.myownidm.host.firefox.json");
    let firefox_manifest = build_firefox_manifest(exe_path);
    std::fs::write(&firefox_manifest_path, firefox_manifest.to_string())
        .map_err(|e| format!("Failed to write Firefox manifest: {}", e))?;

    Ok((chrome_manifest_path, firefox_manifest_path))
}

pub fn register_native_messaging_manifests(exe_path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let manifest_dir = exe_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();

        let (chrome_manifest_path, firefox_manifest_path) = write_manifest_files(&manifest_dir, exe_path)?;

        // 3. Write Windows Registry keys
        use std::process::Command;

        // Chrome
        let _ = Command::new("reg")
            .args([
                "add",
                &format!(r"HKCU\Software\Google\Chrome\NativeMessagingHosts\{}", HOST_NAME),
                "/ve",
                "/t",
                "REG_SZ",
                "/d",
                &chrome_manifest_path.to_string_lossy(),
                "/f",
            ])
            .output();

        // Edge
        let _ = Command::new("reg")
            .args([
                "add",
                &format!(r"HKCU\Software\Microsoft\Edge\NativeMessagingHosts\{}", HOST_NAME),
                "/ve",
                "/t",
                "REG_SZ",
                "/d",
                &chrome_manifest_path.to_string_lossy(),
                "/f",
            ])
            .output();

        // Firefox
        let _ = Command::new("reg")
            .args([
                "add",
                &format!(r"HKCU\Software\Mozilla\NativeMessagingHosts\{}", HOST_NAME),
                "/ve",
                "/t",
                "REG_SZ",
                "/d",
                &firefox_manifest_path.to_string_lossy(),
                "/f",
            ])
            .output();

        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_chrome_manifest() {
        let path = Path::new("C:\\Program Files\\MyOwnIDM\\idm.exe");
        let manifest = build_chrome_manifest(path);

        assert_eq!(manifest["name"], HOST_NAME);
        assert_eq!(manifest["type"], "stdio");
        assert_eq!(manifest["path"], "C:\\Program Files\\MyOwnIDM\\idm.exe");
        let origins = manifest["allowed_origins"].as_array().expect("origins array");
        assert!(origins.iter().any(|o| o == "chrome-extension://*/"));
    }

    #[test]
    fn test_build_firefox_manifest() {
        let path = Path::new("C:\\Program Files\\MyOwnIDM\\idm.exe");
        let manifest = build_firefox_manifest(path);

        assert_eq!(manifest["name"], HOST_NAME);
        assert_eq!(manifest["type"], "stdio");
        assert_eq!(manifest["path"], "C:\\Program Files\\MyOwnIDM\\idm.exe");
        let extensions = manifest["allowed_extensions"].as_array().expect("extensions array");
        assert!(extensions.iter().any(|e| e == "myownidm@extension"));
    }

    #[test]
    fn test_write_manifest_files() {
        let temp = tempfile::tempdir().expect("temp dir");
        let exe = temp.path().join("idm.exe");
        let res = write_manifest_files(temp.path(), &exe);
        assert!(res.is_ok());

        let (chrome_path, firefox_path) = res.unwrap();
        assert!(chrome_path.exists());
        assert!(firefox_path.exists());

        let chrome_content = std::fs::read_to_string(&chrome_path).unwrap();
        let val: serde_json::Value = serde_json::from_str(&chrome_content).unwrap();
        assert_eq!(val["name"], HOST_NAME);
    }

    #[test]
    fn test_native_messaging_loop_success() {
        let msg = r#"{"url":"https://example.com/test.mp4"}"#;
        let mut input = Vec::new();
        let msg_len = (msg.len() as u32).to_ne_bytes();
        input.extend_from_slice(&msg_len);
        input.extend_from_slice(msg.as_bytes());

        let mut output = Vec::new();
        run_native_messaging_loop(&input[..], &mut output, |incoming| {
            assert_eq!(incoming, msg.as_bytes());
            r#"{"status":"ok"}"#.to_string()
        });

        assert!(output.len() >= 4);
        let resp_len = u32::from_ne_bytes([output[0], output[1], output[2], output[3]]) as usize;
        let resp_body = std::str::from_utf8(&output[4..4 + resp_len]).unwrap();
        assert_eq!(resp_body, r#"{"status":"ok"}"#);
    }

    #[test]
    fn test_native_messaging_loop_zero_length() {
        let input = [0u8; 4];
        let mut output = Vec::new();
        run_native_messaging_loop(&input[..], &mut output, |_| "ignored".to_string());
        assert!(output.is_empty());
    }

    #[test]
    fn test_native_messaging_loop_oversized() {
        let len = (15 * 1024 * 1024u32).to_ne_bytes();
        let mut output = Vec::new();
        run_native_messaging_loop(&len[..], &mut output, |_| "ignored".to_string());
        assert!(output.is_empty());
    }

    #[test]
    fn test_forward_to_main_app_closed_pipe() {
        let resp = forward_to_main_app(b"{}");
        assert!(resp.contains("status"));
    }

    #[test]
    fn test_forward_to_pipe_mock() {
        struct MockPipe {
            incoming_buf: Vec<u8>,
            outgoing_data: std::io::Cursor<Vec<u8>>,
        }

        impl std::io::Read for MockPipe {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                self.outgoing_data.read(buf)
            }
        }

        impl std::io::Write for MockPipe {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.incoming_buf.extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let resp_payload = r#"{"status":"ok"}"#;
        let resp_len = (resp_payload.len() as u32).to_le_bytes();
        let mut outgoing = Vec::new();
        outgoing.extend_from_slice(&resp_len);
        outgoing.extend_from_slice(resp_payload.as_bytes());

        let pipe = MockPipe {
            incoming_buf: Vec::new(),
            outgoing_data: std::io::Cursor::new(outgoing),
        };

        let res = forward_to_pipe(pipe, b"{\"test\":true}");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), resp_payload);
    }
}

