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
use crate::engine::limiter::TokenBucketRateLimiter;
use crate::engine::probe::Prober;
#[allow(unused_imports)]
use crate::engine::types::DownloadCategory;
use crate::engine::types::{
    DownloadTask, DuplicateCheckResult, GlobalSpeedLimitConfig, ProbeResult, Segment, SpeedMetrics, TaskStatus,
};
use crate::engine::worker::SegmentWorker;
use crate::engine::writer::FileWriter;

pub struct DownloadManager {
    pub db: Arc<Database>,
    pub client: reqwest::Client,
    pub tasks: Arc<RwLock<HashMap<String, DownloadTask>>>,
    pub cancel_flags: Arc<RwLock<HashMap<String, Arc<AtomicBool>>>>,
    pub global_limiter: Arc<TokenBucketRateLimiter>,
    pub global_limiter_enabled: Arc<AtomicBool>,
    pub task_limiters: Arc<RwLock<HashMap<String, Arc<TokenBucketRateLimiter>>>>,
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
        let mut task_limiters_map = HashMap::new();
        for t in initial_tasks {
            if let Some(bps) = t.speed_limit_bps {
                if bps > 0 {
                    task_limiters_map
                        .insert(t.id.clone(), Arc::new(TokenBucketRateLimiter::new(bps)));
                }
            }
            map.insert(t.id.clone(), t);
        }

        let global_enabled = db
            .get_setting("global_speed_limit_enabled")
            .unwrap_or(None)
            .map(|v| v == "true")
            .unwrap_or(false);
        let global_bps = db
            .get_setting("global_speed_limit_bps")
            .unwrap_or(None)
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(2 * 1024 * 1024);

        let global_limiter = Arc::new(TokenBucketRateLimiter::new(if global_enabled {
            global_bps
        } else {
            0
        }));
        let global_limiter_enabled = Arc::new(AtomicBool::new(global_enabled));

        Self {
            db,
            client,
            tasks: Arc::new(RwLock::new(map)),
            cancel_flags: Arc::new(RwLock::new(HashMap::new())),
            global_limiter,
            global_limiter_enabled,
            task_limiters: Arc::new(RwLock::new(task_limiters_map)),
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

    pub async fn refresh_task_url(
        &self,
        task_id: &str,
        new_url: &str,
    ) -> Result<ProbeResult, String> {
        let clean_url = crate::engine::probe::clean_stream_url(new_url);

        // 1. Probe the new URL
        let probe = Prober::probe(&self.client, &clean_url, None).await?;

        // 2. Validate with existing task
        {
            let mut tasks = self.tasks.write().await;
            if let Some(task) = tasks.get_mut(task_id) {
                // Parity check: if old task had known total_bytes, ensure new_url matches
                if let (Some(old_size), Some(new_size)) = (task.total_bytes, probe.total_bytes) {
                    if old_size != new_size {
                        return Err(format!(
                            "Ukuran berkas baru ({} bytes) tidak sesuai dengan unduhan sebelumnya ({} bytes)",
                            new_size, old_size
                        ));
                    }
                }
                if task.downloaded_bytes > 0 && !probe.supports_range && !probe.is_hls {
                    return Err("Server URL baru tidak mendukung Range (resume byte offset)".to_string());
                }

                // Update task URL in memory
                task.url = clean_url.clone();
                task.error_message = None;
            } else {
                return Err("Task not found".to_string());
            }
        }

        // 3. Update DB
        self.db
            .update_task_url(task_id, &clean_url)
            .map_err(|e| format!("Database error: {}", e))?;

        Ok(probe)
    }

    pub async fn set_global_speed_limit(
        &self,
        enabled: bool,
        limit_bps: Option<u64>,
    ) -> Result<(), String> {
        let bps = limit_bps.unwrap_or(0);
        let _ = self.db.set_setting(
            "global_speed_limit_enabled",
            if enabled { "true" } else { "false" },
        );
        let _ = self
            .db
            .set_setting("global_speed_limit_bps", &bps.to_string());

        self.global_limiter_enabled
            .store(enabled, Ordering::Relaxed);
        self.global_limiter
            .set_limit_bps(if enabled { bps } else { 0 });
        Ok(())
    }

    pub async fn get_global_speed_limit(&self) -> Result<GlobalSpeedLimitConfig, String> {
        let enabled = self.global_limiter_enabled.load(Ordering::Relaxed);
        let limit_bps = self.global_limiter.get_limit_bps();
        Ok(GlobalSpeedLimitConfig {
            enabled,
            limit_bps: if limit_bps > 0 {
                Some(limit_bps)
            } else {
                None
            },
        })
    }

    pub async fn set_task_speed_limit(
        &self,
        task_id: &str,
        limit_bps: Option<u64>,
    ) -> Result<(), String> {
        self.db
            .update_task_speed_limit(task_id, limit_bps)
            .map_err(|e| format!("Database error: {}", e))?;

        let mut tasks = self.tasks.write().await;
        if let Some(t) = tasks.get_mut(task_id) {
            t.speed_limit_bps = limit_bps;
        }

        let mut limiters = self.task_limiters.write().await;
        let bps = limit_bps.unwrap_or(0);
        if bps > 0 {
            if let Some(limiter) = limiters.get(task_id) {
                limiter.set_limit_bps(bps);
            } else {
                limiters.insert(
                    task_id.to_string(),
                    Arc::new(TokenBucketRateLimiter::new(bps)),
                );
            }
        } else if let Some(limiter) = limiters.get(task_id) {
            limiter.set_limit_bps(0);
        }

        Ok(())
    }

    /// Generates a unique filename following the "name (1).ext" convention
    /// if the file already exists in `dir`.
    pub fn suggest_unique_filename(dir: &str, filename: &str) -> String {
        let p = Path::new(filename);
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or(filename);
        let ext = p.extension().and_then(|e| e.to_str());

        for i in 1..=9999 {
            let candidate = match ext {
                Some(e) if !e.is_empty() => format!("{} ({}).{}", stem, i, e),
                _ => format!("{} ({})", stem, i),
            };
            let full_path = Path::new(dir).join(&candidate);
            if !full_path.exists() {
                return candidate;
            }
        }

        format!("{}_{}", filename, &Uuid::new_v4().to_string()[..8])
    }

    /// Checks if a download with the same URL or colliding target file path exists.
    /// Follows strict priority:
    /// 1. In-progress tasks (Downloading, Probing, Queued)
    /// 2. Paused tasks
    /// 3. Completed tasks (only if physical file exists on disk; if deleted, ignored)
    pub async fn check_duplicate_task(
        &self,
        url: &str,
        filename: &str,
        save_dir: &str,
    ) -> DuplicateCheckResult {
        let clean_url = crate::engine::probe::clean_stream_url(url);
        let trimmed_url = url.trim();
        let target_path = if !save_dir.is_empty() && !filename.is_empty() {
            Some(Path::new(save_dir).join(filename))
        } else {
            None
        };

        // Gather in-memory tasks
        let tasks_guard = self.tasks.read().await;
        let mut all_tasks: Vec<DownloadTask> = tasks_guard.values().cloned().collect();
        drop(tasks_guard);

        // Supplement from DB if completed/historic tasks are not in memory
        if let Ok(db_tasks) = self.db.load_all_tasks() {
            for dt in db_tasks {
                if !all_tasks.iter().any(|t| t.id == dt.id) {
                    all_tasks.push(dt);
                }
            }
        }

        let is_matching = |t: &DownloadTask| -> bool {
            if !trimmed_url.is_empty() && (t.url == trimmed_url || (!clean_url.is_empty() && t.url == clean_url)) {
                return true;
            }
            if let Some(target) = &target_path {
                let task_p = Path::new(&t.file_path);
                if task_p == target {
                    return true;
                }
                if !t.save_dir.is_empty() && !t.filename.is_empty() {
                    let combined = Path::new(&t.save_dir).join(&t.filename);
                    if combined == *target {
                        return true;
                    }
                }
            }
            false
        };

        let mut matching_tasks: Vec<DownloadTask> = all_tasks.into_iter().filter(is_matching).collect();

        // Sort by priority: Downloading (0) > Probing/Queued (1) > Paused (2) > Completed (3) > Failed (4)
        matching_tasks.sort_by_key(|t| match t.status {
            TaskStatus::Downloading => 0,
            TaskStatus::Probing | TaskStatus::Queued => 1,
            TaskStatus::Paused => 2,
            TaskStatus::Completed => 3,
            TaskStatus::Failed(_) => 4,
        });

        for matched in matching_tasks {
            let file_exists = Path::new(&matched.file_path).exists();

            // Key Invariant: If Completed, but file physically removed from disk,
            // treat as not duplicated so the user can download anew without hindrance.
            if matched.status == TaskStatus::Completed && !file_exists {
                continue;
            }

            let percent = if let Some(total) = matched.total_bytes {
                if total > 0 {
                    (matched.downloaded_bytes as f64 / total as f64) * 100.0
                } else {
                    0.0
                }
            } else if matched.status == TaskStatus::Completed {
                100.0
            } else {
                0.0
            };

            let base_filename = if !filename.is_empty() {
                filename
            } else {
                &matched.filename
            };
            let target_dir = if !save_dir.is_empty() {
                save_dir
            } else {
                &matched.save_dir
            };

            let suggested_new_filename = Self::suggest_unique_filename(target_dir, base_filename);

            return DuplicateCheckResult {
                is_duplicate: true,
                status: Some(matched.status),
                task_id: Some(matched.id),
                filename: Some(matched.filename),
                file_path: Some(matched.file_path),
                file_exists_on_disk: file_exists,
                downloaded_bytes: matched.downloaded_bytes,
                total_bytes: matched.total_bytes,
                percent,
                completed_at: matched.completed_at,
                suggested_new_filename: Some(suggested_new_filename),
            };
        }

        DuplicateCheckResult {
            is_duplicate: false,
            status: None,
            task_id: None,
            filename: None,
            file_path: None,
            file_exists_on_disk: false,
            downloaded_bytes: 0,
            total_bytes: None,
            percent: 0.0,
            completed_at: None,
            suggested_new_filename: None,
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

        // 1. Probe the URL
        let probe = Prober::probe(&self.client, &clean_url, custom_headers.clone()).await?;

        let final_filename = if filename.is_empty() {
            probe.filename.clone()
        } else {
            filename.to_string()
        };

        let file_path = Path::new(save_dir)
            .join(&final_filename)
            .to_string_lossy()
            .to_string();

        let task_id = Uuid::new_v4().to_string();
        let total_bytes = probe.total_bytes;
        let supports_range = probe.supports_range;
        let is_hls = probe.is_hls;
        let category = probe.category;

        // 2. Partition into segments
        let conn_count = if supports_range && !is_hls {
            connections.max(1)
        } else {
            1
        };

        let segments = if let Some(total) = total_bytes {
            calculate_segments(total, conn_count)
        } else {
            vec![Segment {
                index: 0,
                start_byte: 0,
                end_byte: u64::MAX,
                downloaded_bytes: 0,
                is_finished: false,
            }]
        };

        let referer = custom_headers
            .as_ref()
            .and_then(|h| h.get("referer").or(h.get("Referer")))
            .cloned();

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
            referer,
            speed_limit_bps: None,
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

        // Extract rate limiters: always pass global_limiter and task_limiter so running workers dynamically throttle
        let global_limiter = Some(self.global_limiter.clone());
        let task_limiter = {
            let mut limiters = self.task_limiters.write().await;
            Some(
                limiters
                    .entry(task.id.clone())
                    .or_insert_with(|| {
                        Arc::new(TokenBucketRateLimiter::new(task.speed_limit_bps.unwrap_or(0)))
                    })
                    .clone(),
            )
        };

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
                task_limiter,
                global_limiter,
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

            // Extract rate limiters: always pass global_limiter and task_limiter so running workers dynamically throttle
            let global_limiter = Some(self.global_limiter.clone());
            let task_limiter = {
                let mut limiters = self.task_limiters.write().await;
                Some(
                    limiters
                        .entry(task.id.clone())
                        .or_insert_with(|| {
                            Arc::new(TokenBucketRateLimiter::new(task.speed_limit_bps.unwrap_or(0)))
                        })
                        .clone(),
                )
            };

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
                    task_limiter,
                    global_limiter,
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
        task_limiter: Option<Arc<TokenBucketRateLimiter>>,
        global_limiter: Option<Arc<TokenBucketRateLimiter>>,
    ) {
        Self::execute_task_core(
            Arc::new(app_handle),
            db,
            tasks,
            client,
            task,
            cancel_flag,
            custom_headers,
            task_limiter,
            global_limiter,
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
        task_limiter: Option<Arc<TokenBucketRateLimiter>>,
        global_limiter: Option<Arc<TokenBucketRateLimiter>>,
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

        let effective_limit_bps = match (
            task_limiter.as_ref().map(|l| l.get_limit_bps()).filter(|&b| b > 0),
            global_limiter.as_ref().map(|l| l.get_limit_bps()).filter(|&b| b > 0),
        ) {
            (Some(t), Some(g)) => Some(t.min(g)),
            (Some(t), None) => Some(t),
            (None, Some(g)) => Some(g),
            (None, None) => None,
        };

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
                    effective_limit_bps,
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
                let worker_task_limiter = task_limiter.clone();
                let worker_global_limiter = global_limiter.clone();

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
                        worker_task_limiter,
                        worker_global_limiter,
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

            if is_stream && task.segments.len() > 1 {
                sync_stream_segments(&mut task.segments, task.downloaded_bytes);
            } else {
                update_segment_progress(&mut task.segments, seg_idx, chunk_bytes);
            }

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

                if is_stream && task.segments.len() > 1 {
                    sync_stream_segments(&mut task.segments, actual_final_bytes);
                    for seg in task.segments.iter_mut() {
                        seg.is_finished = true;
                    }
                } else if task.segments.is_empty() {
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

pub fn sync_stream_segments(segments: &mut [Segment], total_downloaded: u64) {
    for seg in segments.iter_mut() {
        if seg.end_byte == u64::MAX {
            seg.downloaded_bytes = total_downloaded.saturating_sub(seg.start_byte);
            seg.is_finished = false;
            continue;
        }
        let seg_len = seg.end_byte.saturating_sub(seg.start_byte).saturating_add(1);
        if total_downloaded >= seg.end_byte.saturating_add(1) {
            seg.downloaded_bytes = seg_len;
            seg.is_finished = true;
        } else if total_downloaded > seg.start_byte {
            seg.downloaded_bytes = total_downloaded.saturating_sub(seg.start_byte);
            seg.is_finished = false;
        } else {
            seg.downloaded_bytes = 0;
            seg.is_finished = false;
        }
    }
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
            referer: None,
            speed_limit_bps: None,
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
            referer: None,
            speed_limit_bps: None,
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
            referer: None,
            speed_limit_bps: None,
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
    fn test_sync_stream_segments() {
        let mut segs = calculate_segments(1000, 4);

        // At 0 bytes
        sync_stream_segments(&mut segs, 0);
        assert_eq!(segs[0].downloaded_bytes, 0);
        assert_eq!(segs[3].downloaded_bytes, 0);

        // At 300 bytes: seg 0 full (250), seg 1 has 50, seg 2 has 0, seg 3 has 0
        sync_stream_segments(&mut segs, 300);
        assert_eq!(segs[0].downloaded_bytes, 250);
        assert!(segs[0].is_finished);
        assert_eq!(segs[1].downloaded_bytes, 50);
        assert!(!segs[1].is_finished);
        assert_eq!(segs[2].downloaded_bytes, 0);
        assert_eq!(segs[3].downloaded_bytes, 0);

        // At 983 bytes (98.3%): seg 0, 1, 2 full, seg 3 has 233
        sync_stream_segments(&mut segs, 983);
        assert_eq!(segs[0].downloaded_bytes, 250);
        assert!(segs[0].is_finished);
        assert_eq!(segs[1].downloaded_bytes, 250);
        assert!(segs[1].is_finished);
        assert_eq!(segs[2].downloaded_bytes, 250);
        assert!(segs[2].is_finished);
        assert_eq!(segs[3].downloaded_bytes, 233);
        assert!(!segs[3].is_finished);

        // At 1000 bytes (100%): all finished
        sync_stream_segments(&mut segs, 1000);
        for seg in &segs {
            assert!(seg.is_finished);
        }

        // Unknown size: end_byte = u64::MAX
        let mut unknown_segs = vec![Segment {
            index: 0,
            start_byte: 0,
            end_byte: u64::MAX,
            downloaded_bytes: 0,
            is_finished: false,
        }];
        sync_stream_segments(&mut unknown_segs, 5000);
        assert_eq!(unknown_segs[0].downloaded_bytes, 5000);
        assert!(!unknown_segs[0].is_finished);
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
            referer: None,
            speed_limit_bps: None,
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
            None,
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
            referer: None,
            speed_limit_bps: None,
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
            None,
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
            referer: None,
            speed_limit_bps: None,
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
            None,
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
            referer: None,
            speed_limit_bps: None,
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
            None,
            None,
        ).await;

        let evs = events.lock().unwrap();
        assert!(evs.iter().any(|(name, _)| name == "download-paused"));
    }

    #[tokio::test]
    async fn test_manager_refresh_task_url_success_and_parity() {
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
                                Content-Length: 100\r\n\
                                Accept-Ranges: bytes\r\n\
                                Connection: close\r\n\r\n";
                    let _ = socket.write_all(resp.as_bytes()).await;
                    let _ = socket.flush().await;
                });
            }
        });

        let db = Arc::new(Database::open_in_memory().unwrap());
        let manager = DownloadManager::new(db.clone());

        let initial_task = DownloadTask {
            id: "task-refresh-1".to_string(),
            url: "http://expired-url.example.com/file.bin".to_string(),
            filename: "file.bin".to_string(),
            save_dir: "C:\\Downloads".to_string(),
            file_path: "C:\\Downloads\\file.bin".to_string(),
            total_bytes: Some(100),
            downloaded_bytes: 40,
            category: DownloadCategory::General,
            status: TaskStatus::Paused,
            connections: 4,
            supports_range: true,
            is_hls: false,
            created_at: "2026-09-20 12:00:00".to_string(),
            completed_at: None,
            error_message: Some("Connection timeout".to_string()),
            segments: vec![],
            referer: Some("https://example.com/download".to_string()),
            speed_limit_bps: None,
        };
        db.insert_task(&initial_task).unwrap();
        manager.tasks.write().await.insert("task-refresh-1".to_string(), initial_task);

        let fresh_url = format!("http://{}/fresh_file.bin", addr);
        let result = manager.refresh_task_url("task-refresh-1", &fresh_url).await;
        assert!(result.is_ok(), "Expected refresh_task_url to succeed: {:?}", result.err());

        // Verify task updated in memory
        let tasks = manager.tasks.read().await;
        let updated = tasks.get("task-refresh-1").unwrap();
        assert_eq!(updated.url, fresh_url);
        assert_eq!(updated.downloaded_bytes, 40); // Preserved byte progress
        assert_eq!(updated.error_message, None);

        // Verify task updated in DB
        let db_tasks = db.load_all_tasks().unwrap();
        assert_eq!(db_tasks[0].url, fresh_url);
    }

    #[tokio::test]
    async fn test_manager_refresh_task_url_size_mismatch() {
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
                                Content-Length: 9999\r\n\
                                Accept-Ranges: bytes\r\n\
                                Connection: close\r\n\r\n";
                    let _ = socket.write_all(resp.as_bytes()).await;
                    let _ = socket.flush().await;
                });
            }
        });

        let db = Arc::new(Database::open_in_memory().unwrap());
        let manager = DownloadManager::new(db.clone());

        let initial_task = DownloadTask {
            id: "task-refresh-mismatch".to_string(),
            url: "http://expired.example.com/file.bin".to_string(),
            filename: "file.bin".to_string(),
            save_dir: "C:\\Downloads".to_string(),
            file_path: "C:\\Downloads\\file.bin".to_string(),
            total_bytes: Some(100), // Old size 100 vs New size 9999
            downloaded_bytes: 20,
            category: DownloadCategory::General,
            status: TaskStatus::Paused,
            connections: 4,
            supports_range: true,
            is_hls: false,
            created_at: "2026-09-20 12:00:00".to_string(),
            completed_at: None,
            error_message: None,
            segments: vec![],
            referer: None,
            speed_limit_bps: None,
        };
        db.insert_task(&initial_task).unwrap();
        manager.tasks.write().await.insert("task-refresh-mismatch".to_string(), initial_task);

        let fresh_url = format!("http://{}/different_size.bin", addr);
        let result = manager.refresh_task_url("task-refresh-mismatch", &fresh_url).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("tidak sesuai"));
    }

    #[tokio::test]
    async fn test_manager_refresh_task_not_found() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        let manager = DownloadManager::new(db.clone());
        let result = manager.refresh_task_url("non-existent-task", "http://127.0.0.1:9999/test.bin").await;
        assert!(result.is_err());
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

    #[tokio::test]
    async fn test_manager_global_speed_limit() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        let manager = DownloadManager::new(db.clone());

        let initial_cfg = manager.get_global_speed_limit().await.unwrap();
        assert!(!initial_cfg.enabled);
        assert_eq!(initial_cfg.limit_bps, None);

        // Enable with 1 MB/s (1_048_576 B/s)
        manager.set_global_speed_limit(true, Some(1_048_576)).await.unwrap();
        let cfg = manager.get_global_speed_limit().await.unwrap();
        assert!(cfg.enabled);
        assert_eq!(cfg.limit_bps, Some(1_048_576));
        assert_eq!(manager.global_limiter.get_limit_bps(), 1_048_576);

        // Disable limit
        manager.set_global_speed_limit(false, None).await.unwrap();
        let disabled_cfg = manager.get_global_speed_limit().await.unwrap();
        assert!(!disabled_cfg.enabled);
        assert_eq!(disabled_cfg.limit_bps, None);
        assert_eq!(manager.global_limiter.get_limit_bps(), 0);
    }

    #[tokio::test]
    async fn test_manager_task_speed_limit() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        let manager = DownloadManager::new(db.clone());

        let task = DownloadTask {
            id: "task-limit-1".to_string(),
            url: "http://127.0.0.1:9999/file.bin".to_string(),
            filename: "file.bin".to_string(),
            save_dir: "/tmp".to_string(),
            file_path: "/tmp/file.bin".to_string(),
            total_bytes: Some(10_000_000),
            downloaded_bytes: 0,
            category: DownloadCategory::General,
            status: TaskStatus::Downloading,
            connections: 4,
            supports_range: true,
            is_hls: false,
            created_at: "2026-09-20 12:00:00".to_string(),
            completed_at: None,
            error_message: None,
            segments: vec![],
            referer: None,
            speed_limit_bps: None,
        };
        db.insert_task(&task).unwrap();
        manager.tasks.write().await.insert(task.id.clone(), task.clone());

        // Set per-task speed limit 500 KB/s (512_000 B/s)
        manager.set_task_speed_limit("task-limit-1", Some(512_000)).await.unwrap();

        // Verify task in memory
        {
            let tasks = manager.tasks.read().await;
            assert_eq!(tasks.get("task-limit-1").unwrap().speed_limit_bps, Some(512_000));
        }

        // Verify limiter in map
        {
            let limiters = manager.task_limiters.read().await;
            let limiter = limiters.get("task-limit-1").unwrap();
            assert_eq!(limiter.get_limit_bps(), 512_000);
        }

        // Verify in DB
        let loaded = db.load_all_tasks().unwrap();
        assert_eq!(loaded[0].speed_limit_bps, Some(512_000));

        // Clear task limit
        manager.set_task_speed_limit("task-limit-1", None).await.unwrap();
        {
            let limiters = manager.task_limiters.read().await;
            let limiter = limiters.get("task-limit-1").unwrap();
            assert_eq!(limiter.get_limit_bps(), 0);
        }
    }

    #[test]
    fn test_suggest_unique_filename() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().to_str().unwrap();

        // No existing file
        let s1 = DownloadManager::suggest_unique_filename(dir, "video.mp4");
        assert_eq!(s1, "video (1).mp4");

        // Create video (1).mp4
        std::fs::write(temp.path().join("video (1).mp4"), b"test").unwrap();
        let s2 = DownloadManager::suggest_unique_filename(dir, "video.mp4");
        assert_eq!(s2, "video (2).mp4");

        // Test without extension
        let s3 = DownloadManager::suggest_unique_filename(dir, "README");
        assert_eq!(s3, "README (1)");
    }

    #[tokio::test]
    async fn test_check_duplicate_task_lifecycle_and_physical_disk_check() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        let manager = DownloadManager::new(db.clone());
        let temp = tempfile::tempdir().unwrap();
        let save_dir = temp.path().to_string_lossy().to_string();
        let file_path = temp.path().join("file.zip").to_string_lossy().to_string();

        // 1. Initial check: not duplicate
        let r0 = manager.check_duplicate_task("https://example.com/file.zip", "file.zip", &save_dir).await;
        assert!(!r0.is_duplicate);

        // 2. Active Downloading task
        let active_task = DownloadTask {
            id: "task-active-1".to_string(),
            url: "https://example.com/file.zip".to_string(),
            filename: "file.zip".to_string(),
            save_dir: save_dir.clone(),
            file_path: file_path.clone(),
            total_bytes: Some(1_000_000),
            downloaded_bytes: 450_000,
            category: DownloadCategory::Compressed,
            status: TaskStatus::Downloading,
            connections: 8,
            supports_range: true,
            is_hls: false,
            created_at: "2026-09-21 12:00:00".to_string(),
            completed_at: None,
            error_message: None,
            segments: vec![],
            referer: None,
            speed_limit_bps: None,
        };
        manager.tasks.write().await.insert(active_task.id.clone(), active_task.clone());

        let r1 = manager.check_duplicate_task("https://example.com/file.zip", "file.zip", &save_dir).await;
        assert!(r1.is_duplicate);
        assert_eq!(r1.status, Some(TaskStatus::Downloading));
        assert_eq!(r1.percent, 45.0);
        assert_eq!(r1.task_id, Some("task-active-1".to_string()));
        assert_eq!(r1.suggested_new_filename, Some("file (1).zip".to_string()));

        // Also check duplicate by colliding target path with different URL
        let r1_by_path = manager.check_duplicate_task("https://other-mirror.com/different-url.zip", "file.zip", &save_dir).await;
        assert!(r1_by_path.is_duplicate);
        assert_eq!(r1_by_path.status, Some(TaskStatus::Downloading));

        // 3. Completed task with physical file on disk vs deleted from disk
        manager.tasks.write().await.clear();
        let completed_file = temp.path().join("completed.iso");
        std::fs::write(&completed_file, b"ISO_DATA").unwrap();

        let completed_task = DownloadTask {
            id: "task-comp-1".to_string(),
            url: "https://example.com/completed.iso".to_string(),
            filename: "completed.iso".to_string(),
            save_dir: save_dir.clone(),
            file_path: completed_file.to_string_lossy().to_string(),
            total_bytes: Some(8),
            downloaded_bytes: 8,
            category: DownloadCategory::Compressed,
            status: TaskStatus::Completed,
            connections: 4,
            supports_range: true,
            is_hls: false,
            created_at: "2026-09-21 10:00:00".to_string(),
            completed_at: Some("2026-09-21 10:01:00".to_string()),
            error_message: None,
            segments: vec![],
            referer: None,
            speed_limit_bps: None,
        };
        db.insert_task(&completed_task).unwrap();

        // A. File exists on disk -> should detect duplicate completed
        let r2 = manager.check_duplicate_task("https://example.com/completed.iso", "completed.iso", &save_dir).await;
        assert!(r2.is_duplicate);
        assert_eq!(r2.status, Some(TaskStatus::Completed));
        assert!(r2.file_exists_on_disk);
        assert_eq!(r2.percent, 100.0);
        assert_eq!(r2.completed_at, Some("2026-09-21 10:01:00".to_string()));

        // B. Physically delete file from disk (user deleted via Explorer) -> must NOT be treated as duplicate!
        std::fs::remove_file(&completed_file).unwrap();
        let r3 = manager.check_duplicate_task("https://example.com/completed.iso", "completed.iso", &save_dir).await;
        assert!(!r3.is_duplicate, "Expected deleted file to not trigger duplicate warning");
    }
}



