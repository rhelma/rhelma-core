//! Redis Stream dispatcher for log events.
//!
//! Dispatches log events to a Redis Stream using XADD command.
//!
//! # Setup
//! Add to `crates/rhelma-logger/Cargo.toml`:
//! ```toml
//! [dependencies]
//! redis = { workspace = true }
//! ```

// Explicitly import the external redis crate with an alias to avoid
// name collision with this module (which is also named `redis`).
extern crate redis as redis_crate;

use redis_crate::aio::ConnectionManager;

use crate::{LogDispatcher, LogEvent};

/// Redis Stream based log dispatcher.
#[derive(Clone)]
pub struct RedisStreamDispatcher {
    /// Redis connection URL (e.g., "redis://127.0.0.1:6379")
    pub url: String,
    /// Stream key name (e.g., "rhelma:logs")
    pub stream_key: String,
}

impl RedisStreamDispatcher {
    /// Create a new Redis Stream dispatcher.
    pub fn new(url: impl Into<String>, stream_key: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            stream_key: stream_key.into(),
        }
    }
}

impl LogDispatcher for RedisStreamDispatcher {
    fn dispatch(&self, event: LogEvent) {
        let url = self.url.clone();
        let stream = self.stream_key.clone();

        if let Some(rt) = rt() {
            rt.spawn(async move {
                if let Err(e) = dispatch_to_redis(&url, &stream, &event).await {
                    // Use eprintln to avoid recursive logging
                    eprintln!(
                        "[rhelma-logger] Failed to dispatch to Redis stream '{}': {}",
                        stream, e
                    );
                }
            });
        }
    }

    fn flush(&self) {
        // Redis streams are immediately persisted, no flush needed
    }

    fn box_clone(&self) -> Box<dyn LogDispatcher + Send + Sync> {
        Box::new(self.clone())
    }
}

/// Internal helper to dispatch event to Redis stream.
async fn dispatch_to_redis(
    url: &str,
    stream: &str,
    event: &LogEvent,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = redis_crate::Client::open(url)?;
    let mut conn: ConnectionManager = client.get_connection_manager().await?;

    let data = serde_json::to_string(event)?;

    redis_crate::cmd("XADD")
        .arg(stream)
        .arg("*")
        .arg("data")
        .arg(&data)
        .query_async::<()>(&mut conn)
        .await?;

    Ok(())
}

/// Helper function to get tokio runtime handle.
fn rt() -> Option<tokio::runtime::Handle> {
    tokio::runtime::Handle::try_current().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_create_dispatcher() {
        let dispatcher = RedisStreamDispatcher::new("redis://127.0.0.1:6379", "rhelma:logs");
        assert_eq!(dispatcher.url, "redis://127.0.0.1:6379");
        assert_eq!(dispatcher.stream_key, "rhelma:logs");
    }

    #[test]
    fn can_clone_dispatcher() {
        let dispatcher = RedisStreamDispatcher::new("redis://127.0.0.1:6379", "rhelma:logs");
        let _cloned = dispatcher.box_clone();
    }
}
