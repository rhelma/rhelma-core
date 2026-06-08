//! audit.rs — Rhelma v5.2 aligned audit publisher

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use rhelma_event::{
    generate_event_id, EventBus, EventEnvelope, EventSource, EventTraceContext, Residency,
};

use rhelma_tracing::context;

use crate::agent::config::ResidencyMode;
use crate::agent::context::system_request_context_global;
use crate::error::AgentError;
use crate::io::internal_metrics;

/// Topic for audit events
pub const TOPIC_OPS_AUDIT: &str = "ops.audit";
/// Topic for audit failure events
pub const TOPIC_OPS_AUDIT_FAILURE: &str = "ops.audit.failure";

/// Audit record structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    /// Service name
    pub service: String,
    /// Service version
    pub service_version: String,
    /// Environment name
    pub environment: String,
    /// Region name
    pub region: String,

    /// Action performed
    pub action: String,
    /// Outcome of the action
    pub outcome: String,

    /// Optional message
    pub message: Option<String>,
    /// Optional actor
    pub actor: Option<String>,
    /// Optional tenant identifier
    pub tenant_id: Option<String>,
    /// Optional incident identifier
    pub incident_id: Option<String>,
    /// Optional correlation identifier
    pub correlation_id: Option<String>,

    /// Timestamp of the audit event
    pub timestamp: DateTime<Utc>,
    /// Optional details
    pub details: Option<Value>,

    /// Optional hash
    pub hash: Option<String>,
    /// Optional chain hash
    pub chain_hash: Option<String>,
    /// Optional signature
    pub signature: Option<String>,
}

/// Audit failure record structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditFailureRecord {
    /// Service name
    pub service: String,
    /// Service version
    pub service_version: String,
    /// Environment name
    pub environment: String,
    /// Region name
    pub region: String,

    /// Action attempted
    pub action: String,
    /// Error kind
    pub error_kind: String,
    /// Error message
    pub error_message: String,

    /// Timestamp of the audit failure
    pub timestamp: DateTime<Utc>,
    /// Optional details
    pub details: Option<Value>,

    /// Optional hash
    pub hash: Option<String>,
    /// Optional chain hash
    pub chain_hash: Option<String>,
    /// Optional signature
    pub signature: Option<String>,
}

/// Audit publisher for emitting audit events
pub struct AuditPublisher<B: EventBus + Send + Sync + 'static> {
    /// Service name
    service: String,
    /// Service version
    version: String,
    /// Environment name
    environment: String,
    /// Region name
    region: String,
    /// Residency mode
    residency: ResidencyMode,
    /// Event bus for publishing
    bus: Arc<B>,
}

impl<B> AuditPublisher<B>
where
    B: EventBus + Send + Sync + 'static,
{
    /// Creates a new audit publisher
    ///
    /// # Arguments
    /// * `service` - Service name
    /// * `version` - Service version
    /// * `env` - Environment name
    /// * `region` - Region name
    /// * `residency` - Residency mode
    /// * `bus` - Event bus instance
    ///
    /// # Returns
    /// A new audit publisher instance
    pub fn new(
        service: String,
        version: String,
        env: String,
        region: String,
        residency: ResidencyMode,
        bus: Arc<B>,
    ) -> Self {
        Self {
            service,
            version,
            environment: env,
            region,
            residency,
            bus,
        }
    }

    /// Publishes an audit event
    ///
    /// # Arguments
    /// * `action` - Action performed
    /// * `outcome` - Outcome of the action
    /// * `message` - Optional message
    /// * `actor` - Optional actor
    /// * `tenant_id` - Optional tenant identifier
    /// * `details` - Optional details
    /// * `incident_id` - Optional incident identifier
    ///
    /// # Returns
    /// `Result<(), AgentError>` - Success or error
    pub async fn audit(
        &self,
        action: &str,
        outcome: &str,
        message: Option<String>,
        actor: Option<String>,
        tenant_id: Option<String>,
        details: Option<Value>,
        incident_id: Option<String>,
    ) -> Result<(), AgentError> {
        internal_metrics::audit_ok();

        let rec = AuditRecord {
            service: self.service.clone(),
            service_version: self.version.clone(),
            environment: self.environment.clone(),
            region: self.region.clone(),

            action: action.into(),
            outcome: outcome.into(),
            message,
            actor,
            tenant_id,
            incident_id,
            correlation_id: context::current_trace_id(),

            timestamp: Utc::now(),
            details,
            hash: None,
            chain_hash: None,
            signature: None,
        };

        let env = self.build_env(
            TOPIC_OPS_AUDIT,
            serde_json::to_value(rec)?,
            "rhelma.ops.Audit",
        )?;
        let env = env.finalize_strict()?;
        self.bus.publish(env).await?;
        Ok(())
    }

    /// Publishes an audit failure event
    ///
    /// # Arguments
    /// * `action` - Action attempted
    /// * `error_kind` - Kind of error
    /// * `err_msg` - Error message
    /// * `details` - Optional details
    ///
    /// # Returns
    /// `Result<(), AgentError>` - Success or error
    pub async fn audit_failure(
        &self,
        action: &str,
        error_kind: &str,
        err_msg: String,
        details: Option<Value>,
    ) -> Result<(), AgentError> {
        internal_metrics::audit_failed();

        let rec = AuditFailureRecord {
            service: self.service.clone(),
            service_version: self.version.clone(),
            environment: self.environment.clone(),
            region: self.region.clone(),

            action: action.into(),
            error_kind: error_kind.into(),
            error_message: err_msg,

            timestamp: Utc::now(),
            details,
            hash: None,
            chain_hash: None,
            signature: None,
        };

        let env = self.build_env(
            TOPIC_OPS_AUDIT_FAILURE,
            serde_json::to_value(rec)?,
            "rhelma.ops.AuditFailure",
        )?;
        let env = env.finalize_strict()?;
        self.bus.publish(env).await?;
        Ok(())
    }

    /// Builds an event envelope
    ///
    /// # Arguments
    /// * `topic` - Event topic
    /// * `payload` - Event payload
    /// * `payload_type` - Payload type identifier
    ///
    /// # Returns
    /// `Result<EventEnvelope, AgentError>` - Event envelope or error
    fn build_env(
        &self,
        topic: &str,
        payload: Value,
        payload_type: &str,
    ) -> Result<EventEnvelope, AgentError> {
        let now = Utc::now();

        Ok(EventEnvelope {
            // Identity
            event_id: generate_event_id(),
            event_version: 1,

            // Routing
            topic: topic.to_string(),
            key: Some(self.service.clone()),

            // Timestamps
            timestamp: now,
            published_at: now,

            // Source & context
            source: EventSource {
                service: self.service.clone(),
                version: self.version.clone(),
                region: self.region.clone(),
            },
            request: system_request_context_global(),
            trace: EventTraceContext {
                trace_id: context::current_trace_id(),
                span_id: context::current_span_id(),
                tracestate: context::current_tracestate(),
                baggage: context::current_baggage(),
                parent_span_id: None,
            },

            // Payload
            payload,
            payload_type: payload_type.to_string(),
            schema_ref: format!("{topic}@v1"),

            // Policy
            policy: rhelma_event::PolicyMeta::public(rhelma_event::purpose::OBSERVABILITY_AGENT),

            // Residency & security
            residency: to_event_residency(&self.residency),
            encryption: None,

            // Integrity
            signature: None,
            hash: None,
        })
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test for residency mode conversion
    #[test]
    fn test_to_event_residency() {
        assert_eq!(
            to_event_residency(&ResidencyMode::Global),
            Residency::Global
        );
        assert_eq!(
            to_event_residency(&ResidencyMode::RegionalPreferred),
            Residency::RegionalOnly
        );
        assert_eq!(
            to_event_residency(&ResidencyMode::RegionalStrict),
            Residency::RegionStrict
        );
    }
}
