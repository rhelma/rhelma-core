#![forbid(unsafe_code)]

//! Observer Evidence contracts (Stage 11B).
//!
//! Typed evidence payloads that observing agents produce and send through the
//! existing event stack ([`crate::platform::PlatformEventEnvelope`] +
//! `rhelma-event-kafka-agent` transport) to the central intelligence /
//! AI orchestrator.
//!
//! This module holds **contracts only** — no Kafka logic. It reuses the durable,
//! hashable [`PlatformEventEnvelope`](crate::platform::PlatformEventEnvelope) and
//! the existing sanitization utility
//! [`contains_obvious_secret_material`](crate::platform::contains_obvious_secret_material).
//!
//! Privacy rules baked into these types:
//! - no raw secrets, tokens, env dumps, or private keys;
//! - store **log patterns / summaries**, not full raw logs;
//! - trace ids and metric windows are allowed.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::platform::{
    contains_obvious_secret_material, PlatformEventEnvelope, PlatformEventError,
    SCHEMA_PLATFORM_EVENT_ENVELOPE_V1,
};
use crate::{EventEnvelope, EventSource, PolicyMeta, Residency};

/// Canonical event types for the evidence flow.
pub const PLATFORM_SIGNAL_DETECTED_V1: &str = "platform.signal.detected.v1";
/// Incident created event type.
pub const PLATFORM_INCIDENT_CREATED_V1: &str = "platform.incident.created.v1";
/// Incident escalated event type.
pub const PLATFORM_INCIDENT_ESCALATED_V1: &str = "platform.incident.escalated.v1";
/// Incident evidence collected event type.
pub const PLATFORM_INCIDENT_EVIDENCE_COLLECTED_V1: &str = "platform.incident.evidence_collected.v1";

/// A stable correlation id used to tie together signals, incidents, and evidence
/// bundles that belong to the same underlying situation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CorrelationId(pub String);

impl CorrelationId {
    /// Wrap an existing correlation id string.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for CorrelationId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Evidence severity. Ordered from least to most severe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Informational, no action needed.
    Info,
    /// A warning worth noting.
    Warning,
    /// Serious but not yet critical.
    Serious,
    /// Critical — requires escalation.
    Critical,
}

impl Severity {
    /// Parse from a loose string ("info"|"warning"|"serious"|"critical",
    /// plus common aliases). Unknown values fall back to `Info`.
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "critical" | "fatal" | "sev1" | "page" => Severity::Critical,
            "serious" | "error" | "high" | "sev2" => Severity::Serious,
            "warning" | "warn" | "moderate" | "sev3" => Severity::Warning,
            _ => Severity::Info,
        }
    }

    /// Whether this severity warrants opening an incident.
    pub fn is_incident_worthy(&self) -> bool {
        *self >= Severity::Serious
    }

    /// Canonical lowercase label.
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Serious => "serious",
            Severity::Critical => "critical",
        }
    }
}

/// A bounded window of a single metric — allowed in evidence (no raw payloads).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricWindow {
    /// Metric name, e.g. `http_5xx_rate`.
    pub metric: String,
    /// Window start.
    pub window_start: DateTime<Utc>,
    /// Window end.
    pub window_end: DateTime<Utc>,
    /// Number of samples in the window.
    pub sample_count: u64,
    /// Aggregate value that triggered attention (e.g. p95, mean, rate).
    pub value: f64,
    /// Optional comparison baseline (e.g. prior window mean).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline: Option<f64>,
    /// Optional threshold that was crossed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f64>,
    /// Aggregation label ("p95", "mean", "rate", ...).
    pub aggregation: String,
    /// Unit label ("ms", "ratio", "count", ...).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

/// A summarized log **pattern** — never full raw logs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogPattern {
    /// Normalized/templated pattern, e.g. `connection refused to <host>:<port>`.
    pub pattern: String,
    /// How many times the pattern occurred in the window.
    pub count: u64,
    /// Log level ("error"|"warn"|...).
    pub level: String,
    /// First seen.
    pub first_seen: DateTime<Utc>,
    /// Last seen.
    pub last_seen: DateTime<Utc>,
    /// Optional single redacted example line (already sanitized by the caller).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<String>,
}

/// A reference to a distributed trace. Ids only — allowed by the privacy rules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceReference {
    /// W3C trace id.
    pub trace_id: String,
    /// Optional span id of interest.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
    /// Optional operation/route name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
    /// Optional observed duration in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<f64>,
}

/// A point-in-time health snapshot of a service.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceHealthSnapshot {
    /// Service name.
    pub service: String,
    /// Overall status ("healthy"|"degraded"|"unhealthy"|"unknown").
    pub status: String,
    /// When it was observed.
    pub observed_at: DateTime<Utc>,
    /// Optional per-dependency status map (name → status).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<DependencyStatus>,
}

/// Status of a single dependency inside a [`ServiceHealthSnapshot`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DependencyStatus {
    /// Dependency name (e.g. "postgres", "redis").
    pub name: String,
    /// Status label.
    pub status: String,
}

/// A short reference to a recent deploy that might be implicated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecentDeployReference {
    /// Service that was deployed.
    pub service: String,
    /// Version/tag deployed.
    pub version: String,
    /// When it was deployed.
    pub deployed_at: DateTime<Utc>,
    /// Optional commit sha.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
}

/// An agent's structured (deterministic, non-LLM) diagnosis note.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentDiagnosis {
    /// Agent identifier that produced the diagnosis.
    pub agent_id: String,
    /// Short summary of the suspected cause.
    pub summary: String,
    /// Confidence in [0.0, 1.0].
    pub confidence: f32,
    /// Optional suspected-cause tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suspected_causes: Vec<String>,
}

/// A single detected signal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Signal {
    /// Signal id (agent-assigned, stable within the agent).
    pub signal_id: String,
    /// Signal kind, e.g. `error_spike`, `latency_spike`, `health_degraded`.
    pub kind: String,
    /// Severity.
    pub severity: Severity,
    /// Correlation id tying this signal to an incident/situation.
    pub correlation_id: CorrelationId,
    /// Service the signal is about.
    pub service: String,
    /// Human-readable message (already sanitized by the caller).
    pub message: String,
    /// When detected.
    pub detected_at: DateTime<Utc>,
    /// Optional metric window backing the signal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric: Option<MetricWindow>,
    /// Optional log pattern backing the signal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_pattern: Option<LogPattern>,
    /// Optional trace reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<TraceReference>,
}

/// An incident opened from one or more signals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Incident {
    /// Incident id.
    pub incident_id: String,
    /// Correlation id (shared with the originating signals).
    pub correlation_id: CorrelationId,
    /// Service the incident is about.
    pub service: String,
    /// Incident title/summary.
    pub title: String,
    /// Current severity.
    pub severity: Severity,
    /// Incident status ("open"|"escalated"|"resolved").
    pub status: String,
    /// When opened.
    pub opened_at: DateTime<Utc>,
    /// Ids of signals that contributed to this incident.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub signal_ids: Vec<String>,
}

/// A bundle of collected evidence for an incident.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceBundle {
    /// Bundle id.
    pub bundle_id: String,
    /// Incident this evidence belongs to.
    pub incident_id: String,
    /// Correlation id.
    pub correlation_id: CorrelationId,
    /// Service.
    pub service: String,
    /// When the bundle was assembled.
    pub collected_at: DateTime<Utc>,
    /// Metric windows.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metrics: Vec<MetricWindow>,
    /// Log patterns (summaries, never raw logs).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub log_patterns: Vec<LogPattern>,
    /// Trace references.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub traces: Vec<TraceReference>,
    /// Health snapshots.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub health: Vec<ServiceHealthSnapshot>,
    /// Recent deploys that might be implicated.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recent_deploys: Vec<RecentDeployReference>,
    /// Optional agent diagnosis.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnosis: Option<AgentDiagnosis>,
}

// ---- Event payloads (`platform.*.v1`) --------------------------------------

/// Payload for `platform.signal.detected.v1`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalDetectedV1 {
    /// The detected signal.
    pub signal: Signal,
}

/// Payload for `platform.incident.created.v1`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IncidentCreatedV1 {
    /// The created incident.
    pub incident: Incident,
}

/// Payload for `platform.incident.escalated.v1`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IncidentEscalatedV1 {
    /// The escalated incident (with its new severity/status).
    pub incident: Incident,
    /// Prior severity before escalation.
    pub previous_severity: Severity,
    /// Optional evidence bundle supporting the escalation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<EvidenceBundle>,
}

/// Payload for `platform.incident.evidence_collected.v1`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IncidentEvidenceCollectedV1 {
    /// The evidence bundle collected for the incident.
    pub evidence: EvidenceBundle,
}

/// Marker trait implemented by evidence payloads, binding each to its canonical
/// `platform.*.v1` event type so publishers/adapters never hard-code strings.
pub trait EvidencePayload: Serialize {
    /// Canonical event type string for this payload.
    const EVENT_TYPE: &'static str;
}

impl EvidencePayload for SignalDetectedV1 {
    const EVENT_TYPE: &'static str = PLATFORM_SIGNAL_DETECTED_V1;
}
impl EvidencePayload for IncidentCreatedV1 {
    const EVENT_TYPE: &'static str = PLATFORM_INCIDENT_CREATED_V1;
}
impl EvidencePayload for IncidentEscalatedV1 {
    const EVENT_TYPE: &'static str = PLATFORM_INCIDENT_ESCALATED_V1;
}
impl EvidencePayload for IncidentEvidenceCollectedV1 {
    const EVENT_TYPE: &'static str = PLATFORM_INCIDENT_EVIDENCE_COLLECTED_V1;
}

/// Returns true if `event_type` is one of the evidence event types.
pub fn is_evidence_event_type(event_type: &str) -> bool {
    matches!(
        event_type,
        PLATFORM_SIGNAL_DETECTED_V1
            | PLATFORM_INCIDENT_CREATED_V1
            | PLATFORM_INCIDENT_ESCALATED_V1
            | PLATFORM_INCIDENT_EVIDENCE_COLLECTED_V1
    )
}

/// Reject a payload that contains obvious secret material.
///
/// This is the guard publishers must call before wrapping a payload in a
/// platform envelope. It reuses the existing
/// [`contains_obvious_secret_material`] detector so there is one definition of
/// "secret" across the platform.
pub fn reject_if_secret<T: Serialize>(payload: &T) -> Result<(), PlatformEventError> {
    let value: Value = serde_json::to_value(payload)
        .map_err(|e| PlatformEventError::Serialization(e.to_string()))?;
    if contains_obvious_secret_material(&value) {
        return Err(PlatformEventError::Validation(
            "evidence payload contains obvious secret material".to_string(),
        ));
    }
    Ok(())
}

/// Origin/context attached to an evidence event when known.
#[derive(Debug, Clone, Default)]
pub struct EvidenceContext {
    /// Correlation id (falls back to a generated id when absent).
    pub correlation_id: Option<String>,
    /// Causation id linking to a prior event.
    pub causation_id: Option<String>,
    /// Workspace id when the evidence belongs to a workspace.
    pub workspace_id: Option<String>,
    /// Tenant id when the evidence has an isolation context.
    pub tenant_id: Option<String>,
}

/// Build a durable [`PlatformEventEnvelope`] for an evidence payload.
///
/// Sanitizes first (`reject_if_secret`), then binds the payload to its canonical
/// `platform.*.v1` event type. Contract-level only — no transport.
pub fn build_platform_event<P: EvidencePayload>(
    source: EventSource,
    payload: &P,
    ctx: &EvidenceContext,
) -> Result<PlatformEventEnvelope, PlatformEventError> {
    reject_if_secret(payload)?;
    let correlation_id = ctx
        .correlation_id
        .clone()
        .unwrap_or_else(crate::generate_event_id);
    PlatformEventEnvelope::from_payload(
        P::EVENT_TYPE,
        1,
        Utc::now(),
        source,
        ctx.workspace_id.clone(),
        ctx.tenant_id.clone(),
        None,
        correlation_id,
        ctx.causation_id.clone(),
        payload,
    )
}

/// Wrap a durable platform event into a transport [`EventEnvelope`] so it can be
/// shipped over the existing Kafka event bus.
///
/// Mirrors ai-orchestrator's `send_platform_payload` bridge: `topic` =
/// `event_type`, `schema_ref` = [`SCHEMA_PLATFORM_EVENT_ENVELOPE_V1`], and the
/// full platform envelope (hash chain included) is the transport payload.
pub fn wrap_platform_event(
    platform_event: &PlatformEventEnvelope,
    source: EventSource,
    purpose: impl Into<String>,
) -> Result<EventEnvelope, PlatformEventError> {
    let payload = serde_json::to_value(platform_event)
        .map_err(|e| PlatformEventError::Serialization(e.to_string()))?;
    let now = Utc::now();
    Ok(EventEnvelope {
        event_id: crate::generate_event_id(),
        event_version: 1,
        topic: platform_event.event_type.clone(),
        key: Some(platform_event.correlation_id.clone()),
        timestamp: now,
        published_at: now,
        source,
        request: crate::EventRequestContext::default(),
        trace: crate::EventTraceContext::default(),
        payload,
        payload_type: "application/json".to_string(),
        schema_ref: SCHEMA_PLATFORM_EVENT_ENVELOPE_V1.to_string(),
        policy: PolicyMeta::public(purpose),
        residency: Residency::Global,
        encryption: None,
        signature: None,
        hash: None,
    })
}

/// Decode a transport [`EventEnvelope`] carrying an evidence platform event back
/// into its [`PlatformEventEnvelope`], validating the hash chain.
///
/// Returns `Ok(None)` when the envelope is not an evidence event (unknown topic),
/// so central consumers can safely ignore non-evidence traffic.
pub fn decode_evidence_envelope(
    env: &EventEnvelope,
) -> Result<Option<PlatformEventEnvelope>, PlatformEventError> {
    if !is_evidence_event_type(&env.topic) {
        return Ok(None);
    }
    let platform: PlatformEventEnvelope = serde_json::from_value(env.payload.clone())
        .map_err(|e| PlatformEventError::Serialization(e.to_string()))?;
    platform.validate()?;
    if !is_evidence_event_type(&platform.event_type) {
        return Err(PlatformEventError::Validation(
            "embedded platform event_type is not an evidence type".to_string(),
        ));
    }
    Ok(Some(platform))
}
