use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, RANGE, USER_AGENT};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use crate::engine::limiter::TokenBucketRateLimiter;
use crate::engine::writer::WorkerHandle;

pub struct SegmentWorker {
    pub segment_index: usize,
    pub start_byte: u64,
    pub end_byte: u64,
    pub downloaded_bytes: u64,
}

impl SegmentWorker {
    pub async fn run(
        client: reqwest::Client,
        url: String,
        segment_index: usize,
        start_byte: u64,
        end_byte: u64,
        mut current_downloaded: u64,
        writer: WorkerHandle,
        cancel_flag: Arc<AtomicBool>,
        progress_tx: Sender<(usize, u64)>,
        custom_headers: Option<HashMap<String, String>>,
        task_limiter: Option<Arc<TokenBucketRateLimiter>>,
        global_limiter: Option<Arc<TokenBucketRateLimiter>>,
    ) -> Result<(), String> {
        let current_offset = start_byte + current_downloaded;
        if current_offset > end_byte {
            // Already finished
            return Ok(());
        }

        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36"),
        );

        if let Some(ch) = custom_headers {
            for (k, v) in ch {
                if let (Ok(hk), Ok(hv)) = (k.parse::<reqwest::header::HeaderName>(), HeaderValue::from_str(&v)) {
                    headers.insert(hk, hv);
                }
            }
        }

        if end_byte == u64::MAX {
            if current_offset > 0 {
                let range_val = format!("bytes={}-", current_offset);
                if let Ok(hv) = HeaderValue::from_str(&range_val) {
                    headers.insert(RANGE, hv);
                }
            }
        } else {
            let range_val = format!("bytes={}-{}", current_offset, end_byte);
            if let Ok(hv) = HeaderValue::from_str(&range_val) {
                headers.insert(RANGE, hv);
            }
        }

        let res = client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| {
                eprintln!("Worker #{} request failed: {}", segment_index, e);
                format!("Worker #{} request failed: {}", segment_index, e)
            })?;

        if !res.status().is_success() && res.status().as_u16() != 206 {
            eprintln!("Worker #{} received HTTP error {}", segment_index, res.status());
            return Err(format!(
                "Worker #{} received HTTP error {}",
                segment_index,
                res.status()
            ));
        }

        let mut stream = res.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            if cancel_flag.load(Ordering::Relaxed) {
                // Task cancelled/paused
                let _ = writer.flush().await;
                return Ok(());
            }

            let chunk = chunk_result.map_err(|e| format!("Worker #{} stream error: {}", segment_index, e))?;
            let chunk_len = chunk.len() as u64;

            if let Some(limiter) = &task_limiter {
                limiter.acquire(chunk.len()).await;
            }
            if let Some(limiter) = &global_limiter {
                limiter.acquire(chunk.len()).await;
            }

            let write_offset = start_byte + current_downloaded;
            writer
                .write_at(write_offset, &chunk)
                .await
                .map_err(|e| format!("Worker #{} write error at {}: {}", segment_index, write_offset, e))?;

            current_downloaded += chunk_len;
            let _ = progress_tx.send((segment_index, chunk_len)).await;
        }

        let _ = writer.flush().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::writer::FileWriter;
    use tempfile::tempdir;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn test_segment_worker_already_finished() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("finished.bin");
        let writer = FileWriter::create_or_open(&path.to_string_lossy(), Some(100)).await.unwrap();
        let handle = writer.open_worker_handle().await.unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, _rx) = tokio::sync::mpsc::channel(10);

        // start=0, end=50, downloaded=51 -> already finished
        let res = SegmentWorker::run(
            reqwest::Client::new(),
            "http://example.com".to_string(),
            0,
            0,
            50,
            51,
            handle,
            cancel,
            tx,
            None,
            None,
            None,
        )
        .await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_segment_worker_successful_download() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        // Spawn mock server
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut req_buf = [0u8; 1024];
            let _ = socket.read(&mut req_buf).await;

            let payload = vec![0x77u8; 50];
            let resp_header = format!(
                "HTTP/1.1 206 Partial Content\r\n\
Content-Range: bytes 0-49/50\r\n\
Content-Length: 50\r\n\
Connection: close\r\n\r\n"
            );
            let _ = socket.write_all(resp_header.as_bytes()).await;
            let _ = socket.write_all(&payload).await;
            let _ = socket.flush().await;
        });

        let dir = tempdir().unwrap();
        let file_path = dir.path().join("worker_test.bin");
        let writer = FileWriter::create_or_open(&file_path.to_string_lossy(), Some(50))
            .await
            .unwrap();
        let handle = writer.open_worker_handle().await.unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, mut rx) = tokio::sync::mpsc::channel(10);

        let url = format!("http://127.0.0.1:{}/stream", port);
        let res = SegmentWorker::run(
            reqwest::Client::new(),
            url,
            0,
            0,
            49,
            0,
            handle,
            cancel,
            tx,
            None,
            None,
            None,
        )
        .await;

        assert!(res.is_ok());

        // Check progress message
        let progress = rx.recv().await.expect("receive progress");
        assert_eq!(progress, (0, 50));

        // Check bytes on disk
        let disk_bytes = tokio::fs::read(&file_path).await.unwrap();
        assert_eq!(disk_bytes.len(), 50);
        assert_eq!(disk_bytes, vec![0x77u8; 50]);
    }

    #[tokio::test]
    async fn test_segment_worker_with_rate_limiter() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut req_buf = [0u8; 1024];
            let _ = socket.read(&mut req_buf).await;

            let payload = vec![0xAAu8; 20];
            let resp_header = format!(
                "HTTP/1.1 206 Partial Content\r\n\
Content-Range: bytes 0-19/20\r\n\
Content-Length: 20\r\n\
Connection: close\r\n\r\n"
            );
            let _ = socket.write_all(resp_header.as_bytes()).await;
            let _ = socket.write_all(&payload).await;
            let _ = socket.flush().await;
        });

        let dir = tempdir().unwrap();
        let file_path = dir.path().join("limiter_worker_test.bin");
        let writer = FileWriter::create_or_open(&file_path.to_string_lossy(), Some(20))
            .await
            .unwrap();
        let handle = writer.open_worker_handle().await.unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, mut rx) = tokio::sync::mpsc::channel(10);

        let task_limiter = Arc::new(TokenBucketRateLimiter::new(100_000));
        let global_limiter = Arc::new(TokenBucketRateLimiter::new(200_000));

        let url = format!("http://127.0.0.1:{}/stream", port);
        let res = SegmentWorker::run(
            reqwest::Client::new(),
            url,
            0,
            0,
            19,
            0,
            handle,
            cancel,
            tx,
            None,
            Some(task_limiter),
            Some(global_limiter),
        )
        .await;

        assert!(res.is_ok());
        let progress = rx.recv().await.expect("receive progress");
        assert_eq!(progress, (0, 20));
    }
}
