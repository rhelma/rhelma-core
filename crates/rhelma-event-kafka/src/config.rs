#![forbid(unsafe_code)]

use rhelma_event::EventBusError;
use serde::{Deserialize, Serialize};

/// Kafka transport configuration for rhelma-event.
///
/// Contract v5.2 add-ons (Step 23):
/// - DLQ / quarantine topic
/// - handler retry policy (optional)
/// - idempotency cache for duplicate deliveries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConfig {
    /// Field `brokers`.
    pub brokers: String,
    /// Field `topic_prefix`.
    pub topic_prefix: String,
    /// Field `group_id`.
    pub group_id: String,

    /// Field `producer_linger_ms`.
    pub producer_linger_ms: u64,
    /// Field `producer_batch_size`.
    pub producer_batch_size: usize,
    /// Field `producer_compression`.
    pub producer_compression: String,

    /// Field `consumer_auto_offset_reset`.
    pub consumer_auto_offset_reset: String,

    /// Poll timeout for consumer loop.
    pub consumer_poll_timeout_ms: u64,

    /// Enable DLQ publishing for poison messages / handler failures.
    pub dlq_enabled: bool,
    /// DLQ topic name (will be prefixed by `topic_prefix` unless already prefixed).
    pub dlq_topic: String,
    /// Max bytes of original payload to include in DLQ record.
    pub dlq_max_payload_bytes: usize,

    /// Enable duplicate-suppression using event_id as idempotency key.
    pub idempotency_enabled: bool,
    /// TTL (seconds) for in-memory idempotency cache.
    pub idempotency_ttl_secs: u64,
    /// Soft cap for in-memory idempotency cache (oldest entries are evicted).
    pub idempotency_max_entries: usize,

    /// Handler retry attempts (for fallible handlers).
    pub handler_retry_max_attempts: u32,
    /// Base backoff in ms.
    pub handler_retry_base_ms: u64,
    /// Max backoff in ms.
    pub handler_retry_max_ms: u64,
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            brokers: "localhost:9092".into(),
            topic_prefix: "rhelma.".into(),
            group_id: "rhelma-event-default-group".into(),
            producer_linger_ms: 5,
            producer_batch_size: 32_768,
            producer_compression: "lz4".into(),
            consumer_auto_offset_reset: "latest".into(),

            consumer_poll_timeout_ms: 250,

            dlq_enabled: true,
            dlq_topic: "dlq".into(),
            dlq_max_payload_bytes: 1_000_000,

            idempotency_enabled: true,
            idempotency_ttl_secs: 60 * 60,
            idempotency_max_entries: 200_000,

            handler_retry_max_attempts: 3,
            handler_retry_base_ms: 75,
            handler_retry_max_ms: 2_000,
        }
    }
}

impl KafkaConfig {
    fn validate_common(&self) -> Result<(), EventBusError> {
        if self.brokers.trim().is_empty() {
            return Err(EventBusError::Validation(
                "kafka brokers must not be empty".into(),
            ));
        }
        if self.topic_prefix.contains('*') || self.topic_prefix.contains('^') {
            return Err(EventBusError::Validation(
                "topic_prefix must not contain wildcard/regex patterns".into(),
            ));
        }
        Ok(())
    }

    /// Validate settings required for producing.
    pub fn validate_for_producer(&self) -> Result<(), EventBusError> {
        self.validate_common()?;

        // Producer tuning
        if self.producer_linger_ms > 60_000 {
            return Err(EventBusError::Validation(
                "producer_linger_ms must be <= 60000".into(),
            ));
        }
        if self.producer_batch_size == 0 {
            return Err(EventBusError::Validation(
                "producer_batch_size must be > 0".into(),
            ));
        }
        // librdkafka supports: none, gzip, snappy, lz4, zstd.
        match self.producer_compression.as_str() {
            "none" | "gzip" | "snappy" | "lz4" | "zstd" => {}
            other => {
                return Err(EventBusError::Validation(format!(
                    "producer_compression must be one of: none,gzip,snappy,lz4,zstd (got '{other}')"
                )));
            }
        }
        Ok(())
    }

    /// Validate settings required for consuming.
    pub fn validate_for_consumer(&self) -> Result<(), EventBusError> {
        self.validate_common()?;

        if self.group_id.trim().is_empty() {
            return Err(EventBusError::Validation(
                "group_id must not be empty".into(),
            ));
        }
        match self.consumer_auto_offset_reset.as_str() {
            "earliest" | "latest" | "error" => {}
            other => {
                return Err(EventBusError::Validation(format!(
                    "consumer_auto_offset_reset must be one of: earliest,latest,error (got '{other}')"
                )));
            }
        }
        if self.consumer_poll_timeout_ms == 0 || self.consumer_poll_timeout_ms > 60_000 {
            return Err(EventBusError::Validation(
                "consumer_poll_timeout_ms must be in 1..=60000".into(),
            ));
        }

        // DLQ settings
        if self.dlq_enabled {
            if self.dlq_topic.trim().is_empty() {
                return Err(EventBusError::Validation(
                    "dlq_topic must not be empty".into(),
                ));
            }
            if self.dlq_max_payload_bytes == 0 {
                return Err(EventBusError::Validation(
                    "dlq_max_payload_bytes must be > 0".into(),
                ));
            }
            // Guardrail: keep it within a reasonable bound to avoid excessive memory spikes.
            if self.dlq_max_payload_bytes > 10_000_000 {
                return Err(EventBusError::Validation(
                    "dlq_max_payload_bytes must be <= 10_000_000".into(),
                ));
            }
        }

        // Idempotency settings
        if self.idempotency_enabled {
            if self.idempotency_ttl_secs == 0 {
                return Err(EventBusError::Validation(
                    "idempotency_ttl_secs must be > 0".into(),
                ));
            }
            if self.idempotency_max_entries == 0 {
                return Err(EventBusError::Validation(
                    "idempotency_max_entries must be > 0".into(),
                ));
            }
            if self.idempotency_max_entries > 2_000_000 {
                return Err(EventBusError::Validation(
                    "idempotency_max_entries must be <= 2_000_000".into(),
                ));
            }
        }

        // Retry policy settings (used by fallible handlers).
        if self.handler_retry_max_attempts == 0 {
            return Err(EventBusError::Validation(
                "handler_retry_max_attempts must be >= 1 (use 1 to disable retries)".into(),
            ));
        }
        if self.handler_retry_max_attempts > 100 {
            return Err(EventBusError::Validation(
                "handler_retry_max_attempts must be <= 100".into(),
            ));
        }
        if self.handler_retry_base_ms == 0 {
            return Err(EventBusError::Validation(
                "handler_retry_base_ms must be > 0".into(),
            ));
        }
        if self.handler_retry_max_ms == 0 {
            return Err(EventBusError::Validation(
                "handler_retry_max_ms must be > 0".into(),
            ));
        }
        if self.handler_retry_base_ms > self.handler_retry_max_ms {
            return Err(EventBusError::Validation(
                "handler_retry_base_ms must be <= handler_retry_max_ms".into(),
            ));
        }

        Ok(())
    }

    /// Validate settings required for DLQ publishing.
    pub fn validate_for_dlq_publisher(&self) -> Result<(), EventBusError> {
        self.validate_common()?;

        if self.dlq_topic.trim().is_empty() {
            return Err(EventBusError::Validation(
                "dlq_topic must not be empty".into(),
            ));
        }
        if self.dlq_max_payload_bytes == 0 {
            return Err(EventBusError::Validation(
                "dlq_max_payload_bytes must be > 0".into(),
            ));
        }
        if self.dlq_max_payload_bytes > 10_000_000 {
            return Err(EventBusError::Validation(
                "dlq_max_payload_bytes must be <= 10_000_000".into(),
            ));
        }
        Ok(())
    }
}
