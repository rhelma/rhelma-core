//! ObservabilityAgentConfig — Rhelma v5.2 aligned
//!
//! Responsibilities:
//!   - Provide all metadata required by Rhelma Contract (service/env/region/version)
//!   - Provide residency configuration for all system events
//!   - Control runtime behavior (command/decision pipelines, degraded mode, sampling reduction)
//!   - Provide optional Kafka hints (bootstrap, topics, group_id)
//!
//! Priority order for configuration (highest → lowest):
//!   1. Explicit RHELMA_* environment variables
//!   2. CentralEnv (global runtime environment)
//!   3. Built-in defaults

use serde::{Deserialize, Serialize};
use std::time::Duration;

use rhelma_config::CentralEnv;

use crate::error::AgentError;

//
// ─────────────────────────────────────────────────────────────
//   ResidencyMode (Rhelma v5.2)
// ─────────────────────────────────────────────────────────────
//

/// Residency determines where system-level events belong and are processed.
/// This affects routing, locality guarantees, and edge-cloud behavior.
///
/// GLOBAL              = The event is globally relevant (default)
/// REGIONAL_PREFERRED = Prefer to publish within region, fallback global
/// REGIONAL_STRICT    = Must remain inside region; never forwarded globally
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResidencyMode {
    /// Global residency - event is globally relevant
    Global,
    /// Regional preferred - prefer to publish within region, fallback global
    RegionalPreferred,
    /// Regional strict - must remain inside region; never forwarded globally
    RegionalStrict,
}

impl Default for ResidencyMode {
    /// Default implementation returns Global residency
    fn default() -> Self {
        ResidencyMode::Global
    }
}

impl ResidencyMode {
    /// Try to load residency from environment
    ///
    /// # Returns
    /// Residency mode from environment variable or None if not set/invalid
    pub fn from_env() -> Option<Self> {
        match std::env::var("RHELMA_RESIDENCY")
            .ok()?
            .to_lowercase()
            .as_str()
        {
            "global" => Some(Self::Global),
            "regional_preferred" => Some(Self::RegionalPreferred),
            "regional_strict" => Some(Self::RegionalStrict),
            _ => None,
        }
    }

    /// Returns residency mode as a static string
    ///
    /// # Returns
    /// String representation of the residency mode
    pub fn as_str(&self) -> &'static str {
        match self {
            ResidencyMode::Global => "GLOBAL",
            ResidencyMode::RegionalPreferred => "REGIONAL_PREFERRED",
            ResidencyMode::RegionalStrict => "REGIONAL_STRICT",
        }
    }
}

//
// ─────────────────────────────────────────────────────────────
//   ObservabilityAgentConfig
// ─────────────────────────────────────────────────────────────
//

/// Configuration for the Observability Agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityAgentConfig {
    //
    // ────────────────────────────────────────────────
    //   Required identity (Rhelma contract)
    // ────────────────────────────────────────────────
    //
    /// Service name
    pub service_name: String,
    /// Environment name
    pub environment: String,
    /// Region name
    pub region: String,
    /// Service version
    pub service_version: String,

    //
    // ────────────────────────────────────────────────
    //   Runtime knobs
    // ────────────────────────────────────────────────
    //
    /// Heartbeat interval in seconds
    pub heartbeat_interval_secs: u64,
    /// Stale threshold in seconds
    pub stale_threshold_secs: u64,
    /// Anomaly window size
    pub anomaly_window_size: u32,

    //
    // ────────────────────────────────────────────────
    //   Rhelma v5.2 additions
    // ────────────────────────────────────────────────
    //
    /// Residency mode
    pub residency_mode: ResidencyMode,

    /// If true, agent begins in degraded mode (AI-aware logic escalates warnings)
    pub degraded_mode_initial: bool,

    /// If true, agent begins with reduced sampling (skip info-type events)
    pub sampling_reduction_initial: bool,

    /// Enable AI-safe command execution pipeline
    pub command_enabled: bool,

    /// Enable AI decision listener pipeline
    pub decision_enabled: bool,

    //
    // ────────────────────────────────────────────────
    //   Optional Kafka config (transport hint)
    // ────────────────────────────────────────────────
    //
    /// Kafka bootstrap servers
    pub kafka_bootstrap: Option<String>,
    /// Kafka command topic
    pub kafka_command_topic: Option<String>,
    /// Kafka decision topic
    pub kafka_decision_topic: Option<String>,
    /// Kafka group ID
    pub kafka_group_id: Option<String>,
}

impl ObservabilityAgentConfig {
    //
    // ────────────────────────────────────────────────
    //   Load from environment + CentralEnv
    // ────────────────────────────────────────────────
    //
    /// Creates configuration from CentralEnv and environment variables
    ///
    /// # Arguments
    /// * `central` - Central environment configuration
    ///
    /// # Returns
    /// `Result<Self, AgentError>` - Configuration or error
    pub fn from_central(central: &CentralEnv) -> Result<Self, AgentError> {
        Ok(Self {
            //
            // Identity
            //
            service_name: std::env::var("RHELMA_SERVICE_NAME")
                .map_err(|_| AgentError::MissingField("RHELMA_SERVICE_NAME".into()))?,

            environment: central.environment.clone(),

            region: central.region.clone(),

            service_version: std::env::var("RHELMA_SERVICE_VERSION")
                .unwrap_or_else(|_| central.service_version.clone()),

            //
            // Runtime knobs
            //
            heartbeat_interval_secs: std::env::var("RHELMA_AGENT_HEARTBEAT_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(15),

            stale_threshold_secs: std::env::var("RHELMA_AGENT_STALE_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(120),

            anomaly_window_size: std::env::var("RHELMA_AGENT_ANOMALY_WINDOW")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(20),

            //
            // ResidencyMode (v5.2)
            //
            residency_mode: ResidencyMode::from_env().unwrap_or_default(),

            //
            // Agent initial modes
            //
            degraded_mode_initial: std::env::var("RHELMA_AGENT_DEGRADED")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),

            sampling_reduction_initial: std::env::var("RHELMA_AGENT_SAMPLING_REDUCTION")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),

            //
            // Pipeline toggles
            //
            command_enabled: std::env::var("RHELMA_AGENT_ENABLE_COMMAND")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(true),

            decision_enabled: std::env::var("RHELMA_AGENT_ENABLE_DECISION")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(true),

            //
            // Kafka hints
            //
            kafka_bootstrap: std::env::var("KAFKA_BOOTSTRAP_SERVERS").ok(),
            kafka_command_topic: std::env::var("OBS_AGENT_COMMAND_TOPIC").ok(),
            kafka_decision_topic: std::env::var("OBS_AGENT_DECISION_TOPIC").ok(),
            kafka_group_id: std::env::var("OBS_AGENT_GROUP_ID").ok(),
        })
    }

    //
    // ────────────────────────────────────────────────
    //   Validation (Rhelma contract compliance)
    // ────────────────────────────────────────────────
    //
    /// Validates the configuration
    ///
    /// # Returns
    /// `Result<(), AgentError>` - Success or validation error
    pub fn validate(&self) -> Result<(), AgentError> {
        if self.service_name.trim().is_empty() {
            return Err(AgentError::MissingField("service_name".into()));
        }
        if self.environment.trim().is_empty() {
            return Err(AgentError::MissingField("environment".into()));
        }
        if self.region.trim().is_empty() {
            return Err(AgentError::MissingField("region".into()));
        }

        match self.environment.as_str() {
            "development" | "staging" | "production" => {}
            other => {
                return Err(AgentError::invalid(format!(
                    "invalid environment '{}'; allowed: development|staging|production",
                    other
                )));
            }
        }

        if self.heartbeat_interval_secs < 5 {
            return Err(AgentError::invalid("heartbeat_interval_secs must be >= 5"));
        }

        if self.anomaly_window_size == 0 {
            return Err(AgentError::invalid("anomaly_window_size must be >= 1"));
        }

        Ok(())
    }

    //
    // ────────────────────────────────────────────────
    //   Accessors
    // ────────────────────────────────────────────────
    //
    /// Gets heartbeat interval as Duration
    ///
    /// # Returns
    /// Heartbeat interval duration
    pub fn heartbeat_interval(&self) -> Duration {
        Duration::from_secs(self.heartbeat_interval_secs)
    }

    /// Gets stale threshold as Duration
    ///
    /// # Returns
    /// Stale threshold duration
    pub fn stale_threshold(&self) -> Duration {
        Duration::from_secs(self.stale_threshold_secs)
    }
}
