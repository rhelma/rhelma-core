use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Payload for reflex signals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalPayload {
    /// Signal type identifier, e.g., "error_spike", "latency_spike"
    pub kind: String,

    /// Severity level: "info" | "warning" | "critical"
    pub severity: String,

    /// Human-readable explanation of the signal
    pub message: String,

    /// Arbitrary metrics payload in JSON format
    pub metrics: Value,

    /// Optional incident correlation identifier (if already known)
    pub incident_id: Option<String>,

    /// Optional trace correlation identifier from the emitting service
    pub trace_id: Option<String>,

    /// Optional span correlation identifier from the emitting service
    pub span_id: Option<String>,
}

impl SignalPayload {
    /// Create a signal payload with the required fields.
    ///
    /// This constructor is intentionally provided to avoid brittle struct
    /// literals in downstream code and tests. When new optional fields are
    /// added to `SignalPayload`, call sites that use `SignalPayload::new(...)`
    /// do not need to change.
    pub fn new(
        kind: impl Into<String>,
        severity: impl Into<String>,
        message: impl Into<String>,
        metrics: Value,
    ) -> Self {
        Self {
            kind: kind.into(),
            severity: severity.into(),
            message: message.into(),
            metrics,
            incident_id: None,
            trace_id: None,
            span_id: None,
        }
    }

    /// Attach an incident correlation identifier.
    #[must_use]
    pub fn with_incident_id(mut self, incident_id: impl Into<String>) -> Self {
        self.incident_id = Some(incident_id.into());
        self
    }

    /// Attach a trace identifier.
    #[must_use]
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// Attach a span identifier.
    #[must_use]
    pub fn with_span_id(mut self, span_id: impl Into<String>) -> Self {
        self.span_id = Some(span_id.into());
        self
    }
}

/// Decision types for reflex signal processing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReflexDecision {
    /// No action required
    None,

    /// Generate insight event
    Insight,

    /// Generate alert event
    Alert,

    /// Escalate to AI incident processing
    EscalateToAI,
}
