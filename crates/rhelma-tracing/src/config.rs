use std::env;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

use rhelma_config::UnifiedObservabilityConfig;

/// Tracing configuration for Rhelma v5.1+.
///
/// NOTE:
/// - OTEL knobs and sampling come from `rhelma-config::UnifiedObservabilityConfig`.
/// - `environment` in rhelma-config is an enum → we store it as a lowercased String here
///   for compatibility with existing tracing-subscriber filters/logging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Field `service_name`.
    pub service_name: String,
    /// Field `service_version`.
    pub service_version: String,
    /// Field `environment`.
    pub environment: String,
    /// Field `region`.
    pub region: String,

    /// Default subscriber level (mapped to EnvFilter / tracing-subscriber).
    pub level: String,

    /// Local head-based sampling probability in [0.0, 1.0].
    pub sampling_rate: f64,

    /// OTEL settings (derived from UnifiedObservabilityConfig).
    pub otel_enabled: bool,
    /// Field `otel_endpoint`.
    pub otel_endpoint: Option<String>,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            service_name: "unknown".into(),
            service_version: "0.0.0".into(),
            environment: "development".into(),
            region: "local".into(),
            level: "info".into(),
            sampling_rate: 1.0,
            otel_enabled: false,
            otel_endpoint: None,
        }
    }
}

impl TracingConfig {
    /// Build from UnifiedObservabilityConfig (observability-core wiring).
    pub fn from_unified(obs: &UnifiedObservabilityConfig) -> Self {
        let sampling_rate = obs.sampling_rate.clamp(0.0, 1.0);
        let otel_enabled = obs.otel_enabled && obs.otel_endpoint.is_some();

        Self {
            service_name: obs.service_name.clone(),
            service_version: obs.service_version.clone(),
            // Environment enum → lowercased string
            environment: format!("{:?}", obs.environment).to_lowercase(),
            region: obs.region.clone(),
            level: obs.log_level.clone(),
            sampling_rate,
            otel_enabled,
            otel_endpoint: obs.otel_endpoint.clone(),
        }
    }

    /// Build from environment variables (useful for CLIs / tests).
    ///
    /// ENVIRONMENT            → environment
    /// SERVICE_NAME           → service_name
    /// REGION                 → region
    /// TRACING_SAMPLING_RATE  → sampling_rate (f64)
    #[deprecated(note = "from_env is deprecated; use rhelma-config UnifiedObservabilityConfig")]
    pub fn from_env() -> Self {
        let mut cfg = TracingConfig::default();

        if let Ok(env_val) = env::var("ENVIRONMENT") {
            let trimmed = env_val.trim();
            if !trimmed.is_empty() {
                cfg.environment = trimmed.to_string();
            }
        }

        if let Ok(name) = env::var("SERVICE_NAME") {
            let trimmed = name.trim();
            if !trimmed.is_empty() {
                cfg.service_name = trimmed.to_string();
            }
        }

        if let Ok(region) = env::var("REGION") {
            let trimmed = region.trim();
            if !trimmed.is_empty() {
                cfg.region = trimmed.to_string();
            }
        }

        if let Ok(rate) = env::var("TRACING_SAMPLING_RATE") {
            if let Ok(parsed) = rate.trim().parse::<f64>() {
                cfg.sampling_rate = parsed;
            }
        }

        cfg
    }

    pub fn with_service_name(mut self, name: String) -> Self {
        self.service_name = name;
        self
    }

    pub fn validate(&self) -> Result<(), TracingConfigError> {
        if self.service_name.trim().is_empty() {
            return Err(TracingConfigError::InvalidConfig(
                "service_name cannot be empty".into(),
            ));
        }

        if self.level.trim().is_empty() {
            return Err(TracingConfigError::InvalidConfig(
                "tracing level cannot be empty".into(),
            ));
        }

        // sampling_rate MUST be in [0.0, 1.0]
        if !(0.0..=1.0).contains(&self.sampling_rate) {
            return Err(TracingConfigError::InvalidConfig(
                "sampling_rate must be in [0.0,1.0]".into(),
            ));
        }

        // If OTEL is enabled, endpoint MUST exist and be a valid URL.
        if self.otel_enabled {
            match self
                .otel_endpoint
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                Some(ep) => {
                    Url::parse(ep).map_err(|e| TracingConfigError::InvalidConfig(e.to_string()))?;
                }
                None => {
                    return Err(TracingConfigError::InvalidConfig(
                        "otel_enabled=true but no otel_endpoint provided".into(),
                    ));
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum TracingConfigError {
    #[error("invalid tracing config: {0}")]
    /// Variant `InvalidConfig`.
    InvalidConfig(String),

    #[error("failed to initialize OTEL: {0}")]
    /// Variant `Otel`.
    Otel(String),

    #[error("subscriber setup failed: {0}")]
    /// Variant `Setup`.
    Setup(String),
}
