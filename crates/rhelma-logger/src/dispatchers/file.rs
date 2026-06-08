use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use serde::Serialize;
use time::OffsetDateTime;

use crate::event::LogEvent;
use crate::extensions::LogDispatcher;

#[derive(Debug, Clone, Serialize)]
pub enum Rotation {
    /// Variant `Never`.
    Never,
    /// Variant `Daily`.
    Daily,
    /// Variant `Hourly`.
    Hourly,
}

/// کانفیگ FileDispatcher (مسیر + rotation)
#[derive(Debug, Clone, Serialize)]
pub struct FileDispatcherConfig {
    /// مثال:
    /// "logs/{service}-{date}.log"
    /// تاریخ human-readable جایگزین می‌شود.
    pub path_pattern: String,

    /// Daily / Hourly / Never
    pub rotation: Rotation,
}

/// Dispatcher برای نوشتن لاگ روی فایل.
#[derive(Debug, Clone)]
pub struct FileDispatcher {
    cfg: FileDispatcherConfig,
}

impl FileDispatcher {
    pub fn new(cfg: FileDispatcherConfig) -> Self {
        Self { cfg }
    }

    /// تاریخ human-readable مطابق rotation
    fn formatted_date(&self, now: &OffsetDateTime) -> String {
        match self.cfg.rotation {
            Rotation::Never => "current".to_string(),

            Rotation::Daily => format!(
                "{:04}-{:02}-{:02}",
                now.year(),
                now.month() as u8,
                now.day()
            ),

            Rotation::Hourly => format!(
                "{:04}-{:02}-{:02}-{:02}",
                now.year(),
                now.month() as u8,
                now.day(),
                now.hour()
            ),
        }
    }

    /// مسیر جایگزینی‌شده با {service} و {date}
    fn current_path(&self, service: &str) -> PathBuf {
        let now = OffsetDateTime::now_utc();
        let date = self.formatted_date(&now);

        let rendered = self
            .cfg
            .path_pattern
            .replace("{service}", service)
            .replace("{date}", &date);

        PathBuf::from(rendered)
    }

    fn append_line(&self, path: &PathBuf, line: &str) -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| format!("open log file '{}': {e}", path.display()))?;

        file.write_all(line.as_bytes())
            .and_then(|_| file.write_all(b"\n"))
            .map_err(|e| format!("write log file '{}': {e}", path.display()))
    }
}

/// پیاده‌سازی dispatcher فایل
impl LogDispatcher for FileDispatcher {
    fn dispatch(&self, event: LogEvent) {
        let path = self.current_path(&event.service_name);

        let line = match serde_json::to_string(&event) {
            Ok(l) => l,
            Err(e) => {
                crate::state::report_internal_error(&format!("FileDispatcher encode error: {e}"));
                return;
            }
        };

        if let Err(e) = self.append_line(&path, &line) {
            crate::state::report_internal_error(&format!(
                "FileDispatcher write failed ({}): {}",
                path.display(),
                e
            ));
        }
    }

    /// 🔥 متد جدید: توسط worker برای flush دوره‌ای فراخوانی می‌شود
    fn flush(&self) {
        // No-op: each write is append+close, so data is already flushed at OS boundaries.
        // Keeping this method avoids special-casing dispatcher types in the async worker.
    }

    fn box_clone(&self) -> Box<dyn LogDispatcher + Send + Sync> {
        Box::new(self.clone())
    }
}
