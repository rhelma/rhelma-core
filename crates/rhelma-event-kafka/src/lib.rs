#![forbid(unsafe_code)]

//! rhelma-event-kafka — Kafka transport adapter for rhelma-event
//!
//! Provides:
//!   - KafkaEventBus (implements EventBus trait)
//!   - KafkaSubscriber loop (Step 23: DLQ + idempotency + optional retries)
//!   - Producer/Consumer configuration

pub mod bus;
pub mod config;
pub mod consumer;
pub mod dlq;
pub mod headers;
pub mod idempotency;
pub mod metrics;
pub mod producer;
pub mod reason;

mod topic;

pub use bus::KafkaEventBus;
pub use config::KafkaConfig;
pub use consumer::{EventHandler, FallibleEventHandler, KafkaSubscriber};
pub use dlq::{DlqPublisher, DlqRecord};
pub use idempotency::IdempotencyCache;
pub use producer::KafkaProducerWrapper;
pub use reason::DlqReason;

// Helper for consistent Contract v5.2 propagation across services.
pub use headers::{
    context_headers_map_from_envelope, context_headers_map_from_kafka_headers_and_envelope,
    extract_context_from_kafka_headers, kafka_headers_from_envelope,
    kafka_headers_from_envelope_prefer_current_otel,
};

#[cfg(feature = "otel")]
pub use headers::otel_context_from_headers_map;

// Re-export for ergonomic graceful shutdown wiring in services.
pub use tokio_util::sync::CancellationToken;
