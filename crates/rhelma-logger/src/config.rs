use serde::{Deserialize, Serialize};
use thiserror::Error;

fn resolve_instance_id() -> Option<String> {
    // Prefer explicit Rhelma var, then common platform env vars.
    for key in ["RHELMA_INSTANCE_ID", "POD_NAME", "HOSTNAME"] {
        if let Ok(v) = std::env::var(key) {
            let v = v.trim().to_string();
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

/// Environment in which the service is running.
///
/// This enum is aligned with `rhelma-config` v5.2 (Local/Dev/Staging/Prod/Test).
/// Unknown/custom values are mapped to `Unknown`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
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
    #[serde(other)]
    /// Variant `Unknown`.
    Unknown,
}

/// Log output format.
/// ⚠️ NOTE: در rhelma-logger v0.18 همه خروجی‌ها JSON هستند.
/// این enum فقط برای سازگاری با unified config نگه داشته شده.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LogFormat {
    /// Variant `Json`.
    Json,
    /// Text/console output (kept for unified config compatibility).
    #[serde(
        alias = "console",
        alias = "text",
        alias = "pretty",
        alias = "pretty_json",
        alias = "PrettyJson"
    )]
    /// Variant `Text`.
    Text,
}

/// Performance profile for logging hot path.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PerformanceProfile {
    /// Variant `LowLatency`.
    LowLatency,
    /// Variant `Balanced`.
    Balanced,
    /// Variant `HighThroughput`.
    HighThroughput,
}

/// Dispatch mode for the logger.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DispatchMode {
    /// Variant `Sync`.
    Sync,
    /// Variant `Async`.
    Async,
}

/// Backpressure strategy when the async queue is full.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BackpressureStrategy {
    /// Variant `DropNewest`.
    DropNewest,
    /// Variant `DropOldest`.
    DropOldest,
    /// Variant `Block`.
    Block,
}

/// Logger configuration used by `RhelmaLogger`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggerConfig {
    /// Field `service_name`.
    pub service_name: String,
    /// Field `service_version`.
    pub service_version: String,

    /// Service instance identifier (pod/container/host).
    /// If not provided, rhelma-logger will try to infer it from environment variables.
    #[serde(default)]
    pub service_instance_id: Option<String>,

    /// Field `environment`.
    pub environment: Environment,
    /// Field `region`.
    pub region: String,
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
    /// Field `dispatch_mode`.
    pub dispatch_mode: DispatchMode,
    /// Field `queue_capacity`.
    pub queue_capacity: usize,
    /// Field `backpressure`.
    pub backpressure: BackpressureStrategy,
    /// Field `flush_interval_ms`.
    pub flush_interval_ms: u64,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            service_name: "unknown".into(),
            service_version: "0.0.0".into(),
            service_instance_id: resolve_instance_id(),
            environment: Environment::Development,
            region: "local".into(),
            log_level: "info".into(),
            log_format: LogFormat::Json,
            json_enabled: true,
            console_enabled: false,
            sampling_rate: 1.0,
            performance_profile: PerformanceProfile::Balanced,
            dispatch_mode: DispatchMode::Async,
            queue_capacity: 8192,
            backpressure: BackpressureStrategy::DropNewest,
            flush_interval_ms: 200, // ← اصلاح شد: هماهنگ با Balanced profile
        }
    }
}

#[derive(Debug, Error)]
pub enum LoggerError {
    #[error("logger already initialised")]
    /// Variant `AlreadyInitialised`.
    AlreadyInitialised,

    #[error("failed to install global dispatcher: {0}")]
    /// Variant `Dispatcher`.
    Dispatcher(String),

    #[error("invalid configuration: {0}")]
    /// Variant `InvalidConfig`.
    InvalidConfig(String),
}

impl LoggerConfig {
    /// Basic configuration validation (fast, non-allocating checks).
    pub fn validate(&self) -> Result<(), LoggerError> {
        if self.queue_capacity == 0 {
            return Err(LoggerError::InvalidConfig(
                "queue_capacity must be > 0".into(),
            ));
        }

        if !(0.0..=1.0).contains(&self.sampling_rate) {
            return Err(LoggerError::InvalidConfig(
                "sampling_rate must be in [0.0, 1.0]".into(),
            ));
        }

        if self.service_name.trim().is_empty() {
            return Err(LoggerError::InvalidConfig(
                "service_name must not be empty".into(),
            ));
        }

        if self.service_version.trim().is_empty() {
            return Err(LoggerError::InvalidConfig(
                "service_version must not be empty".into(),
            ));
        }

        if self.region.trim().is_empty() {
            return Err(LoggerError::InvalidConfig(
                "region must not be empty".into(),
            ));
        }

        let lvl = self.log_level.trim().to_ascii_lowercase();
        match lvl.as_str() {
            "trace" | "debug" | "info" | "warn" | "error" | "critical" => {}
            _ => {
                return Err(LoggerError::InvalidConfig(
                    "log_level must be one of: trace|debug|info|warn|error|critical".into(),
                ))
            }
        }

        // flush_interval_ms is only meaningful in Async mode; it's harmless in Sync mode.
        Ok(())
    }
}

#[cfg(feature = "with-config")]
impl LoggerConfig {
    /// ساخت LoggerConfig از unified config
    pub fn from_unified(
        unified: &rhelma_config::UnifiedObservabilityConfig,
        core: Option<&rhelma_config::CoreConfig>,
    ) -> Self {
        use rhelma_config::Environment as ObsEnvironment;
        use rhelma_config::LogFormat as ObsLogFormat;
        use rhelma_config::PerformanceProfile as ObsPerf;
        use Environment::*;
        use LogFormat::*;
        use PerformanceProfile::*;

        // 1) Map environment
        let env = match unified.environment {
            ObsEnvironment::Local => Local,
            ObsEnvironment::Development => Development,
            ObsEnvironment::Staging => Staging,
            ObsEnvironment::Production => Production,
            ObsEnvironment::Test => Test,
            // Custom/Unknown environments map to Unknown.
            _ => Unknown,
        };

        // 2) Base + overrides
        let mut json_enabled = unified.json_enabled;
        let mut log_level = unified.log_level.clone();

        if let Some(core_cfg) = core {
            if core_cfg.obs_json_logs {
                json_enabled = true;
            }
            if let Some(ref lvl) = core_cfg.obs_log_level {
                log_level = lvl.clone();
            }
        }

        // Production always JSON
        if matches!(unified.environment, ObsEnvironment::Production) {
            json_enabled = true;
        }

        // 3) LogFormat mapping
        let log_format = if json_enabled {
            Json
        } else {
            match unified.log_format {
                ObsLogFormat::Json => Json,
                _ => Text,
            }
        };

        // 4) Performance profile
        let perf = match unified.performance_profile {
            ObsPerf::LowLatency => LowLatency,
            ObsPerf::HighThroughput => HighThroughput,
            ObsPerf::Balanced => Balanced,
        };

        // 5) Runtime tuning
        let (dispatch_mode, queue_capacity, backpressure, flush_interval_ms) = match perf {
            LowLatency => (
                DispatchMode::Sync, // latency-sensitive
                4_096,
                BackpressureStrategy::DropNewest,
                0, // flush impacts latency
            ),
            Balanced => (
                DispatchMode::Async,
                8_192,
                BackpressureStrategy::DropNewest,
                200,
            ),
            HighThroughput => (
                DispatchMode::Async,
                32_768,
                BackpressureStrategy::DropOldest,
                500,
            ),
        };

        Self {
            service_name: unified.service_name.clone(),
            service_version: unified.service_version.clone(),
            service_instance_id: resolve_instance_id(),
            environment: env,
            region: unified.region.clone(),
            log_level,
            log_format,
            json_enabled,
            console_enabled: !json_enabled,
            sampling_rate: unified.sampling_rate,
            performance_profile: perf,
            dispatch_mode,
            queue_capacity,
            backpressure,
            flush_interval_ms,
        }
    }
}
