//! incident.rs — AiIncidentProposed (Rhelma v5.2)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Topic for AI incident proposed events
pub const TOPIC_AI_INCIDENT_PROPOSED: &str = "ai.incident.proposed";

/// AI incident proposed structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiIncidentProposed {
    /// Unique incident identifier
    pub incident_id: String,

    /// Service name
    pub service: String,
    /// Service version
    pub service_version: String,
    /// Environment name
    pub environment: String,
    /// Region name
    pub region: String,

    /// Time when incident was detected
    pub detected_at: DateTime<Utc>,
    /// Incident kind/type
    pub kind: String,
    /// Incident severity level
    pub severity: String,
    /// Human-readable incident message
    pub message: String,

    /// Incident metrics payload
    pub metrics: Value,

    /// Optional incident category
    pub category: Option<String>,
    /// Optional incident tags
    pub tags: Option<Vec<String>>,
    /// Optional confidence score (0.0 to 1.0)
    pub confidence: Option<f32>,
    /// Optional version number
    pub version: Option<u32>,
    /// Optional deduplication key
    pub dedupe_key: Option<String>,
    /// Optional candidate incidents data
    pub candidates: Option<Value>,

    /// Optional trace identifier for correlation
    pub trace_id: Option<String>,
    /// Optional span identifier for correlation
    pub span_id: Option<String>,
}
