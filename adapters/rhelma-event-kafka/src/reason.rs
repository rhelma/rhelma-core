#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Stable DLQ reason taxonomy.
///
/// These values are meant to be machine-friendly, low-cardinality labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DlqReason {
    /// The message payload could not be decoded as an EventEnvelope.
    DecodeError,
    /// The handler failed (may be retryable depending on error class).
    HandlerError,
    /// Event envelope violated the contract (missing required fields, etc.).
    ContractViolation,
    /// Topic name violates platform policy (e.g. wildcard/regex disallowed).
    TopicPolicyViolation,
    /// Kafka/transport error.
    TransportError,
    /// Serialization error (JSON encoding/decoding other than decode_error).
    SerializationError,
    /// Any other reason.
    Unknown,
}

impl DlqReason {
    pub fn as_str(self) -> &'static str {
        match self {
            DlqReason::DecodeError => "decode_error",
            DlqReason::HandlerError => "handler_error",
            DlqReason::ContractViolation => "contract_violation",
            DlqReason::TopicPolicyViolation => "topic_policy_violation",
            DlqReason::TransportError => "transport_error",
            DlqReason::SerializationError => "serialization_error",
            DlqReason::Unknown => "unknown",
        }
    }
}
