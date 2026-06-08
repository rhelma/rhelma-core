#![forbid(unsafe_code)]
#![allow(
    clippy::doc_markdown,
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::struct_excessive_bools,
    clippy::needless_pass_by_value,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps,
    clippy::unused_self,
    clippy::uninlined_format_args,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::new_without_default,
    clippy::return_self_not_must_use,
    clippy::items_after_statements,
    clippy::ignored_unit_patterns,
    clippy::redundant_closure_for_method_calls,
    clippy::map_unwrap_or,
    clippy::unused_async,
    clippy::module_inception,
    clippy::field_reassign_with_default
)]

//! rhelma-observability-agent — Rhelma v5.2 Enterprise Edition
//!
//! Responsibilities:
//!   - Heartbeats
//!   - Advanced anomaly detection (AI-aware)
//!   - Local incident escalation
//!   - AI command executor + result publisher
//!   - AI decision reaction pipeline
//!   - Audit engine
//!   - Residency-aware event envelopes
//!   - Zero-trust system request context
//!   - Dynamic degraded mode controller
//!   - Kafka-based transport integration
//!
//! Core modules:

/// Agent core state and configuration management
pub mod agent;
/// AI module for incident handling and decision processing
pub mod ai;
/// AI-safe command execution and management
pub mod commands;
/// Error types and handling for the observability agent
pub mod error;
/// Input/output operations including Kafka integration
pub mod io;
/// Reflex signals and anomaly detection
pub mod reflex;
/// Runtime management and orchestration
pub mod runtime;

// Re-exports
pub use crate::agent::config::{ObservabilityAgentConfig, ResidencyMode};
pub use crate::agent::ObservabilityAgent;
pub use crate::error::AgentError;
pub use io::KafkaCommandSource;
pub use io::KafkaDecisionSource;
pub use runtime::AgentRuntime;

use uuid::Uuid;

/// Generate a Rhelma-compliant event identifier.
///
/// v5.2 recommendation: prefer UUID v7 for lexicographically sortable event IDs.
///
/// # Returns
/// UUID v7 string
pub fn generate_event_id() -> String {
    Uuid::now_v7().to_string()
}
