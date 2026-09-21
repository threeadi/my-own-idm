use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc::Sender;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

use crate::engine::limiter::TokenBucketRateLimiter;
use crate::engine::types::{format_bytes, DownloadCategory, ProbeResult};

#[derive(Debug, PartialEq, Eq)]
pub enum StreamOutcome {
    Completed,
    Cancelled,
    RateChanged(Option<u64>),
}

pub struct YtDlpRunner;

impl YtDlpRunner {
    pub fn find_yt_dlp() -> Result<PathBuf, String> {
        // 1. Check relative to current exe
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let p = dir.join("bin").join("yt-dlp.exe");
                if p.exists() {
                    return Ok(p);
                }
                let p2 = dir.join("yt-dlp.exe");
                if p2.exists() {
                    return Ok(p2);
                }
            }
        }

        // 2. Check project bin folder
        let dev_bin = PathBuf::from(r"d:\Development\my-own-idm\src-tauri\bin\yt-dlp.exe");
        if dev_bin.exists() {
            return Ok(dev_bin);
        }

        // 3. Check WinGet links
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let winget_p = PathBuf::from(local_app_data)
                .join("Microsoft")
                .join("WinGet")
                .join("Links")
                .join("yt-dlp.exe");
            if winget_p.exists() {
                return Ok(winget_p);
            }
        }

        // 4. Fallback to PATH
        Ok(PathBuf::from("yt-dlp"))
    }

    pub fn find_ffmpeg_dir() -> Option<PathBuf> {
        // 1. Check project bin
        let dev_bin = PathBuf::from(r"d:\Development\my-own-idm\src-tauri\bin");
        if dev_bin.join("ffmpeg.exe").exists() {
            return Some(dev_bin);
        }

        // 2. Check WinGet Gyan FFmpeg
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let winget_dir = PathBuf::from(&local_app_data)
                .join("Microsoft")
                .join("WinGet")
                .join("Packages");
            if let Ok(entries) = std::fs::read_dir(&winget_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.contains("FFmpeg") {
                        let candidate = entry.path().join("bin");
                        if candidate.join("ffmpeg.exe").exists() {
                            return Some(candidate);
                        }
                        // Check subdirectories
                        if let Ok(sub) = std::fs::read_dir(entry.path()) {
                            for s in sub.flatten() {
                                let bin_dir = s.path().join("bin");
                                if bin_dir.join("ffmpeg.exe").exists() {
                                    return Some(bin_dir);
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    pub async fn probe(url: &str) -> Result<ProbeResult, String> {
        let yt_dlp_exe = Self::find_yt_dlp()?;

        let mut cmd = Command::new(&yt_dlp_exe);
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        cmd.arg("--no-warnings")
            .arg("--no-playlist")
            .arg("--print")
            .arg("%(title)s\t%(filesize,filesize_approx)s")
            .arg(url)
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        if let Some(ff_dir) = Self::find_ffmpeg_dir() {
            cmd.arg("--ffmpeg-location").arg(ff_dir);
        }

        let output = cmd
            .output()
            .await
            .map_err(|e| format!("Failed to run yt-dlp: {}", e))?;

        if !output.status.success() {
            return Err("yt-dlp could not extract stream from URL".to_string());
        }

        let out_str = String::from_utf8_lossy(&output.stdout);
        Ok(Self::parse_probe_output(&out_str, url))
    }

    pub fn parse_probe_output(out_str: &str, url: &str) -> ProbeResult {
        let trimmed = out_str.trim();
        let parts: Vec<&str> = trimmed.split('\t').collect();

        let raw_title = parts.first().copied().unwrap_or("YouTube Video");
        let raw_size = parts.get(1).copied().unwrap_or("NA");

        let total_bytes = raw_size.parse::<u64>().ok();
        let clean_title = crate::engine::probe::sanitize_filename(raw_title);
        let filename = format!("{}.mp4", clean_title);

        let formatted_size = match total_bytes {
            Some(b) => format_bytes(b),
            None => "Unknown size".to_string(),
        };

        let default_download_dir = dirs_fallback()
            .join(DownloadCategory::Video.default_subfolder())
            .to_string_lossy()
            .to_string();

        ProbeResult {
            url: url.to_string(),
            filename,
            total_bytes,
            formatted_size,
            supports_range: true,
            category: DownloadCategory::Video,
            suggested_dir: default_download_dir,
            is_hls: false,
        }
    }

    pub fn parse_progress_line(line: &str) -> Option<u64> {
        if let Some(stripped) = line.strip_prefix("IDM_PROGRESS:") {
            stripped.trim().parse::<u64>().ok()
        } else {
            None
        }
    }

    pub fn compute_effective_limit(
        task_limiter: &Option<Arc<TokenBucketRateLimiter>>,
        global_limiter: &Option<Arc<TokenBucketRateLimiter>>,
    ) -> Option<u64> {
        match (
            task_limiter.as_ref().map(|l| l.get_limit_bps()).filter(|&b| b > 0),
            global_limiter.as_ref().map(|l| l.get_limit_bps()).filter(|&b| b > 0),
        ) {
            (Some(t), Some(g)) => Some(t.min(g)),
            (Some(t), None) => Some(t),
            (None, Some(g)) => Some(g),
            (None, None) => None,
        }
    }

    pub fn build_format_selector(quality: Option<&str>, filename: &str) -> String {
        let q_str = quality
            .filter(|q| !q.trim().is_empty())
            .map(|q| q.to_ascii_lowercase())
            .unwrap_or_else(|| {
                let fn_lower = filename.to_ascii_lowercase();
                if fn_lower.contains("2160p") || fn_lower.contains("4k") {
                    "2160p".to_string()
                } else if fn_lower.contains("1080p") {
                    "1080p".to_string()
                } else if fn_lower.contains("720p") {
                    "720p".to_string()
                } else if fn_lower.contains("480p") {
                    "480p".to_string()
                } else if fn_lower.contains("360p") {
                    "360p".to_string()
                } else if fn_lower.contains("audio") {
                    "audio".to_string()
                } else {
                    String::new()
                }
            });

        if q_str.contains("2160") || q_str.contains("4k") {
            "bv*[height<=2160]+ba/b[height<=2160]/best".to_string()
        } else if q_str.contains("1080") {
            "bv*[height<=1080]+ba/b[height<=1080]/best".to_string()
        } else if q_str.contains("720") {
            "bv*[height<=720]+ba/b[height<=720]/best".to_string()
        } else if q_str.contains("480") {
            "bv*[height<=480]+ba/b[height<=480]/best".to_string()
        } else if q_str.contains("360") {
            "bv*[height<=360]+ba/b[height<=360]/best".to_string()
        } else if q_str.contains("audio") {
            "ba/b".to_string()
        } else {
            "bv*+ba/b/best".to_string()
        }
    }

    pub async fn run_download(
        url: String,
        output_file: String,
        headers: Option<HashMap<String, String>>,
        cancel_flag: Arc<AtomicBool>,
        progress_tx: Sender<(usize, u64)>,
        task_limiter: Option<Arc<TokenBucketRateLimiter>>,
        global_limiter: Option<Arc<TokenBucketRateLimiter>>,
        quality: Option<String>,
    ) -> Result<(), String> {
        let yt_dlp_exe = Self::find_yt_dlp()?;
        let mut last_bytes: u64 = 0;
        let mut current_rate = Self::compute_effective_limit(&task_limiter, &global_limiter);
        let format_selector = Self::build_format_selector(quality.as_deref(), &output_file);

        loop {
            if cancel_flag.load(Ordering::Relaxed) {
                return Ok(());
            }

            let mut cmd = Command::new(&yt_dlp_exe);
            #[cfg(windows)]
            cmd.creation_flags(CREATE_NO_WINDOW);

            cmd.env("PYTHONUNBUFFERED", "1");

            cmd.arg("--no-warnings")
                .arg("--no-playlist")
                .arg("--newline")
                .arg("-c") // --continue: resume partially downloaded video files
                .arg("-f")
                .arg(&format_selector)
                .arg("--merge-output-format")
                .arg("mp4")
                .arg("--progress-delta")
                .arg("0.2")
                .arg("--progress-template")
                .arg("IDM_PROGRESS:%(progress.downloaded_bytes)s")
                .arg("-o")
                .arg(&output_file);

            if let Some(ref hdrs) = headers {
                for (k, v) in hdrs {
                    let k_lower = k.to_ascii_lowercase();
                    if k_lower == "referer" && !v.is_empty() {
                        cmd.arg("--add-header").arg(format!("{}: {}", k, v));
                    }
                }
            }

            if let Some(rate) = current_rate {
                if rate > 0 {
                    cmd.arg("--limit-rate").arg(format!("{}", rate));
                }
            }

            cmd.arg(&url)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            if let Some(ff_dir) = Self::find_ffmpeg_dir() {
                cmd.arg("--ffmpeg-location").arg(ff_dir);
            }

            let mut child = cmd
                .spawn()
                .map_err(|e| format!("Failed to spawn yt-dlp: {}", e))?;

            let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
            let stderr = child.stderr.take();
            let reader = BufReader::new(stdout).lines();

            let err_task = tokio::spawn(async move {
                let mut last_lines = Vec::new();
                if let Some(err) = stderr {
                    let mut lines = BufReader::new(err).lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            if last_lines.len() >= 10 {
                                last_lines.remove(0);
                            }
                            last_lines.push(trimmed.to_string());
                        }
                    }
                }
                last_lines.join(" | ")
            });

            let task_notify = task_limiter.as_ref().map(|l| l.get_notify());
            let global_notify = global_limiter.as_ref().map(|l| l.get_notify());

            let stream_outcome = Self::process_progress_stream_with_rate_check(
                reader,
                &output_file,
                cancel_flag.clone(),
                progress_tx.clone(),
                &mut last_bytes,
                &task_limiter,
                &global_limiter,
                current_rate,
                task_notify,
                global_notify,
            )
            .await;

            match stream_outcome {
                StreamOutcome::Completed => {
                    let status = child
                        .wait()
                        .await
                        .map_err(|e| format!("Failed to wait for yt-dlp: {}", e))?;

                    let stderr_output = err_task.await.unwrap_or_default();

                    if status.success() {
                        return Ok(());
                    } else {
                        let msg = if !stderr_output.is_empty() {
                            format!("yt-dlp error: {}", stderr_output)
                        } else {
                            "yt-dlp download failed".to_string()
                        };
                        crate::log_error!("ytdlp", "{}", msg);
                        return Err(msg);
                    }
                }
                StreamOutcome::Cancelled => {
                    let _ = child.kill().await;
                    let _ = err_task.await;
                    return Ok(());
                }
                StreamOutcome::RateChanged(new_rate) => {
                    crate::log_info!("ytdlp", "Speed limit dynamically changed to {:?} bps, restarting stream with resume", new_rate);
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                    let _ = err_task.await;
                    current_rate = new_rate;
                    tokio::time::sleep(Duration::from_millis(150)).await;
                    continue;
                }
            }
        }
    }

    pub async fn process_progress_stream_with_rate_check<R: tokio::io::AsyncBufRead + Unpin>(
        mut reader: tokio::io::Lines<R>,
        output_file: &str,
        cancel_flag: Arc<AtomicBool>,
        progress_tx: Sender<(usize, u64)>,
        last_bytes: &mut u64,
        task_limiter: &Option<Arc<TokenBucketRateLimiter>>,
        global_limiter: &Option<Arc<TokenBucketRateLimiter>>,
        current_rate: Option<u64>,
        task_notify: Option<Arc<tokio::sync::Notify>>,
        global_notify: Option<Arc<tokio::sync::Notify>>,
    ) -> StreamOutcome {
        let part_path = if !output_file.is_empty() {
            format!("{}.part", output_file)
        } else {
            String::new()
        };
        let out_path = output_file.to_string();

        loop {
            if cancel_flag.load(Ordering::Relaxed) {
                return StreamOutcome::Cancelled;
            }

            let new_rate = Self::compute_effective_limit(task_limiter, global_limiter);
            if new_rate != current_rate {
                return StreamOutcome::RateChanged(new_rate);
            }

            let task_fut = async {
                if let Some(n) = &task_notify {
                    n.notified().await;
                } else {
                    std::future::pending::<()>().await;
                }
            };

            let global_fut = async {
                if let Some(n) = &global_notify {
                    n.notified().await;
                } else {
                    std::future::pending::<()>().await;
                }
            };

            tokio::select! {
                biased;

                line_res = reader.next_line() => {
                    match line_res {
                        Ok(Some(line)) => {
                            if let Some(bytes) = Self::parse_progress_line(&line) {
                                if bytes > *last_bytes {
                                    let delta = bytes - *last_bytes;
                                    *last_bytes = bytes;
                                    let _ = progress_tx.send((0, delta)).await;
                                } else if *last_bytes == 0 && bytes > 0 {
                                    *last_bytes = bytes;
                                    let _ = progress_tx.send((0, bytes)).await;
                                }
                            }
                        }
                        Ok(None) => break,
                        Err(_) => break,
                    }
                }
                _ = task_fut => {
                    let check_rate = Self::compute_effective_limit(task_limiter, global_limiter);
                    if check_rate != current_rate {
                        return StreamOutcome::RateChanged(check_rate);
                    }
                }
                _ = global_fut => {
                    let check_rate = Self::compute_effective_limit(task_limiter, global_limiter);
                    if check_rate != current_rate {
                        return StreamOutcome::RateChanged(check_rate);
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(200)) => {
                    if cancel_flag.load(Ordering::Relaxed) {
                        return StreamOutcome::Cancelled;
                    }
                    let check_rate = Self::compute_effective_limit(task_limiter, global_limiter);
                    if check_rate != current_rate {
                        return StreamOutcome::RateChanged(check_rate);
                    }

                    if !output_file.is_empty() {
                        let disk_len = if let Ok(m) = tokio::fs::metadata(&part_path).await {
                            m.len()
                        } else if let Ok(m) = tokio::fs::metadata(&out_path).await {
                            m.len()
                        } else {
                            0
                        };
                        if disk_len > *last_bytes {
                            let delta = disk_len - *last_bytes;
                            *last_bytes = disk_len;
                            let _ = progress_tx.send((0, delta)).await;
                        }
                    }
                }
            }
        }

        if !output_file.is_empty() {
            let disk_len = if let Ok(m) = tokio::fs::metadata(&part_path).await {
                m.len()
            } else if let Ok(m) = tokio::fs::metadata(&out_path).await {
                m.len()
            } else {
                0
            };
            if disk_len > *last_bytes {
                let delta = disk_len - *last_bytes;
                *last_bytes = disk_len;
                let _ = progress_tx.send((0, delta)).await;
            }
        }

        StreamOutcome::Completed
    }

    pub async fn process_progress_stream<R: tokio::io::AsyncBufRead + Unpin>(
        reader: tokio::io::Lines<R>,
        cancel_flag: Arc<AtomicBool>,
        progress_tx: Sender<(usize, u64)>,
    ) -> bool {
        let mut last_bytes = 0;
        let outcome = Self::process_progress_stream_with_rate_check(
            reader,
            "",
            cancel_flag,
            progress_tx,
            &mut last_bytes,
            &None,
            &None,
            None,
            None,
            None,
        )
        .await;
        outcome == StreamOutcome::Completed
    }
}

fn dirs_fallback() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .map(|p| p.join("Downloads"))
            .unwrap_or_else(|| PathBuf::from("C:\\Downloads"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|p| p.join("Downloads"))
            .unwrap_or_else(|| PathBuf::from("~/Downloads"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_probe_output_with_filesize() {
        let raw = "Taylor Swift - Cruel Summer (Official Music Video)\t45678900\n";
        let probe = YtDlpRunner::parse_probe_output(raw, "https://youtube.com/watch?v=123");

        assert_eq!(probe.filename, "Taylor Swift - Cruel Summer (Official Music Video).mp4");
        assert_eq!(probe.total_bytes, Some(45678900));
        assert_eq!(probe.category, DownloadCategory::Video);
        assert!(probe.formatted_size.contains("MB"));
    }

    #[test]
    fn test_parse_probe_output_na_filesize() {
        let raw = "Live Stream Video\tNA\n";
        let probe = YtDlpRunner::parse_probe_output(raw, "https://youtube.com/watch?v=live");

        assert_eq!(probe.filename, "Live Stream Video.mp4");
        assert_eq!(probe.total_bytes, None);
        assert_eq!(probe.formatted_size, "Unknown size");
    }

    #[test]
    fn test_parse_progress_line() {
        assert_eq!(YtDlpRunner::parse_progress_line("IDM_PROGRESS:1024"), Some(1024));
        assert_eq!(YtDlpRunner::parse_progress_line("IDM_PROGRESS:  5000000 "), Some(5000000));
        assert_eq!(YtDlpRunner::parse_progress_line("[download] 50% of 10MiB at 2MB/s"), None);
        assert_eq!(YtDlpRunner::parse_progress_line("IDM_PROGRESS:invalid"), None);
    }

    #[test]
    fn test_find_yt_dlp() {
        let res = YtDlpRunner::find_yt_dlp();
        assert!(res.is_ok());
        let path = res.unwrap();
        assert!(!path.to_string_lossy().is_empty());
    }

    #[test]
    fn test_find_ffmpeg_dir() {
        let res = YtDlpRunner::find_ffmpeg_dir();
        // Just verify it doesn't crash and returns valid optional path
        if let Some(ref p) = res {
            assert!(p.exists());
        }
    }

    #[tokio::test]
    async fn test_process_progress_stream() {
        let simulated_output = b"IDM_PROGRESS:1000\n[download] ignore line\nIDM_PROGRESS:3000\nIDM_PROGRESS:5000\n";
        let reader = BufReader::new(&simulated_output[..]).lines();
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, mut rx) = tokio::sync::mpsc::channel(10);

        let completed = YtDlpRunner::process_progress_stream(reader, cancel, tx).await;
        assert!(completed);

        // Expect deltas: 1000, 2000, 2000
        let mut deltas = Vec::new();
        while let Ok((_, delta)) = rx.try_recv() {
            deltas.push(delta);
        }

        assert_eq!(deltas, vec![1000, 2000, 2000]);
    }

    #[tokio::test]
    async fn test_process_progress_stream_cancelled() {
        let simulated_output = b"IDM_PROGRESS:100\n";
        let reader = BufReader::new(&simulated_output[..]).lines();
        let cancel = Arc::new(AtomicBool::new(true)); // Pre-cancelled
        let (tx, _rx) = tokio::sync::mpsc::channel(10);

        let completed = YtDlpRunner::process_progress_stream(reader, cancel, tx).await;
        assert!(!completed);
    }

    #[test]
    fn test_dirs_fallback() {
        let dir = dirs_fallback();
        assert!(!dir.to_string_lossy().is_empty());
        assert!(dir.to_string_lossy().contains("Downloads"));
    }

    #[tokio::test]
    async fn test_ytdlp_probe_invalid_url() {
        let res = YtDlpRunner::probe("http://127.0.0.1:9999/invalid_stream").await;
        assert!(res.is_err());
    }

    #[test]
    fn test_compute_effective_limit() {
        let task_limiter_500k = Some(Arc::new(TokenBucketRateLimiter::new(500_000)));
        let global_limiter_1m = Some(Arc::new(TokenBucketRateLimiter::new(1_000_000)));
        let global_limiter_200k = Some(Arc::new(TokenBucketRateLimiter::new(200_000)));
        let unlimited = Some(Arc::new(TokenBucketRateLimiter::new(0)));

        // Both set: min applies
        assert_eq!(
            YtDlpRunner::compute_effective_limit(&task_limiter_500k, &global_limiter_1m),
            Some(500_000)
        );
        assert_eq!(
            YtDlpRunner::compute_effective_limit(&task_limiter_500k, &global_limiter_200k),
            Some(200_000)
        );

        // Only task limit set
        assert_eq!(
            YtDlpRunner::compute_effective_limit(&task_limiter_500k, &None),
            Some(500_000)
        );

        // Only global limit set
        assert_eq!(
            YtDlpRunner::compute_effective_limit(&None, &global_limiter_1m),
            Some(1_000_000)
        );

        // Both unlimited / None
        assert_eq!(
            YtDlpRunner::compute_effective_limit(&unlimited, &None),
            None
        );
        assert_eq!(
            YtDlpRunner::compute_effective_limit(&None, &None),
            None
        );
    }

    #[tokio::test]
    async fn test_process_progress_stream_rate_changed_notification() {
        let (client, _server) = tokio::io::duplex(64);
        let reader = BufReader::new(client).lines();
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, _rx) = tokio::sync::mpsc::channel(10);
        let mut last_bytes = 0;

        let task_limiter = Arc::new(TokenBucketRateLimiter::new(500_000));
        let task_notify = task_limiter.get_notify();

        let lim_clone = task_limiter.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            lim_clone.set_limit_bps(1_000_000);
        });

        let outcome = YtDlpRunner::process_progress_stream_with_rate_check(
            reader,
            "",
            cancel,
            tx,
            &mut last_bytes,
            &Some(task_limiter),
            &None,
            Some(500_000),
            Some(task_notify),
            None,
        ).await;

        assert_eq!(outcome, StreamOutcome::RateChanged(Some(1_000_000)));
    }

    #[tokio::test]
    async fn test_process_progress_stream_with_rate_check_progress_deltas() {
        let simulated_output = b"IDM_PROGRESS:5000\nIDM_PROGRESS:12000\n";
        let reader = BufReader::new(&simulated_output[..]).lines();
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, mut rx) = tokio::sync::mpsc::channel(10);
        let mut last_bytes = 0;

        let outcome = YtDlpRunner::process_progress_stream_with_rate_check(
            reader,
            "",
            cancel,
            tx,
            &mut last_bytes,
            &None,
            &None,
            None,
            None,
            None,
        ).await;

        assert_eq!(outcome, StreamOutcome::Completed);
        assert_eq!(last_bytes, 12000);

        let mut deltas = Vec::new();
        while let Ok((_, delta)) = rx.try_recv() {
            deltas.push(delta);
        }
        assert_eq!(deltas, vec![5000, 7000]);
    }

    #[tokio::test]
    async fn test_ytdlp_run_download_immediate_cancel() {
        let (tx, _rx) = tokio::sync::mpsc::channel(10);
        let cancel = Arc::new(AtomicBool::new(true)); // Pre-cancelled
        let temp = tempfile::tempdir().unwrap();
        let out = temp.path().join("out.mp4").to_string_lossy().to_string();
        let res = YtDlpRunner::run_download(
            "http://127.0.0.1:9999/dummy".to_string(),
            out,
            None,
            cancel,
            tx,
            Some(Arc::new(TokenBucketRateLimiter::new(500_000))),
            None,
            Some("720p".to_string()),
        ).await;
        assert!(res.is_ok());
    }

    #[test]
    fn test_build_format_selector() {
        // Explicit quality preset
        assert_eq!(
            YtDlpRunner::build_format_selector(Some("720p"), "video.mp4"),
            "bv*[height<=720]+ba/b[height<=720]/best"
        );
        assert_eq!(
            YtDlpRunner::build_format_selector(Some("1080p"), "video.mp4"),
            "bv*[height<=1080]+ba/b[height<=1080]/best"
        );
        assert_eq!(
            YtDlpRunner::build_format_selector(Some("480p"), "video.mp4"),
            "bv*[height<=480]+ba/b[height<=480]/best"
        );
        assert_eq!(
            YtDlpRunner::build_format_selector(Some("360p"), "video.mp4"),
            "bv*[height<=360]+ba/b[height<=360]/best"
        );
        assert_eq!(
            YtDlpRunner::build_format_selector(Some("2160p"), "video.mp4"),
            "bv*[height<=2160]+ba/b[height<=2160]/best"
        );
        assert_eq!(
            YtDlpRunner::build_format_selector(Some("4k"), "video.mp4"),
            "bv*[height<=2160]+ba/b[height<=2160]/best"
        );
        assert_eq!(
            YtDlpRunner::build_format_selector(Some("audio"), "song.mp4"),
            "ba/b"
        );

        // Quality inferred from filename when quality is None or empty
        assert_eq!(
            YtDlpRunner::build_format_selector(None, "Embed_720p.mp4"),
            "bv*[height<=720]+ba/b[height<=720]/best"
        );
        assert_eq!(
            YtDlpRunner::build_format_selector(Some(""), "Movie_1080p.mkv"),
            "bv*[height<=1080]+ba/b[height<=1080]/best"
        );
        assert_eq!(
            YtDlpRunner::build_format_selector(None, "Stream_480p.mp4"),
            "bv*[height<=480]+ba/b[height<=480]/best"
        );
        assert_eq!(
            YtDlpRunner::build_format_selector(None, "Podcast_audio.mp3"),
            "ba/b"
        );

        // Fallback default
        assert_eq!(
            YtDlpRunner::build_format_selector(None, "regular_download.mp4"),
            "bv*+ba/b/best"
        );
    }
}


