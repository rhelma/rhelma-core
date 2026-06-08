//! Mapping helpers from `rhelma-config` `UnifiedObservabilityConfig` to component configs.

use rhelma_config::{
    Environment as CfgEnv, LogFormat as CfgLogFormat, PerformanceProfile as CfgProfile,
    UnifiedObservabilityConfig,
};

use rhelma_logger::{
    BackpressureStrategy, DispatchMode, Environment as LogEnv, LogFormat as LogFmt, LoggerConfig,
    PerformanceProfile as LogProfile,
};
use rhelma_metrics::MetricsConfig;
use rhelma_tracing::TracingConfig;

/// Convert a config environment enum to a stable lowercase string.
#[must_use]
pub fn env_to_string(e: &CfgEnv) -> String {
    match e {
        CfgEnv::Local => "local".to_string(),
        CfgEnv::Development => "development".to_string(),
        CfgEnv::Staging => "staging".to_string(),
        CfgEnv::Production => "production".to_string(),
        CfgEnv::Test => "test".to_string(),
        CfgEnv::Custom(v) => v.trim().to_ascii_lowercase(),
    }
}

fn sanitize_log_level(level: &str) -> String {
    let v = level.trim().to_ascii_lowercase();
    match v.as_str() {
        "trace" | "debug" | "info" | "warn" | "error" => v,
        _ => "info".to_string(),
    }
}

fn map_log_format(f: &CfgLogFormat) -> LogFmt {
    match f {
        CfgLogFormat::Json => LogFmt::Json,
        CfgLogFormat::Text => LogFmt::Text,
    }
}

fn map_profile(p: CfgProfile) -> LogProfile {
    match p {
        CfgProfile::LowLatency => LogProfile::LowLatency,
        CfgProfile::Balanced => LogProfile::Balanced,
        CfgProfile::HighThroughput => LogProfile::HighThroughput,
    }
}

fn map_logger_environment(env: &CfgEnv) -> LogEnv {
    match env {
        CfgEnv::Local => LogEnv::Local,
        CfgEnv::Development => LogEnv::Development,
        CfgEnv::Staging => LogEnv::Staging,
        CfgEnv::Production => LogEnv::Production,
        CfgEnv::Test => LogEnv::Test,
        CfgEnv::Custom(_) => LogEnv::Unknown,
    }
}

fn derive_logger_tuning(profile: LogProfile) -> (DispatchMode, BackpressureStrategy, usize, u64) {
    // Keep this conservative. The logger crate also has its own defaults.
    match profile {
        LogProfile::LowLatency => (
            DispatchMode::Async,
            BackpressureStrategy::DropNewest,
            4096,
            50,
        ),
        LogProfile::Balanced => (
            DispatchMode::Async,
            BackpressureStrategy::DropNewest,
            8192,
            200,
        ),
        LogProfile::HighThroughput => (
            DispatchMode::Async,
            BackpressureStrategy::DropOldest,
            16384,
            500,
        ),
    }
}

/// Convert unified config into a logger config.
#[must_use]
pub fn to_logger_config(c: &UnifiedObservabilityConfig) -> LoggerConfig {
    let performance_profile = map_profile(c.performance_profile);
    let (dispatch_mode, backpressure, queue_capacity, flush_interval_ms) =
        derive_logger_tuning(performance_profile);

    LoggerConfig {
        service_name: c.service_name.clone(),
        service_version: c.service_version.clone(),
        service_instance_id: None,
        environment: map_logger_environment(&c.environment),
        region: c.region.clone(),
        log_level: sanitize_log_level(&c.log_level),
        log_format: map_log_format(&c.log_format),
        json_enabled: c.json_enabled,
        console_enabled: c.console_enabled,
        sampling_rate: c.sampling_rate.clamp(0.0, 1.0),
        performance_profile,
        dispatch_mode,
        queue_capacity,
        backpressure,
        flush_interval_ms,
    }
}

/// Convert unified config into a tracing config.
#[must_use]
pub fn to_tracing_config(c: &UnifiedObservabilityConfig) -> TracingConfig {
    let mut t = TracingConfig::from_unified(c);
    t.level = sanitize_log_level(&c.log_level);
    t
}

/// Convert unified config into a metrics config.
#[must_use]
pub fn to_metrics_config(c: &UnifiedObservabilityConfig) -> MetricsConfig {
    let mut m = MetricsConfig::new(&c.service_name);
    m.environment = env_to_string(&c.environment);
    m.region = Some(c.region.clone());
    m.service_version = Some(c.service_version.clone());

    // Stable base labels expected by wiring tests + dashboards.
    m.default_labels = vec![
        ("service_name".to_string(), c.service_name.clone()),
        ("environment".to_string(), env_to_string(&c.environment)),
        ("region".to_string(), c.region.clone()),
        ("service_version".to_string(), c.service_version.clone()),
    ];

    m
}
