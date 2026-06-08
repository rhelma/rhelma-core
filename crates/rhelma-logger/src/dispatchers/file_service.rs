#![cfg(feature = "io")]

use std::sync::OnceLock;
use std::time::Duration;

use crate::event::LogEvent;
use crate::extensions::LogDispatcher;
#[cfg(feature = "io")]
use rhelma_http_observability::reqwest::ReqwestRequestBuilderExt;

/// Runtime مستقل فقط در صورت نیاز ساخته می‌شود.
/// از tokio برای ارسال async استفاده می‌کنیم.
static RUNTIME: OnceLock<Result<tokio::runtime::Runtime, String>> = OnceLock::new();

fn rt() -> Option<&'static tokio::runtime::Runtime> {
    let res = RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .map_err(|e| format!("FileServiceDispatcher: failed to build tokio runtime: {e}"))
    });

    match res {
        Ok(rt) => Some(rt),
        Err(msg) => {
            crate::state::report_internal_error(msg);
            None
        }
    }
}

/// Dispatcher برای ارسال لاگ‌ها به FileService / HTTP
#[derive(Debug, Clone)]
pub struct FileServiceDispatcher {
    /// Field `endpoint`.
    pub endpoint: String,
    /// Field `api_key`.
    pub api_key: Option<String>,
    /// Field `timeout`.
    pub timeout: Duration,
}

impl FileServiceDispatcher {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            api_key: None,
            timeout: Duration::from_secs(3),
        }
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout = Duration::from_secs(secs);
        self
    }
}

impl LogDispatcher for FileServiceDispatcher {
    fn dispatch(&self, event: LogEvent) {
        // 1) Encode JSON
        let body = match serde_json::to_vec(&event) {
            Ok(b) => b,
            Err(e) => {
                crate::state::report_internal_error(&format!(
                    "FileServiceDispatcher encode failed: {e}"
                ));
                return;
            }
        };

        // Clone data برای ارسال در async task
        let endpoint = self.endpoint.clone();
        let api_key = self.api_key.clone();
        let timeout = self.timeout;

        // 2) اجرای async در runtime جداگانه (non-blocking)
        let Some(rt) = rt() else {
            return;
        };
        rt.spawn(async move {
            let client = match reqwest::Client::builder().timeout(timeout).build() {
                Ok(c) => c,
                Err(e) => {
                    crate::state::report_internal_error(&format!(
                        "FileServiceDispatcher: http client build failed: {e}"
                    ));
                    return;
                }
            };

            let mut req = client.post(&endpoint).body(body);

            if let Some(key) = api_key {
                req = req.header("X-API-Key", key);
            }

            // 3) ارسال request
            match req.with_rhelma_observability().send().await {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        crate::state::report_internal_error(&format!(
                            "FileServiceDispatcher: HTTP {} from {}",
                            resp.status(),
                            endpoint
                        ));
                    }
                }
                Err(e) => {
                    crate::state::report_internal_error(&format!(
                        "FileServiceDispatcher: request failed: {e}"
                    ));
                }
            }
        });
    }

    /// 🔥 NEW: نیاز برای flush_interval_ms و shutdown
    fn flush(&self) {
        // FileServiceDispatcher هیچ state داخلی ندارد
        // ولی داشتن متد خالی ضروری است
        // چون worker در shutdown آن را فراخوانی می‌کند
    }

    fn box_clone(&self) -> Box<dyn LogDispatcher + Send + Sync> {
        Box::new(self.clone())
    }
}
