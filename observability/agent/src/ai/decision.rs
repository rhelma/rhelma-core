//! decision.rs — AI decision contract + apply logic (Rhelma v5.2)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use rhelma_event::{
    generate_event_id, EventBus, EventEnvelope, EventSource, EventTraceContext, Residency,
};

use crate::agent::config::ResidencyMode;
use crate::agent::context::system_request_context;
use crate::agent::state::{AiDecisionState, EffectiveSeverity, ObservabilityAgent};
use crate::error::AgentError;

/// Topic for AI incident decision events
pub const TOPIC_AI_DECISION: &str = "ai.incident.decision";
/// Topic for AI decision result events
pub const TOPIC_AI_DECISION_RESULT: &str = "ai.decision.result";

/// AI incident decision structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiIncidentDecision {
    /// Incident ID
    pub incident_id: String,
    /// Severity level (critical | warning | info | skip)
    pub severity: String,
    /// Execute flag
    pub execute: bool,

    /// Optional degraded mode override
    pub degraded_mode: Option<bool>,
    /// Optional sampling override (0-100)
    pub sampling_override: Option<u32>,
    /// Optional expiration time
    pub expires_at: Option<DateTime<Utc>>,

    /// Decision timestamp
    pub timestamp: DateTime<Utc>,
}

impl AiIncidentDecision {
    /// Converts severity string to enum
    ///
    /// # Returns
    /// Effective severity enum or None if invalid
    pub fn severity_enum(&self) -> Option<EffectiveSeverity> {
        match self.severity.as_str() {
            "critical" => Some(EffectiveSeverity::Critical),
            "warning" => Some(EffectiveSeverity::Warning),
            "info" => Some(EffectiveSeverity::Info),
            "skip" => Some(EffectiveSeverity::SkipPublish),
            _ => None,
        }
    }
}

/// AI decision result structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiDecisionResult {
    /// Incident ID
    pub incident_id: String,
    /// Applied flag
    pub applied: bool,
    /// New severity after application
    pub new_severity: String,
    /// Result timestamp
    pub timestamp: DateTime<Utc>,
}

impl AiDecisionResult {
    /// Converts to event envelope
    ///
    /// # Arguments
    /// * `service` - Service name
    /// * `service_version` - Service version
    /// * `region` - Region name
    /// * `residency` - Residency mode
    ///
    /// # Returns
    /// Event envelope
    pub fn to_envelope(
        &self,
        service: &str,
        service_version: &str,
        region: &str,
        residency: &ResidencyMode,
    ) -> EventEnvelope {
        let now = self.timestamp;

        EventEnvelope {
            // Identity
            event_id: generate_event_id(),
            event_version: 1,

            // Routing
            topic: TOPIC_AI_DECISION_RESULT.to_string(),
            key: Some(service.to_string()),

            // Timestamps
            timestamp: now,
            published_at: now,

            // Source & context
            source: EventSource {
                service: service.to_string(),
                version: service_version.to_string(),
                region: region.to_string(),
            },
            request: system_request_context(residency),

            // No trace propagation for this signal (keep it empty)
            trace: EventTraceContext {
                trace_id: None,
                span_id: None,
                tracestate: None,
                baggage: None,
                parent_span_id: None,
            },

            // Payload
            payload: serde_json::to_value(self).unwrap(),
            payload_type: "rhelma.ai.AiDecisionResult".to_string(),
            schema_ref: "ai.decision.result@v1".to_string(),

            // Policy
            policy: rhelma_event::PolicyMeta::public(rhelma_event::purpose::OBSERVABILITY_AGENT),

            // Residency & security
            residency: to_event_residency(residency),
            encryption: None,

            // Integrity
            signature: None,
            hash: None,
        }
    }
}

/// Apply AI decision to agent (Rhelma v5.2)
///
/// # Arguments
/// * `agent` - Observability agent
/// * `decision` - AI incident decision
///
/// # Returns
/// `Result<(), AgentError>` - Success or error
pub async fn apply_ai_decision<B: EventBus + Send + Sync + 'static>(
    agent: &ObservabilityAgent<B>,
    decision: AiIncidentDecision,
) -> Result<(), AgentError> {
    // 1) validate & map severity
    let override_severity: EffectiveSeverity = decision
        .severity_enum()
        .ok_or_else(|| AgentError::invalid("invalid severity in ai.incident.decision"))?;

    // 2) build canonical AI decision state
    let state = AiDecisionState {
        incident_id: decision.incident_id.clone(),
        received_at: Utc::now(),

        override_severity: Some(override_severity),
        degraded_mode: decision.degraded_mode,
        sampling_override: decision.sampling_override,
        expires_at: decision.expires_at,
    };

    // 3) apply atomically to agent
    agent.apply_ai_decision_state(state);

    // 4) produce decision result event
    let res = AiDecisionResult {
        incident_id: decision.incident_id,
        applied: true,
        new_severity: decision.severity,
        timestamp: Utc::now(),
    };

    let env = res.to_envelope(
        &agent.cfg.service_name,
        &agent.cfg.service_version,
        &agent.cfg.region,
        &agent.cfg.residency_mode,
    );

    let env = env.finalize_strict()?;
    agent.bus.publish(env).await?;

    Ok(())
}

/// Converts residency mode to event residency
///
/// # Arguments
/// * `mode` - Residency mode
///
/// # Returns
/// Event residency
fn to_event_residency(mode: &ResidencyMode) -> Residency {
    match mode {
        ResidencyMode::Global => Residency::Global,
        ResidencyMode::RegionalPreferred => Residency::RegionalOnly,
        ResidencyMode::RegionalStrict => Residency::RegionStrict,
    }
}
