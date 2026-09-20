use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadCategory {
    All,
    Compressed,
    Programs,
    Video,
    Audio,
    Documents,
    General,
}

impl DownloadCategory {
    pub fn from_filename(filename: &str) -> Self {
        let ext = filename
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();

        match ext.as_str() {
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "iso" | "dmg" => {
                DownloadCategory::Compressed
            }
            "exe" | "msi" | "apk" | "app" | "bat" | "cmd" | "ps1" | "deb" | "rpm" => {
                DownloadCategory::Programs
            }
            "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" | "wmv" | "m4v" | "ts" | "m3u8" => {
                DownloadCategory::Video
            }
            "mp3" | "flac" | "wav" | "aac" | "ogg" | "m4a" | "wma" | "opus" => {
                DownloadCategory::Audio
            }
            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "csv" | "epub" => {
                DownloadCategory::Documents
            }
            _ => DownloadCategory::General,
        }
    }

    pub fn default_subfolder(&self) -> &'static str {
        match self {
            DownloadCategory::Compressed => "Compressed",
            DownloadCategory::Programs => "Programs",
            DownloadCategory::Video => "Video",
            DownloadCategory::Audio => "Audio",
            DownloadCategory::Documents => "Documents",
            _ => "General",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Queued,
    Probing,
    Downloading,
    Paused,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub index: usize,
    pub start_byte: u64,
    pub end_byte: u64,
    pub downloaded_bytes: u64,
    pub is_finished: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTask {
    pub id: String,
    pub url: String,
    pub filename: String,
    pub save_dir: String,
    pub file_path: String,
    pub total_bytes: Option<u64>,
    pub downloaded_bytes: u64,
    pub category: DownloadCategory,
    pub status: TaskStatus,
    pub connections: usize,
    pub supports_range: bool,
    pub is_hls: bool,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeedMetrics {
    pub task_id: String,
    pub speed_bps: u64,
    pub eta_seconds: Option<u64>,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub percent: f64,
    pub status: TaskStatus,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub url: String,
    pub filename: String,
    pub total_bytes: Option<u64>,
    pub formatted_size: String,
    pub supports_range: bool,
    pub category: DownloadCategory,
    pub suggested_dir: String,
    pub is_hls: bool,
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    let b = bytes as f64;
    if b >= TB {
        format!("{:.2} TB", b / TB)
    } else if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.2} MB", b / MB)
    } else if b >= KB {
        format!("{:.2} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_detection() {
        // Video
        assert_eq!(DownloadCategory::from_filename("video.mp4"), DownloadCategory::Video);
        assert_eq!(DownloadCategory::from_filename("movie.MKV"), DownloadCategory::Video);
        assert_eq!(DownloadCategory::from_filename("clip.webm"), DownloadCategory::Video);
        assert_eq!(DownloadCategory::from_filename("stream.m3u8"), DownloadCategory::Video);
        assert_eq!(DownloadCategory::from_filename("segment.ts"), DownloadCategory::Video);

        // Compressed
        assert_eq!(DownloadCategory::from_filename("archive.tar.gz"), DownloadCategory::Compressed);
        assert_eq!(DownloadCategory::from_filename("bundle.zip"), DownloadCategory::Compressed);
        assert_eq!(DownloadCategory::from_filename("backup.7z"), DownloadCategory::Compressed);
        assert_eq!(DownloadCategory::from_filename("disk.iso"), DownloadCategory::Compressed);

        // Programs
        assert_eq!(DownloadCategory::from_filename("setup.exe"), DownloadCategory::Programs);
        assert_eq!(DownloadCategory::from_filename("installer.msi"), DownloadCategory::Programs);
        assert_eq!(DownloadCategory::from_filename("app.apk"), DownloadCategory::Programs);
        assert_eq!(DownloadCategory::from_filename("package.deb"), DownloadCategory::Programs);

        // Audio
        assert_eq!(DownloadCategory::from_filename("song.mp3"), DownloadCategory::Audio);
        assert_eq!(DownloadCategory::from_filename("album.flac"), DownloadCategory::Audio);
        assert_eq!(DownloadCategory::from_filename("audio.ogg"), DownloadCategory::Audio);

        // Documents
        assert_eq!(DownloadCategory::from_filename("paper.pdf"), DownloadCategory::Documents);
        assert_eq!(DownloadCategory::from_filename("notes.docx"), DownloadCategory::Documents);
        assert_eq!(DownloadCategory::from_filename("sheet.xlsx"), DownloadCategory::Documents);
        assert_eq!(DownloadCategory::from_filename("book.epub"), DownloadCategory::Documents);

        // General / Unknown
        assert_eq!(DownloadCategory::from_filename("data.unknown"), DownloadCategory::General);
        assert_eq!(DownloadCategory::from_filename("no_extension"), DownloadCategory::General);
        assert_eq!(DownloadCategory::from_filename(".hidden"), DownloadCategory::General);
    }

    #[test]
    fn test_category_subfolders() {
        assert_eq!(DownloadCategory::Compressed.default_subfolder(), "Compressed");
        assert_eq!(DownloadCategory::Programs.default_subfolder(), "Programs");
        assert_eq!(DownloadCategory::Video.default_subfolder(), "Video");
        assert_eq!(DownloadCategory::Audio.default_subfolder(), "Audio");
        assert_eq!(DownloadCategory::Documents.default_subfolder(), "Documents");
        assert_eq!(DownloadCategory::General.default_subfolder(), "General");
        assert_eq!(DownloadCategory::All.default_subfolder(), "General");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 512), "512.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 10), "10.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(format_bytes(1024 * 1024 * 1024 * 5), "5.00 GB");
        assert_eq!(format_bytes(1024 * 1024 * 1024 * 1024 * 2), "2.00 TB");
    }

    #[test]
    fn test_task_status_serde() {
        let statuses = vec![
            TaskStatus::Queued,
            TaskStatus::Probing,
            TaskStatus::Downloading,
            TaskStatus::Paused,
            TaskStatus::Completed,
            TaskStatus::Failed("Network timeout".to_string()),
        ];

        for s in statuses {
            let json = serde_json::to_string(&s).expect("serialize status");
            let deserialized: TaskStatus = serde_json::from_str(&json).expect("deserialize status");
            assert_eq!(s, deserialized);
        }
    }

    #[test]
    fn test_download_task_serde() {
        let task = DownloadTask {
            id: "task-123".to_string(),
            url: "https://example.com/file.zip".to_string(),
            filename: "file.zip".to_string(),
            save_dir: "C:\\Downloads".to_string(),
            file_path: "C:\\Downloads\\file.zip".to_string(),
            total_bytes: Some(1024),
            downloaded_bytes: 512,
            category: DownloadCategory::Compressed,
            status: TaskStatus::Downloading,
            connections: 4,
            supports_range: true,
            is_hls: false,
            created_at: "2026-09-20 12:00:00".to_string(),
            completed_at: None,
            error_message: None,
            segments: vec![
                Segment {
                    index: 0,
                    start_byte: 0,
                    end_byte: 511,
                    downloaded_bytes: 512,
                    is_finished: true,
                },
                Segment {
                    index: 1,
                    start_byte: 512,
                    end_byte: 1023,
                    downloaded_bytes: 0,
                    is_finished: false,
                },
            ],
        };

        let json = serde_json::to_string(&task).expect("serialize task");
        let decoded: DownloadTask = serde_json::from_str(&json).expect("deserialize task");
        assert_eq!(decoded.id, task.id);
        assert_eq!(decoded.filename, task.filename);
        assert_eq!(decoded.segments.len(), 2);
    }

    #[test]
    fn test_speed_metrics_serde() {
        let metrics = SpeedMetrics {
            task_id: "task-abc".to_string(),
            speed_bps: 1048576,
            eta_seconds: Some(60),
            downloaded_bytes: 52428800,
            total_bytes: Some(104857600),
            percent: 50.0,
            status: TaskStatus::Downloading,
            segments: vec![],
        };

        let json = serde_json::to_string(&metrics).expect("serialize metrics");
        let decoded: SpeedMetrics = serde_json::from_str(&json).expect("deserialize metrics");
        assert_eq!(decoded.task_id, "task-abc");
        assert_eq!(decoded.speed_bps, 1048576);
        assert_eq!(decoded.percent, 50.0);
    }
}

