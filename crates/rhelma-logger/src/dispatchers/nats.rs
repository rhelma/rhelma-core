//! NATS dispatcher for log events.
//!
//! Dispatches log events to a NATS subject.
//!
//! # Setup
//! Add to `crates/rhelma-logger/Cargo.toml`:
//! ```toml
//! [dependencies]
//! async-nats = "0.35"
//! tokio = { workspace = true, features = ["rt-multi-thread"] }
//! ```

#![forbid(unsafe_code)]

use std::sync::OnceLock;

use async_nats::Client;

use crate::event::LogEvent;
use crate::extensions::LogDispatcher;

// ---------------------------------------------------------------------------
// Lazy Tokio Runtime (same pattern as file_service.rs / redis.rs)
// ---------------------------------------------------------------------------

/// Runtime مستقل فقط در صورت نیاز ساخته می‌شود.
static RUNTIME: OnceLock<Result<tokio::runtime::Runtime, String>> = OnceLock::new();

fn rt() -> Option<&'static tokio::runtime::Runtime> {
    let res = RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .map_err(|e| format!("NatsDispatcher: failed to build tokio runtime: {e}"))
    });

    match res {
        Ok(rt) => Some(rt),
        Err(msg) => {
            crate::state::report_internal_error(msg);
            None
        }
    }
}

// ---------------------------------------------------------------------------
// NatsDispatcher
// ---------------------------------------------------------------------------

/// NATS based log dispatcher.
///
/// Sends log events to a NATS subject.
///
/// # Example
/// ```ignore
/// use rhelma_logger::dispatchers::nats::NatsDispatcher;
///
/// let dispatcher = NatsDispatcher::new(
///     "nats://127.0.0.1:4222",
///     "rhelma.logs",
/// );
/// ```
#[derive(Debug, Clone)]
pub struct NatsDispatcher {
    /// NATS server URL (e.g., "nats://127.0.0.1:4222")
    url: String,
    /// Subject to publish to (e.g., "rhelma.logs")
    subject: String,
}

impl NatsDispatcher {
    /// Create a new NATS dispatcher.
    ///
    /// # Arguments
    /// * `url` - NATS server URL (e.g., "nats://127.0.0.1:4222")
    /// * `subject` - The NATS subject to publish to (e.g., "rhelma.logs")
    pub fn new(url: impl Into<String>, subject: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            subject: subject.into(),
        }
    }

    /// Get the NATS URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Get the subject.
    pub fn subject(&self) -> &str {
        &self.subject
    }
}

impl LogDispatcher for NatsDispatcher {
    fn dispatch(&self, event: LogEvent) {
        let url = self.url.clone();
        let subject = self.subject.clone();

        if let Some(runtime) = rt() {
            runtime.spawn(async move {
                if let Err(e) = dispatch_to_nats(&url, &subject, &event).await {
                    // Use crate's internal error reporting to avoid recursive logging
                    crate::state::report_internal_error(&format!(
                        "NatsDispatcher: failed to publish to subject '{}': {}",
                        subject, e
                    ));
                }
            });
        }
    }

    fn flush(&self) {
        // NATS publishes are fire-and-forget by default.
        // No client-side buffering to flush.
    }

    fn box_clone(&self) -> Box<dyn LogDispatcher + Send + Sync> {
        Box::new(self.clone())
    }
}

/// Internal helper to dispatch event to NATS.
async fn dispatch_to_nats(
    url: &str,
    subject: &str,
    event: &LogEvent,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::connect(url).await?;

    let payload = serde_json::to_vec(event)?;
    client.publish(subject.to_string(), payload.into()).await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_create_dispatcher() {
        let dispatcher = NatsDispatcher::new("nats://127.0.0.1:4222", "rhelma.logs");
        assert_eq!(dispatcher.url(), "nats://127.0.0.1:4222");
        assert_eq!(dispatcher.subject(), "rhelma.logs");
    }

    #[test]
    fn can_clone_dispatcher() {
        let dispatcher = NatsDispatcher::new("nats://127.0.0.1:4222", "rhelma.logs");
        let cloned = dispatcher.box_clone();
        drop(cloned);
    }

    #[test]
    fn dispatcher_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<NatsDispatcher>();
    }
}
