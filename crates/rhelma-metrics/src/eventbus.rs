//! Rhelma v5.1 EventBus Metrics
//!
//! Official metrics for Rhelma message fabric.

use metrics::{counter, histogram};
use smallvec::SmallVec;

#[derive(Debug, Clone, Copy)]
pub enum EventBusOutcome {
    /// Variant `Success`.
    Success,
    /// Variant `Error`.
    Error,
    /// Variant `Dropped`.
    Dropped,
}

impl EventBusOutcome {
    #[inline]
    pub fn as_static(&self) -> &'static str {
        match self {
            EventBusOutcome::Success => "success",
            EventBusOutcome::Error => "error",
            EventBusOutcome::Dropped => "dropped",
        }
    }
}

/// Zero-allocation helper (used by MetricRegistry).
pub fn record_event_publish_with_labels(
    topic: &'static str,
    outcome: EventBusOutcome,
    extra_labels: &[(&'static str, &'static str)],
) {
    debug_assert!(
        2 + extra_labels.len() <= 32,
        "too many labels; violates Rhelma cardinality rules"
    );
    let mut labels: SmallVec<[(&'static str, &'static str); 32]> = SmallVec::new();
    labels.extend_from_slice(&[("topic", topic), ("outcome", outcome.as_static())]);
    labels.extend_from_slice(extra_labels);

    counter!("rhelma_eventbus_publish_total", labels.as_slice()).increment(1);

    if matches!(outcome, EventBusOutcome::Error) {
        counter!("rhelma_eventbus_publish_error_total", labels.as_slice()).increment(1);
    }
}

/// Publish latency (seconds).
pub fn record_event_publish_duration_with_labels(
    topic: &'static str,
    outcome: EventBusOutcome,
    duration_secs: f64,
    extra_labels: &[(&'static str, &'static str)],
) {
    debug_assert!(
        2 + extra_labels.len() <= 32,
        "too many labels; violates Rhelma cardinality rules"
    );
    let mut labels: SmallVec<[(&'static str, &'static str); 32]> = SmallVec::new();
    labels.extend_from_slice(&[("topic", topic), ("outcome", outcome.as_static())]);
    labels.extend_from_slice(extra_labels);

    histogram!(
        "rhelma_eventbus_publish_duration_seconds",
        labels.as_slice()
    )
    .record(duration_secs);
}

// ---------------------------------------------------
// Consume + Handler Metrics (v5.2)
// ---------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub enum EventConsumeOutcome {
    /// Variant `Success`.
    Success,
    /// Variant `DecodeError`.
    DecodeError,
}

impl EventConsumeOutcome {
    #[inline]
    pub fn as_static(&self) -> &'static str {
        match self {
            EventConsumeOutcome::Success => "success",
            EventConsumeOutcome::DecodeError => "decode_error",
        }
    }
}

/// Counter: events consumed from transport (Kafka, etc.).
///
/// `topic_family` MUST be low-cardinality (e.g. "ops.audit", "domain", "other").
pub fn record_event_consume(topic_family: &'static str, outcome: EventConsumeOutcome) {
    counter!(
        "rhelma_eventbus_consume_total",
        "topic" => topic_family,
        "outcome" => outcome.as_static()
    )
    .increment(1);

    if matches!(outcome, EventConsumeOutcome::DecodeError) {
        counter!(
            "rhelma_eventbus_consume_decode_error_total",
            "topic" => topic_family
        )
        .increment(1);
    }
}

/// Handler duration (seconds).
pub fn record_event_handle_duration(topic_family: &'static str, duration_secs: f64) {
    histogram!(
        "rhelma_eventbus_handle_duration_seconds",
        "topic" => topic_family
    )
    .record(duration_secs);
}

// ---------------------------------------------------
// Legacy API — now NO LEAKS, but requires &'static str
// This enforces canonical event topics.
// ---------------------------------------------------

pub fn record_event_publish(topic: &'static str, outcome: EventBusOutcome) {
    record_event_publish_with_labels(topic, outcome, &[]);
}

pub fn record_event_publish_success(topic: &'static str) {
    record_event_publish(topic, EventBusOutcome::Success);
}

pub fn record_event_publish_error(topic: &'static str) {
    record_event_publish_with_labels(topic, EventBusOutcome::Error, &[]);
}

pub fn record_event_publish_duration(
    topic: &'static str,
    outcome: EventBusOutcome,
    duration_secs: f64,
) {
    record_event_publish_duration_with_labels(topic, outcome, duration_secs, &[]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eventbus_metrics_basic() {
        record_event_publish("obs.alert", EventBusOutcome::Success);
        record_event_publish_error("obs.alert");
        record_event_publish_duration("obs.alert", EventBusOutcome::Success, 0.002);
    }
}
