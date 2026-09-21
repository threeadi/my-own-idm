use reqwest::header::{HeaderMap, HeaderValue, ACCEPT_RANGES, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE, RANGE, USER_AGENT};
use std::collections::HashMap;
use std::path::Path;
use crate::engine::types::{DownloadCategory, ProbeResult, format_bytes};

pub struct Prober;

impl Prober {
    pub async fn probe(
        client: &reqwest::Client,
        url: &str,
        custom_headers: Option<HashMap<String, String>>,
    ) -> Result<ProbeResult, String> {
        let mut req_headers = HeaderMap::new();
        req_headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36"),
        );

        if let Some(ref headers) = custom_headers {
            for (k, v) in headers {
                if let (Ok(hk), Ok(hv)) = (k.parse::<reqwest::header::HeaderName>(), HeaderValue::from_str(v)) {
                    req_headers.insert(hk, hv);
                }
            }
        }

        let cleaned_url = clean_stream_url(url);
        let url = cleaned_url.as_str();
        crate::log_info!("probe", "Probing URL: {}", url);

        let effective_url = if url.contains("googlevideo.com") {
            custom_headers
                .as_ref()
                .and_then(|h| h.get("referer").or(h.get("Referer")))
                .map(|s| s.as_str())
                .unwrap_or(url)
        } else {
            url
        };

        // If YouTube URL, try yt-dlp first
        if effective_url.contains("youtube.com") || effective_url.contains("youtu.be") {
            if let Ok(mut yt_probe) = crate::engine::ytdlp::YtDlpRunner::probe(effective_url).await {
                yt_probe.url = effective_url.to_string();
                crate::log_info!("probe", "YouTube stream probed via yt-dlp: filename='{}', size={:?}", yt_probe.filename, yt_probe.total_bytes);
                return Ok(yt_probe);
            }
        }

        // Try Range: bytes=0-0 first to get content length and test range support in a single request!
        let mut test_headers = req_headers.clone();
        test_headers.insert(RANGE, HeaderValue::from_static("bytes=0-0"));

        let res = match client.get(url).headers(test_headers).send().await {
            Ok(r) => r,
            Err(e) => {
                // If direct connection failed, check if yt-dlp can handle it
                if let Ok(yt_probe) = crate::engine::ytdlp::YtDlpRunner::probe(url).await {
                    return Ok(yt_probe);
                }
                return Err(format!("Failed to connect to URL: {}", e));
            }
        };

        let status = res.status();
        let headers = res.headers().clone();

        let mut supports_range = false;
        let mut total_bytes: Option<u64> = None;

        if status.as_u16() == 206 {
            // Partial Content returned! Range is 100% supported!
            supports_range = true;
            if let Some(cr) = headers.get("content-range").and_then(|v| v.to_str().ok()) {
                // Format: "bytes 0-0/1234567"
                if let Some(total_str) = cr.rsplit('/').next() {
                    if let Ok(tot) = total_str.trim().parse::<u64>() {
                        total_bytes = Some(tot);
                    }
                }
            }
        } else if status.is_success() {
            // 200 OK returned
            if let Some(ar) = headers.get(ACCEPT_RANGES).and_then(|v| v.to_str().ok()) {
                if ar.to_ascii_lowercase().contains("bytes") {
                    supports_range = true;
                }
            }
            if let Some(cl) = headers.get(CONTENT_LENGTH).and_then(|v| v.to_str().ok()) {
                if let Ok(len) = cl.parse::<u64>() {
                    total_bytes = Some(len);
                }
            }
        }

        // Detect if it's HLS stream
        let mut is_hls = false;
        if let Some(ct) = headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()) {
            let ct_lower = ct.to_ascii_lowercase();
            if ct_lower.contains("mpegurl") || ct_lower.contains("m3u8") {
                is_hls = true;
            }
        }
        let url_lower = url.to_ascii_lowercase();
        if url_lower.contains(".m3u8")
            || url_lower.contains("/hls/")
            || url_lower.contains("/hls3/")
            || url_lower.contains(".urlset")
            || url_lower.ends_with("master.txt")
        {
            is_hls = true;
        }

        // Calculate HLS duration if HLS and total_bytes is None
        let mut hls_duration_secs: Option<f64> = None;
        if is_hls && total_bytes.is_none() {
            if let Ok(resp) = client.get(url).headers(req_headers.clone()).send().await {
                if let Ok(body) = resp.text().await {
                    if body.contains("#EXTM3U") {
                        is_hls = true;
                        let dur = Self::calculate_hls_duration(&body, url, client, &req_headers).await;
                        if dur > 0.0 {
                            hls_duration_secs = Some(dur);
                        }
                    }
                }
            }
        }

        // Extract filename
        let mut filename = String::new();
        if let Some(cd) = headers.get(CONTENT_DISPOSITION).and_then(|v| v.to_str().ok()) {
            if let Some(name) = Self::extract_filename_from_content_disposition(cd) {
                filename = name;
            }
        }

        if filename.is_empty() {
            filename = Self::extract_filename_from_url(url);
        }

        if filename.is_empty() {
            filename = if is_hls {
                "stream_video.mp4".to_string()
            } else {
                "download.bin".to_string()
            };
        }

        // If HLS, make sure extension is .mp4
        if is_hls && !filename.ends_with(".mp4") {
            filename = format!("{}.mp4", filename.trim_end_matches(".m3u8"));
        }

        filename = sanitize_filename_with_ext(&filename, if is_hls { "mp4" } else { "bin" });

        let category = DownloadCategory::from_filename(&filename);
        let formatted_size = match total_bytes {
            Some(b) => format_bytes(b),
            None => {
                if let Some(dur) = hls_duration_secs {
                    format!("{} (HLS)", Self::format_duration(dur))
                } else {
                    "Unknown size".to_string()
                }
            }
        };

        let default_download_dir = dirs::download_dir()
            .unwrap_or_else(|| Path::new("C:\\Downloads").to_path_buf())
            .join(category.default_subfolder())
            .to_string_lossy()
            .to_string();

        crate::log_info!(
            "probe",
            "Probe success: filename='{}', size={:?}, range={}, is_hls={}",
            filename, total_bytes, supports_range, is_hls
        );

        Ok(ProbeResult {
            url: url.to_string(),
            filename,
            total_bytes,
            formatted_size,
            supports_range,
            category,
            suggested_dir: default_download_dir,
            is_hls,
        })
    }

    fn extract_filename_from_content_disposition(cd: &str) -> Option<String> {
        for part in cd.split(';') {
            let trimmed = part.trim();
            if let Some(rest) = trimmed.strip_prefix("filename*=") {
                let cleaned = rest.trim_matches('"');
                if let Some(idx) = cleaned.rfind("''") {
                    let encoded = &cleaned[idx + 2..];
                    if let Ok(decoded) = urlencoding_decode(encoded) {
                        return Some(decoded);
                    }
                }
            } else if let Some(rest) = trimmed.strip_prefix("filename=") {
                let cleaned = rest.trim_matches('"').trim_matches('\'');
                if !cleaned.is_empty() {
                    return Some(cleaned.to_string());
                }
            }
        }
        None
    }

    fn extract_filename_from_url(url: &str) -> String {
        let base = url.split('?').next().unwrap_or(url);
        let base = base.split('#').next().unwrap_or(base);
        let segment = base.trim_end_matches('/').rsplit('/').next().unwrap_or("");
        if let Ok(decoded) = urlencoding_decode(segment) {
            decoded
        } else {
            segment.to_string()
        }
    }

    pub async fn calculate_hls_duration(
        body: &str,
        base_url_str: &str,
        client: &reqwest::Client,
        headers: &HeaderMap,
    ) -> f64 {
        let dur = Self::sum_extinf(body);
        if dur > 0.0 {
            return dur;
        }

        if let Ok(base_url) = reqwest::Url::parse(base_url_str) {
            let mut sub_urls = Vec::new();
            for line in body.lines() {
                let trimmed = line.trim();
                if !trimmed.starts_with('#') && !trimmed.is_empty() {
                    if let Ok(u) = base_url.join(trimmed) {
                        sub_urls.push(u.to_string());
                    }
                }
            }

            if let Some(first_sub) = sub_urls.first() {
                if let Ok(resp) = client.get(first_sub).headers(headers.clone()).send().await {
                    if let Ok(sub_body) = resp.text().await {
                        return Self::sum_extinf(&sub_body);
                    }
                }
            }
        }

        0.0
    }

    pub fn sum_extinf(body: &str) -> f64 {
        let mut total = 0.0;
        for line in body.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("#EXTINF:") {
                if let Some(num_str) = rest.split(',').next() {
                    if let Ok(d) = num_str.trim().parse::<f64>() {
                        total += d;
                    }
                }
            }
        }
        total
    }

    pub fn format_duration(seconds: f64) -> String {
        let total = seconds.round() as u64;
        let h = total / 3600;
        let m = (total % 3600) / 60;
        let s = total % 60;
        if h > 0 {
            format!("{:02}:{:02}:{:02}", h, m, s)
        } else {
            format!("{:02}:{:02}", m, s)
        }
    }
}

pub fn clean_stream_url(url: &str) -> String {
    if !url.contains("bytestart") && !url.contains("byteend") {
        return url.to_string();
    }
    if let Ok(mut parsed) = reqwest::Url::parse(url) {
        let filtered: Vec<(String, String)> = parsed
            .query_pairs()
            .filter(|(k, _)| k != "bytestart" && k != "byteend")
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        parsed.query_pairs_mut().clear().extend_pairs(filtered.iter().map(|(k, v)| (k.as_str(), v.as_str())));
        parsed.to_string()
    } else {
        url.to_string()
    }
}

pub fn sanitize_filename_with_ext(full_name: &str, default_ext: &str) -> String {
    let clean_full = full_name.trim().trim_end_matches('.').trim();
    let p = Path::new(clean_full);
    let raw_ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
    let sanitized_ext: String = raw_ext
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(10)
        .collect();

    let ext = if sanitized_ext.is_empty() {
        let def_clean: String = default_ext
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .take(10)
            .collect();
        if def_clean.is_empty() { "bin".to_string() } else { def_clean }
    } else {
        sanitized_ext
    };

    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or(clean_full);
    let clean_stem = sanitize_filename(stem);
    format!("{}.{}", clean_stem, ext)
}

pub fn sanitize_filename(name: &str) -> String {
    let clean = name
        .replace("\r", " ")
        .replace("\n", " ")
        .replace("\t", " ")
        .replace('—', "-")
        .replace('–', "-")
        .replace('’', "'")
        .replace('‘', "'")
        .replace('“', "_")
        .replace('”', "_")
        .chars()
        .filter(|c| !c.is_control() && (*c == ' ' || *c == '-' || *c == '_' || *c == '.' || *c == '\'' || *c == '(' || *c == ')' || *c == '[' || *c == ']' || c.is_alphanumeric()))
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => c,
        })
        .collect::<String>();

    let trimmed = clean.trim().trim_matches('.').trim();

    // Truncate to maximum 60 chars to avoid Windows MAX_PATH (os error 123)
    let truncated: String = trimmed.chars().take(60).collect();
    let result = truncated.trim().trim_matches('.').trim();
    if result.is_empty() {
        "download".to_string()
    } else {
        result.to_string()
    }
}

fn urlencoding_decode(s: &str) -> Result<String, ()> {
    let mut bytes = Vec::new();
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let h1 = chars.next().ok_or(())?;
            let h2 = chars.next().ok_or(())?;
            let hex_slice = [h1, h2];
            let hex_str = std::str::from_utf8(&hex_slice).map_err(|_| ())?;
            let byte = u8::from_str_radix(hex_str, 16).map_err(|_| ())?;
            bytes.push(byte);
        } else if b == b'+' {
            bytes.push(b' ');
        } else {
            bytes.push(b);
        }
    }
    String::from_utf8(bytes).map_err(|_| ())
}

mod dirs {
    use std::path::PathBuf;
    pub fn download_dir() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            std::env::var_os("USERPROFILE")
                .map(PathBuf::from)
                .map(|p| p.join("Downloads"))
        }
        #[cfg(not(target_os = "windows"))]
        {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|p| p.join("Downloads"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_stream_url() {
        // Simple case with both parameters
        let url = "https://instagram.fsub15-1.fna.fbcdn.net/video.mp4?bytestart=0&byteend=817";
        let cleaned = clean_stream_url(url);
        assert!(!cleaned.contains("bytestart"));
        assert!(!cleaned.contains("byteend"));

        // Preserves other parameters
        let url2 = "https://example.com/stream.mp4?id=123&bytestart=0&token=abc&byteend=999&quality=hd";
        let cleaned2 = clean_stream_url(url2);
        assert!(!cleaned2.contains("bytestart"));
        assert!(!cleaned2.contains("byteend"));
        assert!(cleaned2.contains("id=123"));
        assert!(cleaned2.contains("token=abc"));
        assert!(cleaned2.contains("quality=hd"));

        // No bytestart/byteend returns unchanged
        let url3 = "https://example.com/video.mp4?foo=bar";
        assert_eq!(clean_stream_url(url3), url3);

        // URL without query
        let url4 = "https://example.com/video.mp4";
        assert_eq!(clean_stream_url(url4), url4);
    }

    #[test]
    fn test_sanitize_filename() {
        // Forbidden characters replaced with '_'
        let input = r#"test<file>:name"with/slashes\and|pipes?and*stars"#;
        let sanitized = sanitize_filename(input);
        assert!(!sanitized.contains('<'));
        assert!(!sanitized.contains('>'));
        assert!(!sanitized.contains(':'));
        assert!(!sanitized.contains('"'));
        assert!(!sanitized.contains('/'));
        assert!(!sanitized.contains('\\'));
        assert!(!sanitized.contains('|'));
        assert!(!sanitized.contains('?'));
        assert!(!sanitized.contains('*'));

        // Normalizing dashes and quotes
        let fancy = "My—Video–Title’s ‘Awesome’";
        let sanitized_fancy = sanitize_filename(fancy);
        assert!(sanitized_fancy.contains('-'));
        assert!(sanitized_fancy.contains('\''));
        assert!(!sanitized_fancy.contains('—'));
        assert!(!sanitized_fancy.contains('–'));

        // Long filename truncation (max 60 chars)
        let long_name = "a".repeat(120);
        let sanitized_long = sanitize_filename(&long_name);
        assert!(sanitized_long.len() <= 60);

        // Empty or whitespace fallback
        assert_eq!(sanitize_filename("   "), "download");
        assert_eq!(sanitize_filename("..."), "download");
    }

    #[test]
    fn test_sanitize_filename_with_ext() {
        // Normal filename and extension
        let res = sanitize_filename_with_ext("my_video.mp4", "mp4");
        assert_eq!(res, "my_video.mp4");

        // Filename without extension
        let res2 = sanitize_filename_with_ext("my_video", "mp4");
        assert_eq!(res2, "my_video.mp4");

        // Filename with trailing dots
        let res3 = sanitize_filename_with_ext("my_video.", "zip");
        assert_eq!(res3, "my_video.zip");

        // Fallback when extension is empty
        let res4 = sanitize_filename_with_ext("file", "");
        assert_eq!(res4, "file.bin");
    }

    #[test]
    fn test_urlencoding_decode() {
        assert_eq!(urlencoding_decode("hello%20world").unwrap(), "hello world");
        assert_eq!(urlencoding_decode("hello+world").unwrap(), "hello world");
        assert_eq!(urlencoding_decode("plain_text").unwrap(), "plain_text");
        assert_eq!(urlencoding_decode("file%2Fname").unwrap(), "file/name");
        assert!(urlencoding_decode("%ZZinvalid").is_err());
    }

    #[test]
    fn test_extract_filename_from_content_disposition() {
        // Standard filename
        let cd1 = "attachment; filename=\"archive.tar.gz\"";
        assert_eq!(
            Prober::extract_filename_from_content_disposition(cd1),
            Some("archive.tar.gz".to_string())
        );

        // Single quotes
        let cd2 = "inline; filename='document.pdf'";
        assert_eq!(
            Prober::extract_filename_from_content_disposition(cd2),
            Some("document.pdf".to_string())
        );

        // RFC 5987 encoded
        let cd3 = "attachment; filename*=UTF-8''my%20test%20video.mp4";
        assert_eq!(
            Prober::extract_filename_from_content_disposition(cd3),
            Some("my test video.mp4".to_string())
        );

        // No filename
        let cd4 = "attachment; size=1024";
        assert_eq!(Prober::extract_filename_from_content_disposition(cd4), None);
    }

    #[test]
    fn test_extract_filename_from_url() {
        let url1 = "https://example.com/downloads/setup.exe?token=123#hash";
        assert_eq!(Prober::extract_filename_from_url(url1), "setup.exe");

        let url2 = "https://example.com/media/encoded%20video%20file.mp4";
        assert_eq!(Prober::extract_filename_from_url(url2), "encoded video file.mp4");

        let url3 = "https://example.com/trailing/slash/";
        assert_eq!(Prober::extract_filename_from_url(url3), "slash");
    }

    #[tokio::test]
    async fn test_probe_partial_content_206() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await;
            let resp = "HTTP/1.1 206 Partial Content\r\n\
Content-Range: bytes 0-0/5000000\r\n\
Content-Disposition: attachment; filename=\"archive.zip\"\r\n\
Content-Type: application/zip\r\n\
Content-Length: 1\r\n\r\nX";
            let _ = socket.write_all(resp.as_bytes()).await;
        });

        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{}/download", port);
        let res = Prober::probe(&client, &url, None).await.expect("probe should succeed");

        assert_eq!(res.filename, "archive.zip");
        assert_eq!(res.total_bytes, Some(5000000));
        assert!(res.supports_range);
        assert_eq!(res.category, DownloadCategory::Compressed);
    }

    #[tokio::test]
    async fn test_probe_ok_200() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await;
            let resp = "HTTP/1.1 200 OK\r\n\
Accept-Ranges: bytes\r\n\
Content-Length: 1024\r\n\
Content-Type: application/pdf\r\n\r\n";
            let _ = socket.write_all(resp.as_bytes()).await;
        });

        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{}/doc.pdf", port);
        let res = Prober::probe(&client, &url, None).await.expect("probe 200 ok");

        assert_eq!(res.filename, "doc.pdf");
        assert_eq!(res.total_bytes, Some(1024));
        assert!(res.supports_range);
        assert_eq!(res.category, DownloadCategory::Documents);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(Prober::format_duration(45.0), "00:45");
        assert_eq!(Prober::format_duration(932.0), "15:32");
        assert_eq!(Prober::format_duration(3665.0), "01:01:05");
    }

    #[test]
    fn test_sum_extinf() {
        let playlist = "#EXTM3U\n#EXT-X-VERSION:3\n#EXTINF:10.500,\nseg1.ts\n#EXTINF:9.500,\nseg2.ts\n#EXTINF:12.0,\nseg3.ts\n#EXT-X-ENDLIST\n";
        let total = Prober::sum_extinf(playlist);
        assert!((total - 32.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_calculate_hls_duration_direct_and_master() {
        let client = reqwest::Client::new();
        let headers = reqwest::header::HeaderMap::new();

        // Direct media playlist
        let media_pl = "#EXTM3U\n#EXTINF:10.0,\nseg1.ts\n#EXTINF:20.0,\nseg2.ts\n";
        let dur = Prober::calculate_hls_duration(media_pl, "http://example.com/stream.m3u8", &client, &headers).await;
        assert_eq!(dur, 30.0);
    }
}
