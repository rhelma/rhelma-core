#![forbid(unsafe_code)]

/// Realm-specific telemetry helpers for Rhelma.
///
/// This crate defines stable, low-cardinality metrics for realm services
/// such as `realm-hub` and `ai-companion`.
///
/// ## Cardinality policy
/// - `realm` should be low-cardinality per deployment.
/// - `channel` is clamped to a small in-process allowlist; unknown values become `other`.
/// - `kind` is clamped to a conservative set; unknown values become `other`.
///
/// Metric names are prefixed with `rhelma_` and use the `metrics` crate.
///
/// This crate contains no exporter configuration; that is handled by
/// `observability/core` in Rhelma.
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use metrics::{counter, describe_counter, describe_histogram, histogram};

const MAX_UNIQUE_CHANNELS: usize = 32;
const MAX_UNIQUE_KINDS: usize = 64;

static CHANNEL_LABELS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
static KIND_LABELS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn clamp_label(store: &OnceLock<Mutex<HashSet<String>>>, max: usize, value: &str) -> String {
    let m = store.get_or_init(|| Mutex::new(HashSet::new()));
    let mut set = m.lock().expect("label set lock poisoned");

    if set.contains(value) {
        return value.to_string();
    }
    if set.len() < max {
        set.insert(value.to_string());
        return value.to_string();
    }
    "other".to_string()
}

/// Register metric descriptors for realm telemetry.
///
/// Safe to call multiple times (descriptors are idempotent).
pub fn register_descriptors() {
    describe_counter!(
        "rhelma_realm_events_ingested_total",
        "Total number of realm events ingested by the service"
    );
    describe_counter!(
        "rhelma_realm_events_served_total",
        "Total number of realm events served (read) by the service"
    );
    describe_histogram!(
        "rhelma_realm_events_batch_size",
        "Batch size of realm events served in a single response"
    );
    describe_counter!(
        "rhelma_ai_summaries_total",
        "Total number of AI summaries produced"
    );
    describe_histogram!(
        "rhelma_ai_summary_input_events",
        "Number of input events used for an AI summary"
    );
}

/// Record an ingested event (typically on POST /events).
pub fn record_event_ingested(realm: &str, channel: Option<&str>, kind: &str) {
    let channel = channel
        .map(|c| clamp_label(&CHANNEL_LABELS, MAX_UNIQUE_CHANNELS, c))
        .unwrap_or_else(|| "none".to_string());
    let kind = clamp_label(&KIND_LABELS, MAX_UNIQUE_KINDS, kind);

    counter!(
        "rhelma_realm_events_ingested_total",
        "realm" => realm.to_string(),
        "channel" => channel,
        "kind" => kind
    )
    .increment(1);
}

/// Record served events (typically on GET /events).
pub fn record_events_served(realm: &str, channel: Option<&str>, count: usize) {
    let channel = channel
        .map(|c| clamp_label(&CHANNEL_LABELS, MAX_UNIQUE_CHANNELS, c))
        .unwrap_or_else(|| "none".to_string());

    counter!(
        "rhelma_realm_events_served_total",
        "realm" => realm.to_string(),
        "channel" => channel
    )
    .increment(1);

    histogram!("rhelma_realm_events_batch_size", "realm" => realm.to_string()).record(count as f64);
}

/// Record an AI summary attempt.
pub fn record_ai_summary(
    realm: &str,
    channel: Option<&str>,
    outcome: &'static str,
    input_events: usize,
) {
    let channel = channel
        .map(|c| clamp_label(&CHANNEL_LABELS, MAX_UNIQUE_CHANNELS, c))
        .unwrap_or_else(|| "none".to_string());

    counter!(
        "rhelma_ai_summaries_total",
        "realm" => realm.to_string(),
        "channel" => channel,
        "outcome" => outcome.to_string()
    )
    .increment(1);

    histogram!("rhelma_ai_summary_input_events", "realm" => realm.to_string())
        .record(input_events as f64);
}
