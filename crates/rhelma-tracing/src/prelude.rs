//! rhelma-tracing prelude
//!
//! This prelude exposes the common types, macros, and helpers
//! needed by Rhelma platform services for distributed tracing.
//!
//! Import once per service/module:
//!     use rhelma_tracing::prelude::*;

pub use crate::business::{business_span, BusinessSpanExt};
pub use crate::config::TracingConfig;
pub use crate::RhelmaTracing;

// Macros are exported at crate root, so re-export directly.
pub use crate::instrument_span;

// If later we add instrument_ctx! it will be exported here automatically:
// pub use crate::instrument_ctx;

// Re-export frequently used tracing API items.
pub use tracing::{debug, error, info, span, trace, warn, Instrument, Level, Span};

// Useful subscriber extensions (with/builder).
pub use tracing_subscriber::layer::SubscriberExt;

// If OTEL is enabled, expose the SpanExt and OpenTelemetrySpanExt.
#[cfg(feature = "otel")]
pub use tracing_opentelemetry::{OpenTelemetryLayer, OpenTelemetrySpanExt};

// Kafka OTEL propagation helpers (requires feature `kafka`).
#[cfg(feature = "kafka")]
pub use crate::kafka_propagation::{
    extract_trace_context as kafka_extract_trace_context,
    inject_trace_context as kafka_inject_trace_context, KafkaHeaderExtractor, KafkaHeaderInjector,
};
