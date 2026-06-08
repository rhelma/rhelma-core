//! eventbus_metrics.rs — EventBus metrics (enterprise observability)

use std::sync::atomic::{AtomicU64, Ordering};

/// Outcome of event bus publishing operations
#[derive(Debug, Clone)]
pub enum EventBusOutcome {
    /// Successful publishing
    Success,
    /// Failed publishing
    Error,
}

/// Counter for successful publish operations
pub static PUBLISH_SUCCESS: AtomicU64 = AtomicU64::new(0);
/// Counter for failed publish operations
pub static PUBLISH_ERROR: AtomicU64 = AtomicU64::new(0);

/// Total duration of publish operations in microseconds
pub static PUBLISH_DURATION_TOTAL: AtomicU64 = AtomicU64::new(0);

/// Records an event publish outcome
///
/// # Arguments
/// * `_topic` - Event topic (currently unused, reserved for future use)
/// * `outcome` - Publish outcome (success or error)
pub fn record_event_publish(_topic: &str, outcome: EventBusOutcome) {
    match outcome {
        EventBusOutcome::Success => {
            PUBLISH_SUCCESS.fetch_add(1, Ordering::Relaxed);
        }
        EventBusOutcome::Error => {
            PUBLISH_ERROR.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Records event publish duration
///
/// # Arguments
/// * `_topic` - Event topic (currently unused, reserved for future use)
/// * `_outcome` - Publish outcome (currently unused, reserved for future use)
/// * `duration_secs` - Duration in seconds
pub fn record_event_publish_duration(_topic: &str, _outcome: EventBusOutcome, duration_secs: f64) {
    let micros = (duration_secs * 1_000_000.0) as u64;
    PUBLISH_DURATION_TOTAL.fetch_add(micros, Ordering::Relaxed);
}
