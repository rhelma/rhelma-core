//! Core observability model types used by Rhelma services.

use serde::{Deserialize, Serialize};

use crate::sources::obs_var;
use crate::CentralEnv;

/// Logical environment of a service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Environment {
    /// Variant `Local`.
    Local,
    /// Variant `Development`.
    Development,
    /// Variant `Staging`.
    Staging,
    /// Variant `Production`.
    Production,
    /// Variant `Test`.
    Test,
    /// Custom/unknown environment name.
    ///
    /// Note: In strict validation mode (`CentralEnv::from_env_strict()`), only the
    /// predefined variants are accepted. This variant exists for legacy compatibility
    /// and should not be used in production with strict loaders.
    Custom(String),
}

/// Log output format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogFormat {
    /// Variant `Json`.
    Json,
    /// Variant `Text`.
    Text,
}

/// Performance profile hints for logger/tracing.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PerformanceProfile {
    /// Variant `LowLatency`.
    LowLatency,
    /// Variant `Balanced`.
    Balanced,
    /// Variant `HighThroughput`.
    HighThroughput,
}

/// Unified logger-related knobs exposed to downstream crates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggerConfig {
    /// Field `enabled`.
    pub enabled: bool,
    /// Field `json`.
    pub json: bool,
    /// Field `level`.
    pub level: String,
    /// Field `profile`.
    pub profile: String,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            json: true,
            level: "info".into(),
            profile: "Balanced".into(),
        }
    }
}

/// Unified tracing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Field `sampling_rate`.
    pub sampling_rate: f64,
    /// Field `otel_enabled`.
    pub otel_enabled: bool,
    /// Field `otel_endpoint`.
    pub otel_endpoint: Option<String>,
    /// When true, OTEL support is required by policy (typically production).
    #[serde(default)]
    pub otel_required: bool,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            sampling_rate: 1.0,
            otel_enabled: false,
            otel_endpoint: None,
            otel_required: false,
        }
    }
}

/// Unified metrics configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Field `enable_metrics`.
    pub enable_metrics: bool,
    /// Field `prometheus_port`.
    pub prometheus_port: Option<u16>,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enable_metrics: true,
            prometheus_port: Some(9090),
        }
    }
}

/// Single unified configuration object used by the observability layer.
///
/// This keeps logger/tracing/metrics knobs in one strongly typed struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedObservabilityConfig {
    /// Field `service_name`.
    pub service_name: String,
    /// Field `environment`.
    pub environment: Environment,
    /// Field `region`.
    pub region: String,
    /// Field `service_version`.
    pub service_version: String,

    /// Field `log_level`.
    pub log_level: String,
    /// Field `log_format`.
    pub log_format: LogFormat,
    /// Field `json_enabled`.
    pub json_enabled: bool,
    /// Field `console_enabled`.
    pub console_enabled: bool,
    /// Field `sampling_rate`.
    pub sampling_rate: f64,
    /// Field `performance_profile`.
    pub performance_profile: PerformanceProfile,

    /// Field `otel_enabled`.
    pub otel_enabled: bool,
    /// Field `otel_endpoint`.
    pub otel_endpoint: Option<String>,
    /// When true, OTEL support is required by policy (typically production).
    #[serde(default)]
    pub otel_required: bool,

    /// Field `enable_metrics`.
    pub enable_metrics: bool,
    /// Field `prometheus_port`.
    pub prometheus_port: u16,
}

impl UnifiedObservabilityConfig {
    /// Baseline configuration with safe defaults for a given service.
    pub fn baseline(service: String) -> Self {
        Self {
            service_name: service,
            environment: Environment::Development,
            region: "local".into(),
            service_version: "0.0.0-dev".into(),
            log_level: "info".into(),
            log_format: LogFormat::Json,
            json_enabled: true,
            console_enabled: true,
            sampling_rate: 1.0,
            performance_profile: PerformanceProfile::Balanced,
            otel_enabled: false,
            otel_endpoint: None,
            otel_required: false,
            enable_metrics: true,
            prometheus_port: 9090,
        }
    }

    /// Build a unified config from the central environment and process env.
    ///
    /// This is primarily used by the observability core in "env-only" mode.
    pub fn from_central_env(central: &CentralEnv, service_name: &str) -> Self {
        let mut cfg = Self::baseline(service_name.to_string());

        // Map environment string from CentralEnv to strongly typed Environment.
        cfg.environment = match central.environment.to_ascii_lowercase().as_str() {
            "local" => Environment::Local,
            "development" => Environment::Development,
            "staging" => Environment::Staging,
            "production" => Environment::Production,
            "test" => Environment::Test,
            other => Environment::Custom(other.to_string()),
        };

        // Policy defaults
        // In production, OTEL should be considered required by default.
        cfg.otel_required = matches!(cfg.environment, Environment::Production);

        cfg.region = central.region.clone();
        cfg.service_version = central.service_version.clone();

        // Simple env overrides for logger/tracing/metrics.
        if let Some(level) = obs_var("RHELMA_OBS__LOG_LEVEL", "RHELMA_OBSERVABILITY__LOG_LEVEL") {
            cfg.log_level = level;
        }

        if let Some(fmt) = obs_var("RHELMA_OBS__LOG_FORMAT", "RHELMA_OBSERVABILITY__LOG_FORMAT") {
            cfg.log_format = match fmt.to_ascii_lowercase().as_str() {
                "text" | "plain" => LogFormat::Text,
                _ => LogFormat::Json,
            };
        }

        if let Some(v) = obs_var("RHELMA_OBS__JSON_LOGS", "RHELMA_OBSERVABILITY__JSON_LOGS") {
            let l = v.to_ascii_lowercase();
            cfg.json_enabled = matches!(l.as_str(), "1" | "true" | "yes" | "on");
        }

        if let Some(v) = obs_var(
            "RHELMA_OBS__CONSOLE_LOGS",
            "RHELMA_OBSERVABILITY__CONSOLE_LOGS",
        ) {
            let l = v.to_ascii_lowercase();
            cfg.console_enabled = matches!(l.as_str(), "1" | "true" | "yes" | "on");
        }

        if let Some(v) = obs_var(
            "RHELMA_OBS__SAMPLING_RATE",
            "RHELMA_OBSERVABILITY__SAMPLING_RATE",
        ) {
            if let Ok(f) = v.parse::<f64>() {
                cfg.sampling_rate = f;
            }
        }

        if let Some(v) = obs_var(
            "RHELMA_OBS__PERFORMANCE_PROFILE",
            "RHELMA_OBSERVABILITY__PERFORMANCE_PROFILE",
        ) {
            cfg.performance_profile = match v.as_str() {
                "LowLatency" | "low_latency" => PerformanceProfile::LowLatency,
                "HighThroughput" | "high_throughput" => PerformanceProfile::HighThroughput,
                _ => PerformanceProfile::Balanced,
            };
        }

        if let Some(v) = obs_var(
            "RHELMA_OBS__OTEL_REQUIRED",
            "RHELMA_OBSERVABILITY__OTEL_REQUIRED",
        ) {
            let l = v.to_ascii_lowercase();
            cfg.otel_required = matches!(l.as_str(), "1" | "true" | "yes" | "on");
        }

        if let Some(v) = obs_var(
            "RHELMA_OBS__OTEL_ENABLED",
            "RHELMA_OBSERVABILITY__OTEL_ENABLED",
        ) {
            let l = v.to_ascii_lowercase();
            cfg.otel_enabled = matches!(l.as_str(), "1" | "true" | "yes" | "on");
        }

        if let Some(endpoint) = obs_var(
            "RHELMA_OBS__OTEL_ENDPOINT",
            "RHELMA_OBSERVABILITY__OTEL_ENDPOINT",
        ) {
            if !endpoint.trim().is_empty() {
                cfg.otel_endpoint = Some(endpoint);
            }
        }

        // Policy: if OTEL is required, force enable it (endpoint is validated separately).
        if cfg.otel_required {
            cfg.otel_enabled = true;
        }

        if let Some(v) = obs_var(
            "RHELMA_OBS__METRICS_ENABLED",
            "RHELMA_OBSERVABILITY__METRICS_ENABLED",
        ) {
            let l = v.to_ascii_lowercase();
            cfg.enable_metrics = matches!(l.as_str(), "1" | "true" | "yes" | "on");
        }

        if let Some(v) = obs_var(
            "RHELMA_OBS__PROMETHEUS_PORT",
            "RHELMA_OBSERVABILITY__PROMETHEUS_PORT",
        ) {
            if let Ok(p) = v.parse::<u16>() {
                cfg.prometheus_port = p;
            }
        }

        cfg
    }
}
