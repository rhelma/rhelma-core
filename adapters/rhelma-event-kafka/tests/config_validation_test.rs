#![forbid(unsafe_code)]

use rhelma_event::EventBusError;
use rhelma_event_kafka::KafkaConfig;

fn is_validation(err: EventBusError) -> bool {
    matches!(err, EventBusError::Validation(_))
}

#[test]
fn producer_rejects_empty_brokers() {
    let cfg = KafkaConfig {
        brokers: "   ".into(),
        ..Default::default()
    };

    let err = cfg.validate_for_producer().unwrap_err();
    assert!(is_validation(err));
}

#[test]
fn producer_rejects_unknown_compression() {
    let cfg = KafkaConfig {
        producer_compression: "brotli".into(),
        ..Default::default()
    };

    let err = cfg.validate_for_producer().unwrap_err();
    assert!(is_validation(err));
}

#[test]
fn consumer_rejects_invalid_offset_reset() {
    let cfg = KafkaConfig {
        consumer_auto_offset_reset: "middle".into(),
        ..Default::default()
    };

    let err = cfg.validate_for_consumer().unwrap_err();
    assert!(is_validation(err));
}

#[test]
fn consumer_rejects_bad_retry_bounds() {
    let cfg = KafkaConfig {
        handler_retry_base_ms: 2_000,
        handler_retry_max_ms: 100,
        ..Default::default()
    };

    let err = cfg.validate_for_consumer().unwrap_err();
    assert!(is_validation(err));
}

#[test]
fn dlq_publisher_rejects_empty_topic() {
    let cfg = KafkaConfig {
        dlq_topic: "".into(),
        ..Default::default()
    };

    let err = cfg.validate_for_dlq_publisher().unwrap_err();
    assert!(is_validation(err));
}
