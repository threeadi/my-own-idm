use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::db::Database;
#[allow(unused_imports)]
use crate::engine::hls::HlsDownloader;
use crate::engine::probe::Prober;
use crate::engine::types::{
    DownloadCategory, DownloadTask, Segment, SpeedMetrics, TaskStatus,
};
use crate::engine::worker::SegmentWorker;
use crate::engine::writer::FileWriter;

pub struct DownloadManager {
    pub db: Arc<Database>,
    pub client: reqwest::Client,
    pub tasks: Arc<RwLock<HashMap<String, DownloadTask>>>,
    pub cancel_flags: Arc<RwLock<HashMap<String, Arc<AtomicBool>>>>,
}

impl DownloadManager {
    pub fn new(db: Arc<Database>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .pool_max_idle_per_host(32)
            .build()
            .unwrap_or_default();

        let initial_tasks = db.load_all_tasks().unwrap_or_default();
        let mut map = HashMap::new();
        for t in initial_tasks {
            map.insert(t.id.clone(), t);
        }

        Self {
            db,
            client,
            tasks: Arc::new(RwLock::new(map)),
            cancel_flags: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_all_tasks(&self) -> Vec<DownloadTask> {
        let tasks = self.tasks.read().await;
        let mut list: Vec<DownloadTask> = tasks.values().cloned().collect();
        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        list
    }

    pub async fn update_task_path(
        &self,
        task_id: &str,
        new_save_dir: &str,
        new_file_path: &str,
    ) -> Result<DownloadTask, String> {
        self.db
            .update_task_file_path(task_id, new_save_dir, new_file_path)
            .map_err(|e| format!("Database error: {}", e))?;

        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(task_id) {
            task.save_dir = new_save_dir.to_string();
            task.file_path = new_file_path.to_string();
            Ok(task.clone())
        } else {
            Err("Task not found".to_string())
        }
    }

    pub async fn prepare_download_task(
        &self,
        url: &str,
        filename: &str,
        save_dir: &str,
        connections: usize,
        custom_headers: Option<HashMap<String, String>>,
    ) -> Result<(DownloadTask, Arc<AtomicBool>), String> {
        let clean_url = crate::engine::probe::clean_stream_url(url);

        // 1. Probe URL to detect range support, actual size and HLS
        let probe = Prober::probe(&self.client, &clean_url, custom_headers).await?;

        let candidate_filename = if filename.trim().is_empty() {
            probe.filename
        } else {
            filename.to_string()
        };

        let final_filename = crate::engine::probe::sanitize_filename_with_ext(
            &candidate_filename,
            if probe.is_hls { "mp4" } else { "bin" },
        );

        let file_path = Path::new(save_dir)
            .join(&final_filename)
            .to_string_lossy()
            .to_string();

        let total_bytes = probe.total_bytes;
        let supports_range = probe.supports_range && total_bytes.is_some();
        let is_hls = probe.is_hls;
        let category = DownloadCategory::from_filename(&final_filename);

        let task_id = Uuid::new_v4().to_string();
        let conn_count = if supports_range {
            connections.clamp(1, 32)
        } else {
            1
        };

        crate::log_info!(
            "manager",
            "Starting task {}: filename='{}', dest='{}', conn={}, size={:?}",
            task_id, final_filename, file_path, conn_count, total_bytes
        );

        // 2. Build segments
        let segments = if let Some(total) = total_bytes {
            if supports_range && conn_count > 1 {
                calculate_segments(total, conn_count)
            } else {
                calculate_segments(total, 1)
            }
        } else {
            // Unknown file size
            vec![Segment {
                index: 0,
                start_byte: 0,
                end_byte: u64::MAX,
                downloaded_bytes: 0,
                is_finished: false,
            }]
        };

        let task = DownloadTask {
            id: task_id.clone(),
            url: clean_url,
            filename: final_filename,
            save_dir: save_dir.to_string(),
            file_path,
            total_bytes,
            downloaded_bytes: 0,
            category,
            status: TaskStatus::Downloading,
            connections: conn_count,
            supports_range,
            is_hls,
            created_at: chrono::Local::now().to_rfc3339(),
            completed_at: None,
            error_message: None,
            segments,
        };

        // Save to DB and Memory
        let _ = self.db.insert_task(&task);
        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(task_id.clone(), task.clone());
        }

        let cancel_flag = Arc::new(AtomicBool::new(false));
        {
            let mut flags = self.cancel_flags.write().await;
            flags.insert(task_id.clone(), cancel_flag.clone());
        }

        Ok((task, cancel_flag))
    }

    pub async fn start_download(
        &self,
        app_handle: tauri::AppHandle,
        url: String,
        filename: String,
        save_dir: String,
        connections: usize,
        custom_headers: Option<HashMap<String, String>>,
    ) -> Result<DownloadTask, String> {
        let (task, cancel_flag) = self
            .prepare_download_task(&url, &filename, &save_dir, connections, custom_headers.clone())
            .await?;

        // Spawn runner task
        let db_clone = self.db.clone();
        let tasks_clone = self.tasks.clone();
        let client_clone = self.client.clone();
        let task_for_run = task.clone();

        tauri::async_runtime::spawn(async move {
            Self::execute_task(
                app_handle,
                db_clone,
                tasks_clone,
                client_clone,
                task_for_run,
                cancel_flag,
                custom_headers,
            )
            .await;
        });

        Ok(task)
    }


    pub async fn pause_download(&self, task_id: &str) -> Result<(), String> {
        let flags = self.cancel_flags.read().await;
        if let Some(flag) = flags.get(task_id) {
            flag.store(true, Ordering::Relaxed);
        }

        let mut tasks = self.tasks.write().await;
        if let Some(t) = tasks.get_mut(task_id) {
            t.status = TaskStatus::Paused;
            let _ = self.db.update_task_progress(&t.id, t.downloaded_bytes, &t.status, &t.segments);
        }
        Ok(())
    }

    pub async fn resume_download(
        &self,
        app_handle: tauri::AppHandle,
        task_id: &str,
    ) -> Result<(), String> {
        let task_opt = {
            let tasks = self.tasks.read().await;
            tasks.get(task_id).cloned()
        };

        if let Some(mut task) = task_opt {
            if task.status == TaskStatus::Downloading || task.status == TaskStatus::Completed {
                return Ok(());
            }

            task.status = TaskStatus::Downloading;
            {
                let mut tasks = self.tasks.write().await;
                tasks.insert(task_id.to_string(), task.clone());
            }
            let _ = self.db.update_task_progress(&task.id, task.downloaded_bytes, &task.status, &task.segments);

            let cancel_flag = Arc::new(AtomicBool::new(false));
            {
                let mut flags = self.cancel_flags.write().await;
                flags.insert(task_id.to_string(), cancel_flag.clone());
            }

            let db_clone = self.db.clone();
            let tasks_clone = self.tasks.clone();
            let client_clone = self.client.clone();

            tauri::async_runtime::spawn(async move {
                Self::execute_task(
                    app_handle,
                    db_clone,
                    tasks_clone,
                    client_clone,
                    task,
                    cancel_flag,
                    None,
                )
                .await;
            });
        }
        Ok(())
    }

    pub async fn cancel_download(&self, task_id: &str, delete_file: bool) -> Result<(), String> {
        // Signal cancel
        {
            let flags = self.cancel_flags.read().await;
            if let Some(flag) = flags.get(task_id) {
                flag.store(true, Ordering::Relaxed);
            }
        }

        let removed_task = {
            let mut tasks = self.tasks.write().await;
            tasks.remove(task_id)
        };

        let _ = self.db.delete_task(task_id);

        if delete_file {
            if let Some(t) = removed_task {
                let _ = tokio::fs::remove_file(&t.file_path).await;
            }
        }

        Ok(())
    }
}

pub trait EventEmitter: Send + Sync + 'static {
    fn emit_event(&self, event: &str, payload: serde_json::Value);
}

impl EventEmitter for tauri::AppHandle {
    fn emit_event(&self, event: &str, payload: serde_json::Value) {
        use tauri::Emitter;
        let _ = self.emit(event, payload);
    }
}

impl DownloadManager {
    async fn execute_task(
        app_handle: tauri::AppHandle,
        db: Arc<Database>,
        tasks: Arc<RwLock<HashMap<String, DownloadTask>>>,
        client: reqwest::Client,
        task: DownloadTask,
        cancel_flag: Arc<AtomicBool>,
        custom_headers: Option<HashMap<String, String>>,
    ) {
        Self::execute_task_core(
            Arc::new(app_handle),
            db,
            tasks,
            client,
            task,
            cancel_flag,
            custom_headers,
        )
        .await;
    }

    pub async fn execute_task_core<E: EventEmitter>(
        emitter: Arc<E>,
        db: Arc<Database>,
        tasks: Arc<RwLock<HashMap<String, DownloadTask>>>,
        client: reqwest::Client,
        mut task: DownloadTask,
        cancel_flag: Arc<AtomicBool>,
        custom_headers: Option<HashMap<String, String>>,
    ) {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<(usize, u64)>(500);

        let effective_url = if task.url.contains("googlevideo.com") {
            custom_headers
                .as_ref()
                .and_then(|h| h.get("referer").or(h.get("Referer")))
                .cloned()
                .unwrap_or_else(|| task.url.clone())
        } else {
            task.url.clone()
        };

        let is_youtube = effective_url.contains("youtube.com") || effective_url.contains("youtu.be");
        let is_stream = is_youtube || task.is_hls || effective_url.contains(".m3u8");

        if is_stream {
            // Stream mode via YtDlpRunner (handles YouTube + HLS m3u8 streams with ffmpeg remuxing)
            let out_file = task.file_path.clone();
            let url_clone = effective_url;
            let cancel_clone = cancel_flag.clone();
            let tx_clone = tx.clone();

            tauri::async_runtime::spawn(async move {
                let res = crate::engine::ytdlp::YtDlpRunner::run_download(
                    url_clone,
                    out_file,
                    cancel_clone,
                    tx_clone,
                )
                .await;
                if let Err(e) = res {
                    eprintln!("Stream runner error: {}", e);
                }
            });
        } else {
            // Regular Multi-part or Single-part Download mode
            let writer = match FileWriter::create_or_open(&task.file_path, task.total_bytes).await {
                Ok(w) => Arc::new(w),
                Err(err) => {
                    let _ = db.mark_task_failed(&task.id, &err);
                    let mut ts = tasks.write().await;
                    if let Some(t) = ts.get_mut(&task.id) {
                        t.status = TaskStatus::Failed(err.clone());
                    }
                    emitter.emit_event("download-failed", serde_json::json!(&task.id));
                    return;
                }
            };

            for seg in &task.segments {
                if seg.is_finished {
                    continue;
                }

                let writer_handle = match writer.open_worker_handle().await {
                    Ok(h) => h,
                    Err(e) => {
                        eprintln!("Failed to open worker handle: {}", e);
                        return;
                    }
                };

                let worker_client = client.clone();
                let worker_url = task.url.clone();
                let idx = seg.index;
                let start = seg.start_byte;
                let end = seg.end_byte;
                let current_downloaded = seg.downloaded_bytes;
                let worker_cancel = cancel_flag.clone();
                let worker_tx = tx.clone();
                let headers_clone = custom_headers.clone();

                tauri::async_runtime::spawn(async move {
                    let _ = SegmentWorker::run(
                        worker_client,
                        worker_url,
                        idx,
                        start,
                        end,
                        current_downloaded,
                        writer_handle,
                        worker_cancel,
                        worker_tx,
                        headers_clone,
                    )
                    .await;
                });
            }
        }

        // Progress collection & Throttled Event Emitter loop
        let mut last_emit = Instant::now();
        let mut last_db_save = Instant::now();
        let mut speed_calc_instant = Instant::now();
        let mut bytes_since_last_calc: u64 = 0;
        let mut current_speed_bps: u64 = 0;

        drop(tx); // Drop extra sender so rx completes when all worker senders drop

        while let Some((seg_idx, chunk_bytes)) = rx.recv().await {
            task.downloaded_bytes += chunk_bytes;
            bytes_since_last_calc += chunk_bytes;

            update_segment_progress(&mut task.segments, seg_idx, chunk_bytes);

            let now = Instant::now();

            // Calculate Speed every 500ms
            if now.duration_since(speed_calc_instant) >= Duration::from_millis(500) {
                let elapsed_secs = now.duration_since(speed_calc_instant).as_secs_f64();
                if elapsed_secs > 0.0 {
                    current_speed_bps = (bytes_since_last_calc as f64 / elapsed_secs) as u64;
                }
                bytes_since_last_calc = 0;
                speed_calc_instant = now;
            }

            // Emit progress event to UI every 150ms
            if now.duration_since(last_emit) >= Duration::from_millis(150) {
                let metrics = calculate_metrics(
                    &task.id,
                    task.downloaded_bytes,
                    task.total_bytes,
                    current_speed_bps,
                    &task.status,
                    &task.segments,
                );
                emitter.emit_event("download-progress", serde_json::to_value(&metrics).unwrap_or_default());
                last_emit = now;
            }

            // Save to DB every 2 seconds for resume safety
            if now.duration_since(last_db_save) >= Duration::from_secs(2) {
                let _ = db.update_task_progress(&task.id, task.downloaded_bytes, &task.status, &task.segments);
                last_db_save = now;
            }
        }

        // All workers completed or cancelled
        let is_cancelled = cancel_flag.load(Ordering::Relaxed);
        let file_len = tokio::fs::metadata(&task.file_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);

        let outcome = evaluate_task_outcome(
            task.total_bytes,
            task.downloaded_bytes,
            file_len,
            is_stream,
            is_cancelled,
        );

        match outcome {
            Ok(final_bytes) => {
                let actual_final_bytes = if file_len > 0 { file_len } else { final_bytes };
                let completed_at = chrono::Local::now().to_rfc3339();
                task.status = TaskStatus::Completed;
                task.completed_at = Some(completed_at.clone());
                task.downloaded_bytes = actual_final_bytes;
                task.total_bytes = Some(actual_final_bytes);

                if task.segments.is_empty() {
                    task.segments.push(Segment {
                        index: 0,
                        start_byte: 0,
                        end_byte: actual_final_bytes.saturating_sub(1),
                        downloaded_bytes: actual_final_bytes,
                        is_finished: true,
                    });
                } else {
                    let seg_count = task.segments.len();
                    for seg in task.segments.iter_mut() {
                        seg.is_finished = true;
                        if seg.downloaded_bytes == 0 && seg_count == 1 {
                            seg.downloaded_bytes = actual_final_bytes;
                        }
                    }
                }

                let _ = db.mark_task_completed(&task.id, &completed_at, actual_final_bytes);
                {
                    let mut ts = tasks.write().await;
                    if let Some(t) = ts.get_mut(&task.id) {
                        t.status = TaskStatus::Completed;
                        t.completed_at = Some(completed_at);
                        t.downloaded_bytes = actual_final_bytes;
                        t.total_bytes = Some(actual_final_bytes);
                        t.segments = task.segments.clone();
                    }
                }

                let metrics = SpeedMetrics {
                    task_id: task.id.clone(),
                    speed_bps: 0,
                    eta_seconds: Some(0),
                    downloaded_bytes: actual_final_bytes,
                    total_bytes: Some(actual_final_bytes),
                    percent: 100.0,
                    status: TaskStatus::Completed,
                    segments: task.segments.clone(),
                };

                emitter.emit_event("download-progress", serde_json::to_value(&metrics).unwrap_or_default());
                emitter.emit_event("download-completed", serde_json::json!(&task.id));
                crate::log_info!("manager", "Task {} COMPLETED successfully: file='{}' ({} bytes)", task.id, task.file_path, actual_final_bytes);
            }
            Err(ref e) if e == "paused" => {
                task.status = TaskStatus::Paused;
                let _ = db.update_task_progress(&task.id, task.downloaded_bytes, &task.status, &task.segments);
                {
                    let mut ts = tasks.write().await;
                    if let Some(t) = ts.get_mut(&task.id) {
                        t.status = TaskStatus::Paused;
                    }
                }
                emitter.emit_event("download-paused", serde_json::json!(&task.id));
            }
            Err(err_msg) => {
                crate::log_error!("manager", "Task {} FAILED: {}", task.id, err_msg);
                let _ = db.mark_task_failed(&task.id, &err_msg);
                {
                    let mut ts = tasks.write().await;
                    if let Some(t) = ts.get_mut(&task.id) {
                        t.status = TaskStatus::Failed(err_msg.clone());
                    }
                }
                emitter.emit_event("download-failed", serde_json::json!(&task.id));
            }
        }
    }
}

pub fn update_segment_progress(segments: &mut [Segment], seg_idx: usize, chunk_bytes: u64) -> bool {
    if let Some(seg) = segments.get_mut(seg_idx) {
        seg.downloaded_bytes += chunk_bytes;
        if seg.start_byte + seg.downloaded_bytes >= seg.end_byte + 1 {
            seg.is_finished = true;
            return true;
        }
    }
    false
}

pub fn calculate_metrics(
    task_id: &str,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    current_speed_bps: u64,
    status: &TaskStatus,
    segments: &[Segment],
) -> SpeedMetrics {
    let percent = if let Some(tot) = total_bytes {
        if tot > 0 {
            ((downloaded_bytes as f64 / tot as f64) * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        }
    } else {
        0.0
    };

    let eta_seconds = if current_speed_bps > 0 {
        total_bytes.map(|tot| {
            let remaining = tot.saturating_sub(downloaded_bytes);
            remaining / current_speed_bps
        })
    } else {
        None
    };

    SpeedMetrics {
        task_id: task_id.to_string(),
        speed_bps: current_speed_bps,
        eta_seconds,
        downloaded_bytes,
        total_bytes,
        percent,
        status: status.clone(),
        segments: segments.to_vec(),
    }
}

pub fn evaluate_task_outcome(
    total_bytes: Option<u64>,
    downloaded_bytes: u64,
    file_len: u64,
    is_stream: bool,
    is_cancelled: bool,
) -> Result<u64, String> {
    if is_cancelled {
        return Err("paused".to_string());
    }
    let is_success = if let Some(tot) = total_bytes {
        downloaded_bytes >= tot || file_len >= tot || (is_stream && file_len > 0)
    } else {
        downloaded_bytes > 0 || file_len > 0
    };

    if is_success {
        let final_downloaded = if downloaded_bytes == 0 && file_len > 0 {
            file_len
        } else {
            downloaded_bytes
        };
        Ok(final_downloaded)
    } else {
        let err_msg = if file_len == 0 && downloaded_bytes == 0 {
            "Download failed: 0 bytes received from server or connection closed".to_string()
        } else {
            format!("Download incomplete: received {} bytes", downloaded_bytes)
        };
        Err(err_msg)
    }
}


pub fn calculate_segments(total: u64, conn_count: usize) -> Vec<Segment> {
    let mut segments = Vec::new();
    let conn_count = conn_count.max(1);
    if total == 0 {
        return vec![Segment {
            index: 0,
            start_byte: 0,
            end_byte: 0,
            downloaded_bytes: 0,
            is_finished: false,
        }];
    }

    let actual_conns = if total < conn_count as u64 {
        total as usize
    } else {
        conn_count
    };

    if actual_conns <= 1 {
        segments.push(Segment {
            index: 0,
            start_byte: 0,
            end_byte: total - 1,
            downloaded_bytes: 0,
            is_finished: false,
        });
    } else {
        let chunk_size = total / (actual_conns as u64);
        for i in 0..actual_conns {
            let start = i as u64 * chunk_size;
            let end = if i == actual_conns - 1 {
                total - 1
            } else {
                (i as u64 + 1) * chunk_size - 1
            };
            segments.push(Segment {
                index: i,
                start_byte: start,
                end_byte: end,
                downloaded_bytes: 0,
                is_finished: false,
            });
        }
    }
    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_segments_exact_coverage() {
        // Test varying file sizes and connection counts
        let test_cases = vec![
            (100u64, 8usize),
            (1000u64, 4usize),
            (1024 * 1024 * 10u64, 16usize),
            (7_341_289_102u64, 32usize),
            (1u64, 8usize),
            (7u64, 10usize),
            (0u64, 4usize),
        ];

        for (total, conn_count) in test_cases {
            let segs = calculate_segments(total, conn_count);
            assert!(!segs.is_empty(), "Segments should never be empty");

            if total == 0 {
                assert_eq!(segs.len(), 1);
                assert_eq!(segs[0].start_byte, 0);
                assert_eq!(segs[0].end_byte, 0);
                continue;
            }

            // Invariant 1: First segment starts at 0
            assert_eq!(segs[0].start_byte, 0);

            // Invariant 2: Last segment ends at total - 1
            assert_eq!(segs.last().unwrap().end_byte, total - 1);

            // Invariant 3: Contiguous, non-overlapping segments
            for i in 0..segs.len() - 1 {
                assert!(segs[i].start_byte <= segs[i].end_byte);
                assert_eq!(segs[i].end_byte + 1, segs[i + 1].start_byte);
            }

            // Invariant 4: Sum of all segment lengths must EXACTLY equal total bytes
            let sum_bytes: u64 = segs.iter().map(|s| s.end_byte - s.start_byte + 1).sum();
            assert_eq!(sum_bytes, total, "Sum of chunks must match total file size exactly");
        }
    }

    #[test]
    fn test_calculate_segments_single_connection() {
        let segs = calculate_segments(5000, 1);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].start_byte, 0);
        assert_eq!(segs[0].end_byte, 4999);
    }

    #[tokio::test]
    async fn test_download_manager_lifecycle() {
        let db = Arc::new(Database::open_in_memory().expect("in-memory db"));
        let task1 = DownloadTask {
            id: "task-mgr-1".to_string(),
            url: "https://example.com/file1.zip".to_string(),
            filename: "file1.zip".to_string(),
            save_dir: "C:\\Downloads".to_string(),
            file_path: "C:\\Downloads\\file1.zip".to_string(),
            total_bytes: Some(1024),
            downloaded_bytes: 0,
            category: DownloadCategory::Compressed,
            status: TaskStatus::Queued,
            connections: 4,
            supports_range: true,
            is_hls: false,
            created_at: "2026-09-20 12:00:00".to_string(),
            completed_at: None,
            error_message: None,
            segments: vec![],
        };
        db.insert_task(&task1).unwrap();

        let manager = DownloadManager::new(db.clone());
        let all_tasks = manager.get_all_tasks().await;
        assert_eq!(all_tasks.len(), 1);
        assert_eq!(all_tasks[0].id, "task-mgr-1");

        // Set a cancel flag for the task to test cancel signaling
        let flag = Arc::new(AtomicBool::new(false));
        manager.cancel_flags.write().await.insert("task-mgr-1".to_string(), flag.clone());

        // Cancel task without deleting file
        let cancel_res = manager.cancel_download("task-mgr-1", false).await;
        assert!(cancel_res.is_ok());

        // Verify cancel flag was set to true
        assert!(flag.load(Ordering::Relaxed));

        // Verify task removed from manager state and DB
        assert_eq!(manager.get_all_tasks().await.len(), 0);
        assert_eq!(db.load_all_tasks().unwrap().len(), 0);

        // Test pause
        let task2 = DownloadTask {
            id: "task-mgr-2".to_string(),
            url: "https://example.com/file2.zip".to_string(),
            filename: "file2.zip".to_string(),
            save_dir: "C:\\Downloads".to_string(),
            file_path: "C:\\Downloads\\file2.zip".to_string(),
            total_bytes: Some(2048),
            downloaded_bytes: 512,
            category: DownloadCategory::Compressed,
            status: TaskStatus::Downloading,
            connections: 4,
            supports_range: true,
            is_hls: false,
            created_at: "2026-09-20 12:00:00".to_string(),
            completed_at: None,
            error_message: None,
            segments: vec![],
        };
        db.insert_task(&task2).unwrap();
        manager.tasks.write().await.insert("task-mgr-2".to_string(), task2);
        
        let pause_res = manager.pause_download("task-mgr-2").await;
        assert!(pause_res.is_ok());
        let updated = manager.tasks.read().await.get("task-mgr-2").unwrap().clone();
        assert_eq!(updated.status, TaskStatus::Paused);

        // Test cancel with delete_file = true
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("to_delete.bin");
        tokio::fs::write(&file_path, b"test content").await.unwrap();
        assert!(file_path.exists());

        let task3 = DownloadTask {
            id: "task-mgr-3".to_string(),
            url: "https://example.com/file3.zip".to_string(),
            filename: "to_delete.bin".to_string(),
            save_dir: dir.path().to_string_lossy().to_string(),
            file_path: file_path.to_string_lossy().to_string(),
            total_bytes: Some(12),
            downloaded_bytes: 12,
            category: DownloadCategory::General,
            status: TaskStatus::Completed,
            connections: 1,
            supports_range: false,
            is_hls: false,
            created_at: "2026-09-20 12:00:00".to_string(),
            completed_at: None,
            error_message: None,
            segments: vec![],
        };
        manager.tasks.write().await.insert("task-mgr-3".to_string(), task3);
        let del_res = manager.cancel_download("task-mgr-3", true).await;
        assert!(del_res.is_ok());
        assert!(!file_path.exists(), "File should be deleted on disk");
    }

    #[test]
    fn test_calculate_metrics() {
        let segs = vec![Segment {
            index: 0,
            start_byte: 0,
            end_byte: 1000,
            downloaded_bytes: 500,
            is_finished: false,
        }];

        // Known total with speed
        let m1 = calculate_metrics("t1", 500, Some(1000), 250, &TaskStatus::Downloading, &segs);
        assert_eq!(m1.task_id, "t1");
        assert_eq!(m1.percent, 50.0);
        assert_eq!(m1.speed_bps, 250);
        assert_eq!(m1.eta_seconds, Some(2));
        assert_eq!(m1.segments.len(), 1);

        // Zero speed -> ETA None
        let m2 = calculate_metrics("t2", 500, Some(1000), 0, &TaskStatus::Downloading, &segs);
        assert_eq!(m2.eta_seconds, None);

        // Unknown total
        let m3 = calculate_metrics("t3", 500, None, 100, &TaskStatus::Downloading, &segs);
        assert_eq!(m3.percent, 0.0);
        assert_eq!(m3.eta_seconds, None);
    }

    #[test]
    fn test_update_segment_progress() {
        let mut segs = vec![
            Segment {
                index: 0,
                start_byte: 0,
                end_byte: 99,
                downloaded_bytes: 50,
                is_finished: false,
            },
        ];

        // Partial chunk: 50 + 20 = 70 < 100, not finished
        let finished = update_segment_progress(&mut segs, 0, 20);
        assert!(!finished);
        assert_eq!(segs[0].downloaded_bytes, 70);
        assert!(!segs[0].is_finished);

        // Final chunk: 70 + 30 = 100 >= 100, finished!
        let finished2 = update_segment_progress(&mut segs, 0, 30);
        assert!(finished2);
        assert_eq!(segs[0].downloaded_bytes, 100);
        assert!(segs[0].is_finished);

        // Out of bounds index should return false safely
        let oob = update_segment_progress(&mut segs, 99, 10);
        assert!(!oob);
    }

    #[test]
    fn test_evaluate_task_outcome() {
        // Cancelled
        assert_eq!(evaluate_task_outcome(Some(100), 50, 50, false, true), Err("paused".to_string()));

        // Success: downloaded_bytes >= total_bytes
        assert_eq!(evaluate_task_outcome(Some(100), 100, 100, false, false), Ok(100));

        // Success: file_len >= total_bytes (even if worker counter lagged)
        assert_eq!(evaluate_task_outcome(Some(100), 80, 100, false, false), Ok(80));

        // Success: stream with non-empty file
        assert_eq!(evaluate_task_outcome(None, 0, 5000, true, false), Ok(5000));

        // Failure: 0 bytes received
        let err_0 = evaluate_task_outcome(Some(100), 0, 0, false, false);
        assert!(err_0.is_err());
        assert!(err_0.unwrap_err().contains("0 bytes received"));

        // Failure: incomplete download
        let err_inc = evaluate_task_outcome(Some(100), 40, 40, false, false);
        assert!(err_inc.is_err());
        assert!(err_inc.unwrap_err().contains("incomplete"));
    }

    #[tokio::test]
    async fn test_prepare_download_task_mock() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let mut buf = [0u8; 1024];
                    let _ = socket.read(&mut buf).await;
                    let resp = "HTTP/1.1 206 Partial Content\r\n\
                                Content-Range: bytes 0-0/1048576\r\n\
                                Content-Length: 1\r\n\
                                Content-Type: video/mp4\r\n\
                                Connection: close\r\n\r\n\0";
                    let _ = socket.write_all(resp.as_bytes()).await;
                    let _ = socket.flush().await;
                });
            }
        });

        let db = Arc::new(Database::open_in_memory().unwrap());
        let manager = DownloadManager::new(db.clone());
        let mock_url = format!("http://{}/clip.mp4", addr);
        let temp = tempfile::tempdir().unwrap();

        let (task, cancel_flag) = manager
            .prepare_download_task(
                &mock_url,
                "my_clip.mp4",
                temp.path().to_str().unwrap(),
                4,
                None,
            )
            .await
            .unwrap();

        assert_eq!(task.filename, "my_clip.mp4");
        assert_eq!(task.category, DownloadCategory::Video);
        assert_eq!(task.connections, 4);
        assert_eq!(task.segments.len(), 4);
        assert_eq!(task.total_bytes, Some(1048576));
        assert!(!cancel_flag.load(Ordering::Relaxed));

        // Verify task stored in memory and DB
        let in_memory = manager.tasks.read().await.get(&task.id).cloned();
        assert!(in_memory.is_some());
        let in_db = db.load_all_tasks().unwrap();
        assert_eq!(in_db.len(), 1);
        assert_eq!(in_db[0].id, task.id);
    }

    struct TestEventEmitter {
        events: Arc<std::sync::Mutex<Vec<(String, serde_json::Value)>>>,
    }

    impl EventEmitter for TestEventEmitter {
        fn emit_event(&self, event: &str, payload: serde_json::Value) {
            self.events.lock().unwrap().push((event.to_string(), payload));
        }
    }

    #[tokio::test]
    async fn test_execute_task_core_multipart_success() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let mut buf = [0u8; 1024];
                    let _ = socket.read(&mut buf).await;
                    let resp = "HTTP/1.1 206 Partial Content\r\n\
                                Content-Range: bytes 0-49/50\r\n\
                                Content-Length: 50\r\n\
                                Connection: close\r\n\r\n\
                                01234567890123456789012345678901234567890123456789";
                    let _ = socket.write_all(resp.as_bytes()).await;
                    let _ = socket.flush().await;
                });
            }
        });

        let db = Arc::new(Database::open_in_memory().unwrap());
        let temp = tempfile::tempdir().unwrap();
        let file_path = temp.path().join("dl.bin").to_string_lossy().to_string();

        let task = DownloadTask {
            id: "task-exec-1".to_string(),
            url: format!("http://{}/file.bin", addr),
            filename: "dl.bin".to_string(),
            save_dir: temp.path().to_string_lossy().to_string(),
            file_path,
            total_bytes: Some(50),
            downloaded_bytes: 0,
            category: DownloadCategory::General,
            status: TaskStatus::Downloading,
            connections: 1,
            supports_range: true,
            is_hls: false,
            created_at: "2026-09-20 12:00:00".to_string(),
            completed_at: None,
            error_message: None,
            segments: vec![Segment {
                index: 0,
                start_byte: 0,
                end_byte: 49,
                downloaded_bytes: 0,
                is_finished: false,
            }],
        };
        db.insert_task(&task).unwrap();

        let tasks = Arc::new(RwLock::new(HashMap::new()));
        tasks.write().await.insert(task.id.clone(), task.clone());
        let cancel = Arc::new(AtomicBool::new(false));

        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let emitter = Arc::new(TestEventEmitter { events: events.clone() });
        let client = reqwest::Client::new();

        DownloadManager::execute_task_core(
            emitter,
            db.clone(),
            tasks.clone(),
            client,
            task,
            cancel,
            None,
        ).await;

        let evs = events.lock().unwrap();
        assert!(evs.iter().any(|(name, _)| name == "download-completed"));
        assert_eq!(db.load_all_tasks().unwrap()[0].status, TaskStatus::Completed);
    }

    #[tokio::test]
    async fn test_execute_task_core_invalid_path_fails() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        let task = DownloadTask {
            id: "task-fail-1".to_string(),
            url: "http://127.0.0.1:9999/bad.bin".to_string(),
            filename: "bad.bin".to_string(),
            save_dir: "".to_string(),
            file_path: "Z:\\NonExistentDirectoryXYZ\\dl.bin".to_string(),
            total_bytes: Some(100),
            downloaded_bytes: 0,
            category: DownloadCategory::General,
            status: TaskStatus::Downloading,
            connections: 1,
            supports_range: true,
            is_hls: false,
            created_at: "2026-09-20 12:00:00".to_string(),
            completed_at: None,
            error_message: None,
            segments: vec![],
        };
        db.insert_task(&task).unwrap();

        let tasks = Arc::new(RwLock::new(HashMap::new()));
        tasks.write().await.insert(task.id.clone(), task.clone());
        let cancel = Arc::new(AtomicBool::new(false));

        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let emitter = Arc::new(TestEventEmitter { events: events.clone() });
        let client = reqwest::Client::new();

        DownloadManager::execute_task_core(
            emitter,
            db.clone(),
            tasks.clone(),
            client,
            task,
            cancel,
            None,
        ).await;

        let evs = events.lock().unwrap();
        assert!(evs.iter().any(|(name, _)| name == "download-failed"));
        match db.load_all_tasks().unwrap()[0].status {
            TaskStatus::Failed(_) => {},
            _ => panic!("Expected failed status"),
        }
    }

    #[tokio::test]
    async fn test_execute_task_core_paused() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        let temp = tempfile::tempdir().unwrap();
        let file_path = temp.path().join("paused.bin").to_string_lossy().to_string();

        let task = DownloadTask {
            id: "task-pause-1".to_string(),
            url: "http://127.0.0.1:9999/pause.bin".to_string(),
            filename: "paused.bin".to_string(),
            save_dir: temp.path().to_string_lossy().to_string(),
            file_path,
            total_bytes: Some(100),
            downloaded_bytes: 20,
            category: DownloadCategory::General,
            status: TaskStatus::Downloading,
            connections: 1,
            supports_range: true,
            is_hls: false,
            created_at: "2026-09-20 12:00:00".to_string(),
            completed_at: None,
            error_message: None,
            segments: vec![],
        };
        db.insert_task(&task).unwrap();

        let tasks = Arc::new(RwLock::new(HashMap::new()));
        tasks.write().await.insert(task.id.clone(), task.clone());
        let cancel = Arc::new(AtomicBool::new(true)); // Pre-cancelled -> paused

        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let emitter = Arc::new(TestEventEmitter { events: events.clone() });
        let client = reqwest::Client::new();

        DownloadManager::execute_task_core(
            emitter,
            db.clone(),
            tasks.clone(),
            client,
            task,
            cancel,
            None,
        ).await;

        let evs = events.lock().unwrap();
        assert!(evs.iter().any(|(name, _)| name == "download-paused"));
        assert_eq!(db.load_all_tasks().unwrap()[0].status, TaskStatus::Paused);
    }

    #[tokio::test]
    async fn test_execute_task_core_stream_branch() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        let temp = tempfile::tempdir().unwrap();
        let file_path = temp.path().join("stream.mp4").to_string_lossy().to_string();

        let mut headers = HashMap::new();
        headers.insert("referer".to_string(), "https://youtube.com/watch?v=123".to_string());

        let task = DownloadTask {
            id: "task-stream-1".to_string(),
            url: "https://googlevideo.com/videoplayback?id=123".to_string(),
            filename: "stream.mp4".to_string(),
            save_dir: temp.path().to_string_lossy().to_string(),
            file_path,
            total_bytes: None,
            downloaded_bytes: 0,
            category: DownloadCategory::Video,
            status: TaskStatus::Downloading,
            connections: 1,
            supports_range: false,
            is_hls: true,
            created_at: "2026-09-20 12:00:00".to_string(),
            completed_at: None,
            error_message: None,
            segments: vec![],
        };
        db.insert_task(&task).unwrap();

        let tasks = Arc::new(RwLock::new(HashMap::new()));
        tasks.write().await.insert(task.id.clone(), task.clone());
        let cancel = Arc::new(AtomicBool::new(true)); // Pre-cancelled to stop yt-dlp quickly

        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let emitter = Arc::new(TestEventEmitter { events: events.clone() });
        let client = reqwest::Client::new();

        DownloadManager::execute_task_core(
            emitter,
            db.clone(),
            tasks.clone(),
            client,
            task,
            cancel,
            Some(headers),
        ).await;

        let evs = events.lock().unwrap();
        assert!(evs.iter().any(|(name, _)| name == "download-paused"));
    }

    #[tokio::test]
    async fn test_prepare_download_task_empty_filename_and_no_range() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let mut buf = [0u8; 1024];
                    let _ = socket.read(&mut buf).await;
                    let resp = "HTTP/1.1 200 OK\r\n\
                                Content-Type: application/octet-stream\r\n\
                                Connection: close\r\n\r\n";
                    let _ = socket.write_all(resp.as_bytes()).await;
                    let _ = socket.flush().await;
                });
            }
        });

        let db = Arc::new(Database::open_in_memory().unwrap());
        let manager = DownloadManager::new(db.clone());
        let mock_url = format!("http://{}/server_file.dat", addr);
        let temp = tempfile::tempdir().unwrap();

        let (task, _) = manager
            .prepare_download_task(
                &mock_url,
                "", // empty candidate filename
                temp.path().to_str().unwrap(),
                8,
                None,
            )
            .await
            .unwrap();

        assert_eq!(task.connections, 1);
        assert!(!task.supports_range);
        assert_eq!(task.total_bytes, None);
        assert_eq!(task.segments.len(), 1);
        assert_eq!(task.segments[0].end_byte, u64::MAX);
    }
}



