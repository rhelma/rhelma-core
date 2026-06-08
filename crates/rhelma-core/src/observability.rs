use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::result::RhelmaResult;
use crate::{Environment, RhelmaError};

/// Unified observability configuration for a Rhelma service.
///
/// Contract notes:
/// - `rhelma-config` is the Source of Truth for environment/overrides.
/// - `rhelma-core` MUST NOT read process environment variables.
/// - This module provides a typed configuration shape + safe constructors only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedObservabilityConfig {
    /// Logical service name used in logs/metrics/traces.
    pub service_name: String,
    /// Environment string (lowercase): development|staging|production.
    pub environment: String,
    /// Region identifier, e.g. "eu-west-1".
    pub region: String,

    /// Whether JSON logging is enabled.
    pub json_logs: bool,

    /// OpenTelemetry exporter enabled.
    pub otlp_enabled: bool,
    /// Optional OTLP endpoint (e.g. http://otel-collector:4317).
    pub otlp_endpoint: Option<String>,
    /// Optional log level override.
    pub log_level: Option<String>,
}

impl UnifiedObservabilityConfig {
    /// Backward-compatible constructor used by older tests/services.
    ///
    /// IMPORTANT:
    /// - Does NOT read process environment variables.
    /// - Assumes `AppConfig` was already validated upstream (typically by rhelma-config).
    ///
    /// In v5.1 this is still the stable, ergonomic constructor.
    /// Prefer [`from_app_config_with_env`] in new code when you already have a validated
    /// [`Environment`] from rhelma-config.
    pub fn from_app_config(config: &AppConfig) -> RhelmaResult<Self> {
        let env = config.environment_typed()?;
        Self::from_app_config_with_env(config, env)
    }

    /// Contract-compliant constructor.
    ///
    /// The caller MUST supply the validated environment (typically from rhelma-config::CentralEnv).
    ///
    /// NO PANIC ALLOWED — always returns Result.
    pub fn from_app_config_with_env(
        config: &AppConfig,
        environment: Environment,
    ) -> RhelmaResult<Self> {
        // Contract rule: in production, service_name MUST be explicitly provided (fail-closed).
        let service_name = match environment {
            Environment::Production => {
                if let Some(name) = &config.service_name {
                    let t = name.trim();
                    if t.is_empty() {
                        return Err(RhelmaError::Config("service_name must not be empty".into()));
                    }
                    t.to_string()
                } else {
                    return Err(RhelmaError::Config(
                        "service_name is required in production (must be provided by rhelma-config)"
                            .into(),
                    ));
                }
            }
            _ => {
                if let Some(name) = &config.service_name {
                    let t = name.trim();
                    if t.is_empty() {
                        return Err(RhelmaError::Config("service_name must not be empty".into()));
                    }
                    t.to_string()
                } else {
                    "unknown-service".to_string()
                }
            }
        };

        // These knobs are expected to be resolved upstream (rhelma-config UnifiedObservabilityConfig).
        // We keep them as safe defaults here to avoid reading process env.
        let json_logs = config.json_logs.unwrap_or(false);

        Ok(Self {
            service_name,
            environment: environment.as_str().to_string(),
            region: config.region.clone(),
            json_logs,
            otlp_enabled: false,
            otlp_endpoint: None,
            log_level: None,
        })
    }
}
