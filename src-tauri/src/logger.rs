use chrono::Local;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::fs::{create_dir_all, File};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::Context;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter, Layer};

static LOGGER: OnceLock<AppLogger> = OnceLock::new();

#[derive(Clone)]
pub struct RingBuffer {
    buffer: Arc<Mutex<VecDeque<String>>>,
    max_capacity: usize,
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            max_capacity: capacity,
        }
    }

    pub fn push(&self, entry: String) {
        let mut guard = self.buffer.lock();
        if guard.len() >= self.max_capacity {
            guard.pop_front();
        }
        guard.push_back(entry);
    }

    pub fn get_recent(&self, max_lines: usize) -> Vec<String> {
        let guard = self.buffer.lock();
        let len = guard.len();
        let start = len.saturating_sub(max_lines);
        guard.range(start..).cloned().collect()
    }
}

pub struct RingBufferLayer {
    ring: RingBuffer,
}

impl RingBufferLayer {
    pub fn new(ring: RingBuffer) -> Self {
        Self { ring }
    }
}

struct EventVisitor<'a> {
    message: &'a mut String,
}

impl<'a> Visit for EventVisitor<'a> {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            use std::fmt::Write;
            let _ = write!(self.message, "{:?}", value);
        } else {
            use std::fmt::Write;
            if !self.message.is_empty() {
                self.message.push(' ');
            }
            let _ = write!(self.message, "{}={:?}", field.name(), value);
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message.push_str(value);
        } else {
            if !self.message.is_empty() {
                self.message.push(' ');
            }
            self.message.push_str(field.name());
            self.message.push('=');
            self.message.push_str(value);
        }
    }
}

impl<S: Subscriber> Layer<S> for RingBufferLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let level = event.metadata().level().as_str();
        let target = event.metadata().target();

        let mut message = String::new();
        let mut visitor = EventVisitor {
            message: &mut message,
        };
        event.record(&mut visitor);

        let line = format!("[{}] [{:<5}] [{}] {}", timestamp, level, target, message);
        self.ring.push(line);
    }
}

pub struct AppLogger {
    ring: RingBuffer,
    _guard: Mutex<Option<WorkerGuard>>,
    log_dir: PathBuf,
}

impl AppLogger {
    fn new() -> Self {
        let log_dir = get_log_dir();
        let _ = create_dir_all(&log_dir);

        let ring = RingBuffer::new(200);

        // Pre-populate ring buffer from today's log file if it exists
        let today = Local::now().format("%Y-%m-%d").to_string();
        let today_log = log_dir.join(format!("idm.log.{}", today));
        let fallback_log = log_dir.join("idm.log");
        let initial_file = if today_log.exists() {
            Some(today_log)
        } else if fallback_log.exists() {
            Some(fallback_log)
        } else {
            None
        };

        if let Some(file_path) = initial_file {
            if let Ok(file) = File::open(file_path) {
                let reader = BufReader::new(file);
                let lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();
                let start = lines.len().saturating_sub(100);
                for line in &lines[start..] {
                    ring.push(line.clone());
                }
            }
        }

        // Daily rolling file appender in %APPDATA%/logs/ (idm.log.YYYY-MM-DD)
        let file_appender = tracing_appender::rolling::daily(&log_dir, "idm.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info,tauri_app_lib=debug,reqwest=info,hyper=warn"));

        let subscriber = tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().with_writer(std::io::stdout))
            .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
            .with(RingBufferLayer::new(ring.clone()));

        // In test runners or multiple setups, set_global_default might already be configured
        let _ = tracing::subscriber::set_global_default(subscriber);

        let logger = AppLogger {
            ring,
            _guard: Mutex::new(Some(guard)),
            log_dir,
        };

        tracing::info!(
            target: "system",
            "=== My Own IDM Hybrid Logger Started (PID {}) ===",
            std::process::id()
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
        let today = Local::now().format("%Y-%m-%d").to_string();
        let today_path = self.log_dir.join(format!("idm.log.{}", today));
        if today_path.exists() {
            today_path
        } else {
            self.log_dir.join("idm.log")
        }
    }

    pub fn log(&self, level: &str, target: &str, message: &str) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let line = format!("[{}] [{:<5}] [{}] {}", timestamp, level.to_uppercase(), target, message);
        self.ring.push(line);
    }

    pub fn get_recent_lines(&self, max_lines: usize) -> Vec<String> {
        self.ring.get_recent(max_lines)
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
        ::tracing::info!(target: $target, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_warn {
    ($target:expr, $($arg:tt)*) => {
        ::tracing::warn!(target: $target, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_error {
    ($target:expr, $($arg:tt)*) => {
        ::tracing::error!(target: $target, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_debug {
    ($target:expr, $($arg:tt)*) => {
        ::tracing::debug!(target: $target, $($arg)*)
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
        log_info!("test_target", "{}", unique_msg);

        let lines = logger.get_recent_lines(50);
        assert!(!lines.is_empty());
        let found = lines.iter().any(|l| l.contains(&unique_msg) && l.contains("[INFO ]") && l.contains("[test_target]"));
        assert!(found, "Expected log message to be in recent lines, got: {:?}", lines);
    }

    #[test]
    fn test_concurrent_logging() {
        let handles: Vec<_> = (0..10)
            .map(|i| {
                std::thread::spawn(move || {
                    let logger = AppLogger::get();
                    logger.log("DEBUG", "thread_test", &format!("Message from thread {}", i));
                    log_debug!("thread_test", "Concurrent tracing {}", i);
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

    #[test]
    fn test_ring_buffer_capacity() {
        let ring = RingBuffer::new(5);
        for i in 0..10 {
            ring.push(format!("line {}", i));
        }
        let recent = ring.get_recent(10);
        assert_eq!(recent.len(), 5);
        assert_eq!(recent[0], "line 5");
        assert_eq!(recent[4], "line 9");
    }
}
