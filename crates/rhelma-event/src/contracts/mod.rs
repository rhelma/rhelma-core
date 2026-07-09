//! Typed event contracts for Rhelma Observability + AI Self-Healing (v1).
//!
//! This module defines type-safe structs for events defined in Topic Map v1
//! and provides conversion between `EventEnvelope` and these structs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    generate_event_id, purpose, EventBusError, EventEnvelope, EventRequestContext, EventResidency,
    EventSource, EventTraceContext, PolicyMeta,
};

fn ser_err<E: std::fmt::Display>(e: E) -> EventBusError {
    EventBusError::Serialization(e.to_string())
}

fn ensure_topic(expected: &str, actual: &str) -> Result<(), EventBusError> {
    if actual != expected {
        return Err(EventBusError::Serialization(format!(
            "topic mismatch: expected='{expected}', got='{actual}'"
        )));
    }
    Ok(())
}

fn payload_from_envelope<T: for<'de> Deserialize<'de>>(
    env: &EventEnvelope,
    expected_topic: &str,
) -> Result<T, EventBusError> {
    ensure_topic(expected_topic, &env.topic)?;
    serde_json::from_value::<T>(env.payload.clone()).map_err(ser_err)
}

fn payload_to_value<T: Serialize>(v: &T) -> Result<Value, EventBusError> {
    serde_json::to_value(v).map_err(ser_err)
}

fn service_version() -> String {
    std::env::var("RHELMA_SERVICE_VERSION")
        .or_else(|_| std::env::var("SERVICE_VERSION"))
        .unwrap_or_else(|_| "unknown".to_string())
}

fn new_request_context(tenant_id: Option<String>) -> EventRequestContext {
    EventRequestContext {
        request_id: Some(Uuid::now_v7().to_string()),
        correlation_id: Some(Uuid::now_v7().to_string()),
        tenant_id,
        user_id: None,
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Observability module
// ---------------------------------------------------------------------------

/// Observability-related event contracts.
pub mod obs {
    use super::*;

    /// Canonical heartbeat payload used for `obs.heartbeat`.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct HeartbeatPayload {
        /// Service name emitting the heartbeat.
        pub service: String,
        /// Deployment region of the service.
        pub region: String,
        /// Timestamp when the heartbeat was generated.
        pub timestamp: DateTime<Utc>,
        /// Service status: "healthy" | "degraded" | "down".
        pub status: String,
    }

    impl HeartbeatPayload {
        /// Topic name for heartbeat events.
        pub const TOPIC: &'static str = "obs.heartbeat";
        const SCHEMA: &'static str = "obs.heartbeat@v1";

        /// Build EventEnvelope from heartbeat payload.
        ///
        /// # Arguments
        /// * `key` - Optional partition key. If None, uses service name.
        /// * `tenant_id` - Optional tenant identifier.
        pub fn into_envelope(
            self,
            key: Option<String>,
            tenant_id: Option<String>,
        ) -> Result<EventEnvelope, EventBusError> {
            let region = self.region.clone();
            let ver = service_version();
            let schema = Self::SCHEMA.to_string();

            Ok(EventEnvelope {
                event_id: generate_event_id(),
                event_version: 1,

                topic: Self::TOPIC.to_string(),
                key: key.or_else(|| Some(self.service.clone())),

                timestamp: self.timestamp,
                published_at: Utc::now(),

                source: EventSource {
                    service: self.service.clone(),
                    version: ver,
                    region,
                },

                request: new_request_context(tenant_id),
                trace: EventTraceContext::default(),

                payload: payload_to_value(&self)?,
                payload_type: schema.clone(),
                schema_ref: schema,

                policy: PolicyMeta::public(purpose::CONTRACTS),
                residency: EventResidency::RegionStrict,
                encryption: None,
                signature: None,
                hash: None,
            })
        }

        /// Convert EventEnvelope back to HeartbeatPayload.
        pub fn from_envelope(env: &EventEnvelope) -> Result<Self, EventBusError> {
            payload_from_envelope::<HeartbeatPayload>(env, Self::TOPIC)
        }
    }

    /// Alert event for system notifications.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AlertEvent {
        /// Service that generated the alert.
        pub service: String,
        /// Region where alert was detected.
        pub region: String,
        /// When the alert was detected.
        pub detected_at: DateTime<Utc>,
        /// Alert description.
        pub message: String,
        /// Alert severity: "info" | "warning" | "critical".
        pub severity: String,
    }

    impl AlertEvent {
        /// Topic name for alert events.
        pub const TOPIC: &'static str = "obs.alert";
        const SCHEMA: &'static str = "obs.alert@v1";

        /// Convert AlertEvent to EventEnvelope.
        pub fn into_envelope(self) -> Result<EventEnvelope, EventBusError> {
            let region = self.region.clone();
            let ver = service_version();
            let schema = Self::SCHEMA.to_string();

            Ok(EventEnvelope {
                event_id: generate_event_id(),
                event_version: 1,
                topic: Self::TOPIC.to_string(),
                key: Some(self.service.clone()),

                timestamp: self.detected_at,
                published_at: Utc::now(),

                source: EventSource {
                    service: self.service.clone(),
                    version: ver,
                    region,
                },

                request: new_request_context(None),
                trace: EventTraceContext::default(),

                payload: payload_to_value(&self)?,
                payload_type: schema.clone(),
                schema_ref: schema,
                policy: PolicyMeta::public(purpose::CONTRACTS),

                residency: EventResidency::RegionalOnly,
                encryption: None,
                signature: None,
                hash: None,
            })
        }

        /// Convert EventEnvelope back to AlertEvent.
        pub fn from_envelope(env: &EventEnvelope) -> Result<Self, EventBusError> {
            payload_from_envelope::<AlertEvent>(env, Self::TOPIC)
        }
    }

    /// Insight event for system observations and analysis.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct InsightEvent {
        /// Service generating the insight.
        pub service: String,
        /// Region of observation.
        pub region: String,
        /// When insight was generated.
        pub detected_at: DateTime<Utc>,
        /// Type/category of insight.
        pub kind: String,
        /// Insight description.
        pub message: String,
        /// Insight severity: "info" | "warning" | "critical" | ...
        pub severity: String,
    }

    impl InsightEvent {
        /// Topic name for insight events.
        pub const TOPIC: &'static str = "obs.insight";
        const SCHEMA: &'static str = "obs.insight@v1";

        /// Convert InsightEvent to EventEnvelope.
        pub fn into_envelope(self) -> Result<EventEnvelope, EventBusError> {
            let region = self.region.clone();
            let ver = service_version();
            let schema = Self::SCHEMA.to_string();

            Ok(EventEnvelope {
                event_id: generate_event_id(),
                event_version: 1,
                topic: Self::TOPIC.to_string(),
                key: Some(self.service.clone()),

                timestamp: self.detected_at,
                published_at: Utc::now(),

                source: EventSource {
                    service: self.service.clone(),
                    version: ver,
                    region,
                },

                request: new_request_context(None),
                trace: EventTraceContext::default(),

                payload: payload_to_value(&self)?,
                payload_type: schema.clone(),
                schema_ref: schema,
                policy: PolicyMeta::public(purpose::CONTRACTS),

                residency: EventResidency::RegionalOnly,
                encryption: None,
                signature: None,
                hash: None,
            })
        }

        /// Convert EventEnvelope back to InsightEvent.
        pub fn from_envelope(env: &EventEnvelope) -> Result<Self, EventBusError> {
            payload_from_envelope::<InsightEvent>(env, Self::TOPIC)
        }
    }

    /// Event for proposed incident creation.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct IncidentProposedEvent {
        /// Unique incident identifier.
        pub incident_id: String,
        /// Affected service.
        pub service: String,
        /// Region where incident occurred.
        pub region: String,
        /// Incident severity: "minor" | "moderate" | "critical".
        pub severity: String,
        /// Incident description. Accepts `message` (the observability-agent /
        /// docs-canonical field name) as an alias.
        #[serde(alias = "message")]
        pub description: String,
        /// When incident was detected.
        pub detected_at: DateTime<Utc>,
    }

    impl IncidentProposedEvent {
        /// Topic name for incident proposed events.
        ///
        /// Canonical topic is `ai.incident.proposed` — the observability-agent
        /// is the producer (see `AiIncidentProposed`) and the ai-orchestrator
        /// the consumer. Keep this in sync with the agent's emitter and the
        /// orchestrator's subscription list.
        pub const TOPIC: &'static str = "ai.incident.proposed";
        const SCHEMA: &'static str = "ai.incident.proposed@v1";

        /// Convert IncidentProposedEvent to EventEnvelope.
        pub fn into_envelope(self) -> Result<EventEnvelope, EventBusError> {
            let region = self.region.clone();
            let ver = service_version();
            let schema = Self::SCHEMA.to_string();

            Ok(EventEnvelope {
                event_id: generate_event_id(),
                event_version: 1,
                topic: Self::TOPIC.to_string(),
                key: Some(self.incident_id.clone()),

                timestamp: self.detected_at,
                published_at: Utc::now(),

                source: EventSource {
                    service: self.service.clone(),
                    version: ver,
                    region,
                },

                request: new_request_context(None),
                trace: EventTraceContext::default(),

                payload: payload_to_value(&self)?,
                payload_type: schema.clone(),
                schema_ref: schema,
                policy: PolicyMeta::public(purpose::CONTRACTS),

                residency: EventResidency::RegionStrict,
                encryption: None,
                signature: None,
                hash: None,
            })
        }

        /// Convert EventEnvelope back to IncidentProposedEvent.
        pub fn from_envelope(env: &EventEnvelope) -> Result<Self, EventBusError> {
            payload_from_envelope::<IncidentProposedEvent>(env, Self::TOPIC)
        }
    }
    /// Region health snapshot emitted by gateways/routers.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RegionHealthEvent {
        /// Publisher service (typically `api-gateway`).
        pub service: String,
        /// Region where the publisher is running.
        pub observed_region: String,
        /// Target region being checked.
        pub target_region: String,
        /// When the check was performed.
        pub checked_at: DateTime<Utc>,
        /// Whether the target region is considered healthy.
        pub is_healthy: bool,
        /// Number of healthy endpoints observed.
        pub healthy_endpoints: u8,
        /// Best observed latency (ms) among healthy endpoints; `u32::MAX` when unknown.
        pub latency_ms: u32,
        /// Health path used for checks (e.g. `/healthz`).
        #[serde(skip_serializing_if = "Option::is_none")]
        pub health_path: Option<String>,
    }

    impl RegionHealthEvent {
        /// Topic name for region health events.
        pub const TOPIC: &'static str = "obs.region_health";
        const SCHEMA: &'static str = "obs.region_health@v1";

        /// Convert RegionHealthEvent to EventEnvelope.
        pub fn into_envelope(self) -> Result<EventEnvelope, EventBusError> {
            let region = self.observed_region.clone();
            let ver = service_version();
            let schema = Self::SCHEMA.to_string();

            Ok(EventEnvelope {
                event_id: generate_event_id(),
                event_version: 1,
                topic: Self::TOPIC.to_string(),
                key: Some(self.target_region.clone()),

                timestamp: self.checked_at,
                published_at: Utc::now(),

                source: EventSource {
                    service: self.service.clone(),
                    version: ver,
                    region,
                },

                request: new_request_context(None),
                trace: EventTraceContext::default(),

                payload: payload_to_value(&self)?,
                payload_type: schema.clone(),
                schema_ref: schema,
                policy: PolicyMeta::public(purpose::API_GATEWAY),

                residency: EventResidency::RegionalOnly,
                encryption: None,
                signature: None,
                hash: None,
            })
        }

        /// Convert EventEnvelope back to RegionHealthEvent.
        pub fn from_envelope(env: &EventEnvelope) -> Result<Self, EventBusError> {
            payload_from_envelope::<RegionHealthEvent>(env, Self::TOPIC)
        }
    }

    /// Region failover event emitted by gateways/routers.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RegionFailoverEvent {
        /// Publisher service (typically `api-gateway`).
        pub service: String,
        /// Region where the publisher is running.
        pub observed_region: String,
        /// Upstream service name (e.g. `search-service`).
        pub upstream_service: String,
        /// From region id (`fallback` if unknown).
        pub from_region: String,
        /// To region id (`fallback` if unknown).
        pub to_region: String,
        /// Failover reason (`timeout`, `connect`, `5xx`, ...).
        pub reason: String,
        /// When failover was observed.
        pub occurred_at: DateTime<Utc>,
        /// Optional request id (for correlation).
        #[serde(skip_serializing_if = "Option::is_none")]
        pub request_id: Option<String>,
        /// Optional correlation id.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub correlation_id: Option<String>,
    }

    impl RegionFailoverEvent {
        /// Topic name for region failover events.
        pub const TOPIC: &'static str = "obs.region_failover";
        const SCHEMA: &'static str = "obs.region_failover@v1";

        /// Convert RegionFailoverEvent to EventEnvelope.
        pub fn into_envelope(self) -> Result<EventEnvelope, EventBusError> {
            let region = self.observed_region.clone();
            let ver = service_version();
            let schema = Self::SCHEMA.to_string();

            Ok(EventEnvelope {
                event_id: generate_event_id(),
                event_version: 1,
                topic: Self::TOPIC.to_string(),
                key: Some(self.upstream_service.clone()),

                timestamp: self.occurred_at,
                published_at: Utc::now(),

                source: EventSource {
                    service: self.service.clone(),
                    version: ver,
                    region,
                },

                request: new_request_context(None),
                trace: EventTraceContext::default(),

                payload: payload_to_value(&self)?,
                payload_type: schema.clone(),
                schema_ref: schema,
                policy: PolicyMeta::public(purpose::API_GATEWAY),

                residency: EventResidency::RegionalOnly,
                encryption: None,
                signature: None,
                hash: None,
            })
        }

        /// Convert EventEnvelope back to RegionFailoverEvent.
        pub fn from_envelope(env: &EventEnvelope) -> Result<Self, EventBusError> {
            payload_from_envelope::<RegionFailoverEvent>(env, Self::TOPIC)
        }
    }
}

// ---------------------------------------------------------------------------
// Operations module
// ---------------------------------------------------------------------------

/// Operations-related event contracts.
pub mod ops {
    use super::*;

    /// Canonical audit event for `ops.audit`.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AuditEvent {
        /// Who performed the action.
        pub actor: String,
        /// What operation was performed.
        pub operation: String,
        /// Type of resource affected.
        pub resource_type: String,
        /// ID of resource affected.
        pub resource_id: String,
        /// Operation result: "success" | "failure".
        pub result: String,
        /// Tenant/account context.
        pub tenant_id: String,
        /// Region where operation occurred.
        pub region: String,
        /// When operation occurred.
        pub timestamp: DateTime<Utc>,
    }

    impl AuditEvent {
        /// Topic name for audit events.
        pub const TOPIC: &'static str = "ops.audit";
        const SCHEMA: &'static str = "ops.audit@v1";

        /// Convert AuditEvent to EventEnvelope.
        ///
        /// Note: signature + hash are enforced at publish boundary by `EventEnvelope::finalize_strict`.
        pub fn into_envelope(self) -> Result<EventEnvelope, EventBusError> {
            let region = self.region.clone();
            let ver = service_version();
            let schema = Self::SCHEMA.to_string();

            Ok(EventEnvelope {
                event_id: generate_event_id(),
                event_version: 1,
                topic: Self::TOPIC.to_string(),
                key: Some(self.resource_id.clone()),

                timestamp: self.timestamp,
                published_at: Utc::now(),

                source: EventSource {
                    service: "ops".to_string(),
                    version: ver,
                    region,
                },

                request: new_request_context(Some(self.tenant_id.clone())),
                trace: EventTraceContext::default(),

                payload: payload_to_value(&self)?,
                payload_type: schema.clone(),
                schema_ref: schema,
                policy: PolicyMeta::public(purpose::CONTRACTS),

                residency: EventResidency::RegionStrict,
                encryption: None,
                signature: None,
                hash: None,
            })
        }

        /// Convert EventEnvelope back to AuditEvent.
        pub fn from_envelope(env: &EventEnvelope) -> Result<Self, EventBusError> {
            payload_from_envelope::<AuditEvent>(env, Self::TOPIC)
        }
    }
}

// ---------------------------------------------------------------------------
// AI module
// ---------------------------------------------------------------------------

/// AI-related event contracts.
pub mod ai {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AiCommandExecute {
        /// Field `command_id`.
        pub command_id: String,
        /// Field `incident_id`.
        pub incident_id: Option<String>,
        /// Field `service`.
        pub service: String,
        /// Field `region`.
        pub region: String,
        /// Field `action`.
        pub action: String,
        /// Field `parameters`.
        pub parameters: Value,
        /// Field `requested_at`.
        pub requested_at: DateTime<Utc>,
    }

    impl AiCommandExecute {
        pub const TOPIC: &'static str = "ai.command.execute";
        const SCHEMA: &'static str = "ai.command.execute@v1";

        pub fn into_envelope(self) -> Result<EventEnvelope, EventBusError> {
            let region = self.region.clone();
            let ver = service_version();
            let schema = Self::SCHEMA.to_string();

            Ok(EventEnvelope {
                event_id: generate_event_id(),
                event_version: 1,
                topic: Self::TOPIC.to_string(),
                key: Some(self.command_id.clone()),

                timestamp: self.requested_at,
                published_at: Utc::now(),

                source: EventSource {
                    service: self.service.clone(),
                    version: ver,
                    region,
                },

                request: new_request_context(None),
                trace: EventTraceContext::default(),

                payload: payload_to_value(&self)?,
                payload_type: schema.clone(),
                schema_ref: schema,
                policy: PolicyMeta::public(purpose::CONTRACTS),

                residency: EventResidency::RegionalOnly,
                encryption: None,
                signature: None,
                hash: None,
            })
        }

        pub fn from_envelope(env: &EventEnvelope) -> Result<Self, EventBusError> {
            payload_from_envelope::<AiCommandExecute>(env, Self::TOPIC)
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AiCommandResult {
        /// Field `command_id`.
        pub command_id: String,
        /// Field `success`.
        pub success: bool,
        /// Field `message`.
        pub message: String,
        /// Field `completed_at`.
        pub completed_at: DateTime<Utc>,
    }

    impl AiCommandResult {
        pub const TOPIC: &'static str = "ai.command.result";
        const SCHEMA: &'static str = "ai.command.result@v1";

        pub fn into_envelope(self) -> Result<EventEnvelope, EventBusError> {
            let ver = service_version();
            let schema = Self::SCHEMA.to_string();

            Ok(EventEnvelope {
                event_id: generate_event_id(),
                event_version: 1,
                topic: Self::TOPIC.to_string(),
                key: Some(self.command_id.clone()),

                timestamp: self.completed_at,
                published_at: Utc::now(),

                source: EventSource {
                    service: "ai-orchestrator".to_string(),
                    version: ver,
                    region: "unknown".to_string(),
                },

                request: new_request_context(None),
                trace: EventTraceContext::default(),

                payload: payload_to_value(&self)?,
                payload_type: schema.clone(),
                schema_ref: schema,
                policy: PolicyMeta::public(purpose::CONTRACTS),

                residency: EventResidency::RegionalOnly,
                encryption: None,
                signature: None,
                hash: None,
            })
        }

        pub fn from_envelope(env: &EventEnvelope) -> Result<Self, EventBusError> {
            payload_from_envelope::<AiCommandResult>(env, Self::TOPIC)
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AiEscalationRaised {
        /// Field `escalation_id`.
        pub escalation_id: String,
        /// Field `incident_id`.
        pub incident_id: String,
        /// Field `service`.
        pub service: String,
        /// Field `region`.
        pub region: String,
        /// Field `reason`.
        pub reason: String,
        /// Field `raised_at`.
        pub raised_at: DateTime<Utc>,
    }

    impl AiEscalationRaised {
        pub const TOPIC: &'static str = "ai.escalation.raised";
        const SCHEMA: &'static str = "ai.escalation.raised@v1";

        pub fn into_envelope(self) -> Result<EventEnvelope, EventBusError> {
            let region = self.region.clone();
            let ver = service_version();
            let schema = Self::SCHEMA.to_string();

            Ok(EventEnvelope {
                event_id: generate_event_id(),
                event_version: 1,
                topic: Self::TOPIC.to_string(),
                key: Some(self.escalation_id.clone()),

                timestamp: self.raised_at,
                published_at: Utc::now(),

                source: EventSource {
                    service: self.service.clone(),
                    version: ver,
                    region,
                },

                request: new_request_context(None),
                trace: EventTraceContext::default(),

                payload: payload_to_value(&self)?,
                payload_type: schema.clone(),
                schema_ref: schema,
                policy: PolicyMeta::public(purpose::CONTRACTS),

                residency: EventResidency::RegionalOnly,
                encryption: None,
                signature: None,
                hash: None,
            })
        }

        pub fn from_envelope(env: &EventEnvelope) -> Result<Self, EventBusError> {
            payload_from_envelope::<AiEscalationRaised>(env, Self::TOPIC)
        }
    }
}
