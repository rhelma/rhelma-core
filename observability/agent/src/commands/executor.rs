//! command.rs — AI-Safe Command Executor (Rhelma v5.2 Enterprise)
//!
//! Responsibilities:
//!   - Apply AI decisions (commands) safely
//!   - Only allow whitelisted commands (AI safety)
//!   - Publish ai.command.result (custom v2 payload)
//!   - Work with Mixed Mode (critical auto, warning suggest)

use std::sync::Arc;

use chrono::Utc;
use serde_json::Value;
use tracing::info;

use rhelma_event::{
    generate_event_id, EventBus, EventEnvelope, EventResidency, EventSource, EventTraceContext,
};

use rhelma_event::contracts::ai::AiCommandExecute;

use crate::agent::config::ResidencyMode;
use crate::agent::context::system_request_context_global;
use crate::error::AgentError;
use crate::io::internal_metrics::{
    agent_command_denied, agent_command_failure, agent_command_success,
};

/// Allowed actions (AI safety allow-list)
pub const COMMAND_ALLOWED: &[&str] = &[
    "set_log_level",
    "increase_tracing_sampling",
    "decrease_tracing_sampling",
];

/// Topic for AI command result events
pub const TOPIC_AI_COMMAND_RESULT: &str = "ai.command.result";
/// Schema reference for AI command result v2
pub const SCHEMA_AI_COMMAND_RESULT_V2: &str = "ai.command.result@v2";

/// Custom v2 command result (agent-side)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AiCommandResultV2 {
    /// Command ID
    pub command_id: String,
    /// Optional incident ID
    pub incident_id: Option<String>,

    /// Service name
    pub service: String,
    /// Region name
    pub region: String,

    /// Success flag
    pub success: bool,
    /// Result message
    pub message: String,
    /// Completion timestamp
    pub finished_at: chrono::DateTime<Utc>,
    /// Version number
    pub version: u32,

    /// Optional request ID
    pub request_id: Option<String>,
    /// Optional correlation ID
    pub correlation_id: Option<String>,
    /// Optional residency mode
    pub residency: Option<String>,
}

impl AiCommandResultV2 {
    /// Converts to event envelope
    ///
    /// # Arguments
    /// * `residency` - Residency mode
    /// * `service_version` - Service version
    ///
    /// # Returns
    /// Event envelope
    pub fn to_envelope(&self, residency: &ResidencyMode, service_version: &str) -> EventEnvelope {
        let schema = SCHEMA_AI_COMMAND_RESULT_V2.to_string();

        EventEnvelope {
            event_id: generate_event_id(),
            event_version: 1,
            topic: TOPIC_AI_COMMAND_RESULT.to_string(),
            key: Some(self.service.clone()),

            timestamp: self.finished_at,
            published_at: Utc::now(),

            source: EventSource {
                service: self.service.clone(),
                version: service_version.to_string(),
                region: self.region.clone(),
            },

            request: system_request_context_global(),

            // If you want to link to a trace, put it here (must be valid trace id format)
            trace: EventTraceContext {
                trace_id: self.correlation_id.clone(),
                span_id: None,
                tracestate: None,
                baggage: None,
                parent_span_id: None,
            },

            payload: serde_json::to_value(self).unwrap(),
            payload_type: schema.clone(),
            schema_ref: schema,

            // Policy
            policy: rhelma_event::PolicyMeta::public(rhelma_event::purpose::OBSERVABILITY_AGENT),

            residency: match residency {
                ResidencyMode::Global => EventResidency::Global,
                ResidencyMode::RegionalPreferred => EventResidency::RegionalOnly,
                ResidencyMode::RegionalStrict => EventResidency::RegionStrict,
            },

            encryption: None,
            signature: None,
            hash: None,
        }
    }
}

/// Command Executor
pub struct CommandExecutor<B: EventBus + Send + Sync + 'static> {
    /// Event bus for publishing
    bus: Arc<B>,
    /// Residency mode
    residency: ResidencyMode,
    /// Service name
    service: String,
    /// Service version
    service_version: String,
    /// Region name
    region: String,
}

impl<B> CommandExecutor<B>
where
    B: EventBus + Send + Sync + 'static,
{
    /// Backward-compatible constructor: service_version becomes "unknown"
    ///
    /// # Arguments
    /// * `bus` - Event bus instance
    /// * `residency` - Residency mode
    /// * `service` - Service name
    /// * `region` - Region name
    ///
    /// # Returns
    /// Command executor instance
    pub fn new(bus: Arc<B>, residency: ResidencyMode, service: String, region: String) -> Self {
        Self {
            bus,
            residency,
            service,
            service_version: "unknown".to_string(),
            region,
        }
    }

    /// Preferred constructor (use cfg.service_version)
    ///
    /// # Arguments
    /// * `bus` - Event bus instance
    /// * `residency` - Residency mode
    /// * `service` - Service name
    /// * `service_version` - Service version
    /// * `region` - Region name
    ///
    /// # Returns
    /// Command executor instance
    pub fn new_with_version(
        bus: Arc<B>,
        residency: ResidencyMode,
        service: String,
        service_version: String,
        region: String,
    ) -> Self {
        Self {
            bus,
            residency,
            service,
            service_version,
            region,
        }
    }

    /// Executes an AI command
    ///
    /// # Arguments
    /// * `cmd` - AI command to execute
    ///
    /// # Returns
    /// `Result<(), AgentError>` - Success or error
    pub async fn execute(&self, cmd: AiCommandExecute) -> Result<(), AgentError> {
        info!(command_id = %cmd.command_id, action = %cmd.action, "[agent] executing command");

        // safety allow-list
        if !COMMAND_ALLOWED.contains(&cmd.action.as_str()) {
            agent_command_denied();
            return self
                .send_result(&cmd, false, format!("action '{}' not allowed", cmd.action))
                .await;
        }

        let msg = match self.dispatch(&cmd.action, &cmd.parameters) {
            Ok(m) => {
                agent_command_success();
                m
            }
            Err(e) => {
                agent_command_failure();
                return self.send_result(&cmd, false, e.to_string()).await;
            }
        };

        self.send_result(&cmd, true, msg).await
    }

    /// Dispatches command to appropriate handler
    ///
    /// # Arguments
    /// * `action` - Command action
    /// * `params` - Command parameters
    ///
    /// # Returns
    /// `Result<String, AgentError>` - Result message or error
    fn dispatch(&self, action: &str, params: &Value) -> Result<String, AgentError> {
        match action {
            "set_log_level" => self.handle_set_log_level(params),
            "increase_tracing_sampling" => self.handle_sampling_change(1, params),
            "decrease_tracing_sampling" => self.handle_sampling_change(-1, params),
            _ => Err(AgentError::invalid("unknown command action")),
        }
    }

    /// Handles set_log_level command
    ///
    /// # Arguments
    /// * `params` - Command parameters
    ///
    /// # Returns
    /// `Result<String, AgentError>` - Result message or error
    fn handle_set_log_level(&self, params: &Value) -> Result<String, AgentError> {
        let Some(level) = params.get("level").and_then(|v| v.as_str()) else {
            return Err(AgentError::invalid("missing parameters.level"));
        };

        info!("[agent] set_log_level -> {}", level);
        Ok(format!("log level changed to {}", level))
    }

    /// Handles sampling change command
    ///
    /// # Arguments
    /// * `delta` - Sampling delta (+1 or -1)
    /// * `_params` - Command parameters (unused)
    ///
    /// # Returns
    /// `Result<String, AgentError>` - Result message or error
    fn handle_sampling_change(&self, delta: i32, _params: &Value) -> Result<String, AgentError> {
        info!(
            "[agent] tracing sampling change delta={} service={}",
            delta, self.service
        );
        Ok(format!("sampling adjusted by {}", delta))
    }

    /// Sends command result
    ///
    /// # Arguments
    /// * `cmd` - Original command
    /// * `success` - Success flag
    /// * `message` - Result message
    ///
    /// # Returns
    /// `Result<(), AgentError>` - Success or error
    async fn send_result(
        &self,
        cmd: &AiCommandExecute,
        success: bool,
        message: String,
    ) -> Result<(), AgentError> {
        let result = AiCommandResultV2 {
            command_id: cmd.command_id.clone(),
            incident_id: cmd.incident_id.clone(),

            service: self.service.clone(),
            region: self.region.clone(),

            success,
            message,
            finished_at: Utc::now(),
            version: 2,

            request_id: None,
            correlation_id: None,
            residency: Some(self.residency.as_str().into()),
        };

        let env = result.to_envelope(&self.residency, &self.service_version);
        let env = env.finalize_strict()?;
        self.bus.publish(env).await?;

        Ok(())
    }
}
