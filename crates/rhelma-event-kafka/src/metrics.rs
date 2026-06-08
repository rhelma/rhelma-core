#![forbid(unsafe_code)]

use crate::reason::DlqReason;

use metrics::{counter, describe_counter, describe_histogram, histogram};

fn retryable_label(v: Option<bool>) -> &'static str {
    match v {
        Some(true) => "true",
        Some(false) => "false",
        None => "unknown",
    }
}

/// Optional: call once at process start (after metrics recorder is installed)
/// to register descriptions for dashboards.
pub fn register_descriptors() {
    describe_counter!("rhelma_event_kafka_publish_total", "Kafka publish attempts");
    describe_counter!(
        "rhelma_event_kafka_publish_error_total",
        "Kafka publish failures"
    );

    describe_counter!(
        "rhelma_event_kafka_consume_total",
        "Kafka consumed messages (payload present)"
    );
    describe_counter!(
        "rhelma_event_kafka_consume_empty_total",
        "Kafka consumed messages with no payload"
    );
    describe_counter!(
        "rhelma_event_kafka_decode_error_total",
        "JSON decode errors for incoming messages"
    );
    describe_counter!(
        "rhelma_event_kafka_idempotency_duplicate_total",
        "Duplicate events skipped by idempotency cache"
    );

    describe_counter!(
        "rhelma_event_kafka_handled_total",
        "Successfully handled events"
    );
    describe_counter!(
        "rhelma_event_kafka_handler_retry_total",
        "Handler retries attempted"
    );
    describe_counter!(
        "rhelma_event_kafka_handler_failure_total",
        "Handler failures (final)"
    );

    describe_counter!(
        "rhelma_event_kafka_dlq_record_total",
        "DLQ records created (decision to DLQ)"
    );
    describe_counter!(
        "rhelma_event_kafka_dlq_publish_total",
        "DLQ records published successfully"
    );
    describe_counter!(
        "rhelma_event_kafka_dlq_publish_error_total",
        "DLQ publish failures"
    );

    describe_histogram!(
        "rhelma_event_kafka_handler_latency_ms",
        "Handler latency per attempt (ms)"
    );
}

pub fn inc_publish() {
    counter!("rhelma_event_kafka_publish_total").increment(1);
}

pub fn inc_publish_error() {
    counter!("rhelma_event_kafka_publish_error_total").increment(1);
}

pub fn inc_consume() {
    counter!("rhelma_event_kafka_consume_total").increment(1);
}

pub fn inc_consume_empty() {
    counter!("rhelma_event_kafka_consume_empty_total").increment(1);
}

pub fn inc_decode_error() {
    counter!("rhelma_event_kafka_decode_error_total").increment(1);
}

pub fn inc_idempotency_duplicate() {
    counter!("rhelma_event_kafka_idempotency_duplicate_total").increment(1);
}

pub fn inc_handled() {
    counter!("rhelma_event_kafka_handled_total").increment(1);
}

pub fn inc_handler_retry() {
    counter!("rhelma_event_kafka_handler_retry_total").increment(1);
}

pub fn inc_handler_failure(retryable: Option<bool>) {
    counter!(
        "rhelma_event_kafka_handler_failure_total",
        "retryable" => retryable_label(retryable)
    )
    .increment(1);
}

pub fn observe_handler_latency_ms(ms: u64) {
    histogram!("rhelma_event_kafka_handler_latency_ms").record(ms as f64);
}

pub fn inc_dlq_record(reason: DlqReason, retryable: Option<bool>) {
    counter!(
        "rhelma_event_kafka_dlq_record_total",
        "reason" => reason.as_str(),
        "retryable" => retryable_label(retryable)
    )
    .increment(1);
}

pub fn inc_dlq_publish_success(reason: DlqReason) {
    counter!(
        "rhelma_event_kafka_dlq_publish_total",
        "reason" => reason.as_str()
    )
    .increment(1);
}

pub fn inc_dlq_publish_error(reason: DlqReason) {
    counter!(
        "rhelma_event_kafka_dlq_publish_error_total",
        "reason" => reason.as_str()
    )
    .increment(1);
}
