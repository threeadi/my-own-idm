use chrono::Local;
use parking_lot::Mutex;
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

static LOGGER: OnceLock<AppLogger> = OnceLock::new();

pub struct AppLogger {
    log_file: Mutex<Option<File>>,
    log_path: PathBuf,
}

impl AppLogger {
    fn new() -> Self {
        let log_dir = get_log_dir();
        let _ = create_dir_all(&log_dir);
        let log_path = log_dir.join("idm.log");

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&log_path)
            .ok();

        let logger = AppLogger {
            log_file: Mutex::new(file),
            log_path,
        };

        logger.log(
            "INFO",
            "system",
            &format!("=== My Own IDM Started (PID {}) ===", std::process::id()),
        );

        logger
    }

    pub fn init() -> &'static AppLogger {
        LOGGER.get_or_init(Self::new)
    }

    pub fn get() -> &'static AppLogger {
        LOGGER.get_or_init(Self::new)
    }

    pub fn log_path(&self) -> PathBuf {
        self.log_path.clone()
    }

    pub fn log(&self, level: &str, target: &str, message: &str) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let line = format!("[{}] [{:<5}] [{}] {}\n", timestamp, level, target, message);

        // 1. Print to console for dev
        print!("{}", line);

        // 2. Append to persistent log file
        let mut guard = self.log_file.lock();
        if let Some(ref mut f) = *guard {
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
        }
    }

    pub fn get_recent_lines(&self, max_lines: usize) -> Vec<String> {
        if let Ok(content) = std::fs::read_to_string(&self.log_path) {
            let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
            let start = lines.len().saturating_sub(max_lines);
            lines[start..].to_vec()
        } else {
            Vec::new()
        }
    }
}

pub fn get_log_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata)
                .join("com.myownidm.app")
                .join("logs");
        }
    }
    PathBuf::from("logs")
}

#[macro_export]
macro_rules! log_info {
    ($target:expr, $($arg:tt)*) => {
        $crate::logger::AppLogger::get().log("INFO", $target, &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($target:expr, $($arg:tt)*) => {
        $crate::logger::AppLogger::get().log("WARN", $target, &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($target:expr, $($arg:tt)*) => {
        $crate::logger::AppLogger::get().log("ERROR", $target, &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($target:expr, $($arg:tt)*) => {
        $crate::logger::AppLogger::get().log("DEBUG", $target, &format!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_singleton() {
        let logger1 = AppLogger::get();
        let logger2 = AppLogger::get();
        assert_eq!(logger1.log_path(), logger2.log_path());
    }

    #[test]
    fn test_logging_and_recent_lines() {
        let logger = AppLogger::get();
        let unique_msg = format!("Test log entry {}", uuid::Uuid::new_v4());
        logger.log("INFO", "test_target", &unique_msg);

        let lines = logger.get_recent_lines(50);
        assert!(!lines.is_empty());
        let found = lines.iter().any(|l| l.contains(&unique_msg) && l.contains("[INFO ]") && l.contains("[test_target]"));
        assert!(found, "Expected log message to be in recent lines");
    }

    #[test]
    fn test_concurrent_logging() {
        let handles: Vec<_> = (0..10)
            .map(|i| {
                std::thread::spawn(move || {
                    let logger = AppLogger::get();
                    logger.log("DEBUG", "thread_test", &format!("Message from thread {}", i));
                })
            })
            .collect();

        for h in handles {
            h.join().expect("thread join");
        }
    }

    #[test]
    fn test_log_dir() {
        let dir = get_log_dir();
        assert!(dir.to_string_lossy().len() > 0);
    }
}
