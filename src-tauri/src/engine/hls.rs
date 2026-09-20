use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use cbc::Decryptor;
type Aes128CbcDec = Decryptor<aes::Aes128>;

use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct HlsKey {
    pub method: String,
    pub uri: String,
    pub iv: Option<[u8; 16]>,
}

#[derive(Debug, Clone)]
pub struct HlsSegment {
    pub index: usize,
    pub uri: String,
    pub duration: f64,
    pub key: Option<HlsKey>,
}

pub struct HlsDownloader;

impl HlsDownloader {
    pub async fn run(
        client: reqwest::Client,
        m3u8_url: String,
        output_file_path: String,
        cancel_flag: Arc<AtomicBool>,
        progress_tx: Sender<(usize, u64)>,
        custom_headers: Option<HashMap<String, String>>,
    ) -> Result<(), String> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36"),
        );

        if let Some(ch) = &custom_headers {
            for (k, v) in ch {
                if let (Ok(hk), Ok(hv)) = (k.parse::<reqwest::header::HeaderName>(), HeaderValue::from_str(v)) {
                    headers.insert(hk, hv);
                }
            }
        }

        // 1. Fetch playlist content
        let res = client
            .get(&m3u8_url)
            .headers(headers.clone())
            .send()
            .await
            .map_err(|e| format!("Failed to fetch HLS playlist: {}", e))?;

        let playlist_text = res
            .text()
            .await
            .map_err(|e| format!("Failed to read playlist body: {}", e))?;

        // 2. Parse segments & keys
        let base_url = reqwest::Url::parse(&m3u8_url).map_err(|e| e.to_string())?;
        let segments = parse_m3u8(&playlist_text, &base_url)?;

        if segments.is_empty() {
            return Err("No video segments found in HLS playlist".to_string());
        }

        let out_path = Path::new(&output_file_path);
        if let Some(p) = out_path.parent() {
            let _ = tokio::fs::create_dir_all(p).await;
        }

        let mut out_file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(out_path)
            .await
            .map_err(|e| format!("Failed to open output file: {}", e))?;

        // Cache keys so we don't re-download the same key 1000 times
        let mut key_cache: HashMap<String, [u8; 16]> = HashMap::new();

        // 3. Download and decrypt segments sequentially to ensure seamless video assembly
        for (i, seg) in segments.iter().enumerate() {
            if cancel_flag.load(Ordering::Relaxed) {
                return Ok(());
            }

            // Download segment
            let seg_res = client
                .get(&seg.uri)
                .headers(headers.clone())
                .send()
                .await
                .map_err(|e| format!("Failed to fetch segment #{}: {}", i, e))?;

            let raw_data = seg_res
                .bytes()
                .await
                .map_err(|e| format!("Failed to read segment #{} bytes: {}", i, e))?;

            let mut final_data = raw_data.to_vec();

            // Decrypt if AES-128
            if let Some(key_info) = &seg.key {
                if key_info.method.to_uppercase() == "AES-128" {
                    let key_bytes = match key_cache.get(&key_info.uri) {
                        Some(k) => *k,
                        None => {
                            let k_res = client
                                .get(&key_info.uri)
                                .headers(headers.clone())
                                .send()
                                .await
                                .map_err(|e| format!("Failed to fetch AES key from {}: {}", key_info.uri, e))?;
                            let k_data = k_res.bytes().await.map_err(|e| e.to_string())?;
                            if k_data.len() != 16 {
                                return Err(format!("Invalid AES-128 key length: {} bytes", k_data.len()));
                            }
                            let mut k_arr = [0u8; 16];
                            k_arr.copy_from_slice(&k_data);
                            key_cache.insert(key_info.uri.clone(), k_arr);
                            k_arr
                        }
                    };

                    let iv = key_info.iv.unwrap_or_else(|| {
                        // Default IV: sequence number in big-endian
                        let mut iv_arr = [0u8; 16];
                        let seq = (i + 1) as u128;
                        iv_arr.copy_from_slice(&seq.to_be_bytes());
                        iv_arr
                    });

                    // Decrypt AES-128-CBC
                    if let Ok(dec) = Aes128CbcDec::new_from_slices(&key_bytes, &iv) {
                        if let Ok(decrypted) = dec.decrypt_padded_vec_mut::<Pkcs7>(&final_data) {
                            final_data = decrypted;
                        }
                    }
                }
            }

            let len = final_data.len() as u64;
            out_file
                .write_all(&final_data)
                .await
                .map_err(|e| format!("Failed to write segment #{}: {}", i, e))?;

            let _ = progress_tx.send((0, len)).await;
        }

        out_file.flush().await.map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn parse_m3u8(text: &str, base_url: &reqwest::Url) -> Result<Vec<HlsSegment>, String> {
    let mut segments = Vec::new();
    let mut current_key: Option<HlsKey> = None;
    let mut current_duration = 0.0;
    let mut segment_index = 0;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("#EXT-X-KEY:") {
            current_key = parse_key_tag(trimmed, base_url);
        } else if trimmed.starts_with("#EXTINF:") {
            if let Some(dur_str) = trimmed.strip_prefix("#EXTINF:").and_then(|s| s.split(',').next()) {
                current_duration = dur_str.trim().parse::<f64>().unwrap_or(0.0);
            }
        } else if !trimmed.starts_with('#') {
            // Segment URI
            let seg_url = match base_url.join(trimmed) {
                Ok(u) => u.to_string(),
                Err(_) => trimmed.to_string(),
            };

            segments.push(HlsSegment {
                index: segment_index,
                uri: seg_url,
                duration: current_duration,
                key: current_key.clone(),
            });
            segment_index += 1;
        }
    }

    Ok(segments)
}

fn parse_key_tag(tag: &str, base_url: &reqwest::Url) -> Option<HlsKey> {
    let content = tag.strip_prefix("#EXT-X-KEY:")?;
    let mut method = String::new();
    let mut uri = String::new();
    let mut iv: Option<[u8; 16]> = None;

    for attr in content.split(',') {
        let parts: Vec<&str> = attr.splitn(2, '=').collect();
        if parts.len() == 2 {
            let key = parts[0].trim().to_uppercase();
            let val = parts[1].trim().trim_matches('"');
            match key.as_str() {
                "METHOD" => method = val.to_string(),
                "URI" => {
                    uri = match base_url.join(val) {
                        Ok(u) => u.to_string(),
                        Err(_) => val.to_string(),
                    };
                }
                "IV" => {
                    let clean_hex = val.strip_prefix("0x").or_else(|| val.strip_prefix("0X")).unwrap_or(val);
                    if let Ok(bytes) = hex_to_bytes(clean_hex) {
                        if bytes.len() == 16 {
                            let mut arr = [0u8; 16];
                            arr.copy_from_slice(&bytes);
                            iv = Some(arr);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Some(HlsKey { method, uri, iv })
}

fn hex_to_bytes(s: &str) -> Result<Vec<u8>, ()> {
    let mut bytes = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    for i in (0..chars.len()).step_by(2) {
        if i + 1 < chars.len() {
            let pair: String = [chars[i], chars[i + 1]].iter().collect();
            let b = u8::from_str_radix(&pair, 16).map_err(|_| ())?;
            bytes.push(b);
        }
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes::cipher::BlockEncryptMut;
    type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;

    #[test]
    fn test_parse_m3u8_basic() {
        let playlist = r#"#EXTM3U
#EXT-X-VERSION:3
#EXT-X-TARGETDURATION:10
#EXTINF:9.009,
segment_0.ts
#EXTINF:8.500,
segment_1.ts
#EXT-X-ENDLIST
"#;
        let base_url = reqwest::Url::parse("https://stream.example.com/live/playlist.m3u8").unwrap();
        let segments = parse_m3u8(playlist, &base_url).expect("parse segments");

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].index, 0);
        assert_eq!(segments[0].duration, 9.009);
        assert_eq!(segments[0].uri, "https://stream.example.com/live/segment_0.ts");
        assert!(segments[0].key.is_none());

        assert_eq!(segments[1].index, 1);
        assert_eq!(segments[1].duration, 8.5);
        assert_eq!(segments[1].uri, "https://stream.example.com/live/segment_1.ts");
    }

    #[test]
    fn test_parse_m3u8_with_encryption() {
        let playlist = r#"#EXTM3U
#EXT-X-VERSION:3
#EXT-X-KEY:METHOD=AES-128,URI="enc.key",IV=0x000102030405060708090A0B0C0D0E0F
#EXTINF:10.0,
video0.ts
#EXTINF:10.0,
video1.ts
#EXT-X-ENDLIST
"#;
        let base_url = reqwest::Url::parse("https://cdn.example.com/hls/master.m3u8").unwrap();
        let segments = parse_m3u8(playlist, &base_url).expect("parse segments");

        assert_eq!(segments.len(), 2);
        let key = segments[0].key.as_ref().expect("segment 0 key");
        assert_eq!(key.method, "AES-128");
        assert_eq!(key.uri, "https://cdn.example.com/hls/enc.key");

        let expected_iv = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
            0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
        ];
        assert_eq!(key.iv, Some(expected_iv));
    }

    #[test]
    fn test_hex_to_bytes() {
        assert_eq!(
            hex_to_bytes("0001020304050607").unwrap(),
            vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]
        );
        assert_eq!(hex_to_bytes("deadbeef").unwrap(), vec![0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(hex_to_bytes("A1B2").unwrap(), vec![0xa1, 0xb2]);
        assert!(hex_to_bytes("invalid_hex").is_err());
    }

    #[test]
    fn test_aes_128_cbc_roundtrip() {
        let key = [0x42u8; 16];
        let iv = [0x24u8; 16];
        let plaintext = b"Hello, this is a secret video chunk for My Own IDM!";

        // Encrypt with PKCS7
        let enc = Aes128CbcEnc::new_from_slices(&key, &iv).unwrap();
        let ciphertext = enc.encrypt_padded_vec_mut::<Pkcs7>(plaintext);

        // Decrypt
        let dec = Aes128CbcDec::new_from_slices(&key, &iv).unwrap();
        let to_decrypt = ciphertext;
        let decrypted = dec.decrypt_padded_vec_mut::<Pkcs7>(&to_decrypt).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[tokio::test]
    async fn test_hls_downloader_run_mock() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let mut buf = [0u8; 1024];
                    let n = socket.read(&mut buf).await.unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]);

                    if req.contains("GET /playlist.m3u8") {
                        let body = "#EXTM3U\n#EXT-X-VERSION:3\n#EXTINF:2.0,\nseg0.ts\n#EXTINF:2.0,\nseg1.ts\n#EXT-X-ENDLIST\n";
                        let resp = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/vnd.apple.mpegurl\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            body.len(),
                            body
                        );
                        let _ = socket.write_all(resp.as_bytes()).await;
                    } else if req.contains("GET /seg0.ts") {
                        let data = b"SEGMENT0_BYTES_";
                        let resp = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: video/MP2T\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            data.len()
                        );
                        let _ = socket.write_all(resp.as_bytes()).await;
                        let _ = socket.write_all(data).await;
                    } else if req.contains("GET /seg1.ts") {
                        let data = b"SEGMENT1_BYTES";
                        let resp = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: video/MP2T\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            data.len()
                        );
                        let _ = socket.write_all(resp.as_bytes()).await;
                        let _ = socket.write_all(data).await;
                    }
                    let _ = socket.flush().await;
                });
            }
        });

        let client = reqwest::Client::new();
        let m3u8_url = format!("http://{}/playlist.m3u8", addr);
        let temp = tempfile::tempdir().unwrap();
        let out_path = temp.path().join("video.mp4").to_string_lossy().to_string();
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, mut rx) = tokio::sync::mpsc::channel(10);

        let res = HlsDownloader::run(client, m3u8_url, out_path.clone(), cancel, tx, None).await;
        assert!(res.is_ok());

        let mut progress_count = 0;
        while let Ok(_) = rx.try_recv() {
            progress_count += 1;
        }
        assert!(progress_count >= 2);

        let file_bytes = tokio::fs::read(&out_path).await.unwrap();
        assert_eq!(file_bytes, b"SEGMENT0_BYTES_SEGMENT1_BYTES");
    }

    #[tokio::test]
    async fn test_hls_downloader_empty_playlist() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                let body = "#EXTM3U\n#EXT-X-ENDLIST\n";
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(resp.as_bytes()).await;
            }
        });

        let client = reqwest::Client::new();
        let m3u8_url = format!("http://{}/playlist.m3u8", addr);
        let temp = tempfile::tempdir().unwrap();
        let out_path = temp.path().join("empty.mp4").to_string_lossy().to_string();
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, _rx) = tokio::sync::mpsc::channel(10);

        let res = HlsDownloader::run(client, m3u8_url, out_path, cancel, tx, None).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("No video segments found"));
    }
}

