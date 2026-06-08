//! Fluent builder for constructing UnifiedObservabilityConfig instances.

use serde_json::{Map, Value};

use crate::errors::ConfigResult;
use crate::merge::{deep_merge, insert_nested};
use crate::{CentralEnv, UnifiedObservabilityConfig};

/// Builder for composing a UnifiedObservabilityConfig from CentralEnv
/// and ad-hoc overrides (used mainly in tests and CLI tools).
#[derive(Debug)]
pub struct ConfigBuilder {
    central: CentralEnv,
    overrides: Map<String, Value>,
}

impl ConfigBuilder {
    pub fn new(central: CentralEnv) -> Self {
        Self {
            central,
            overrides: Map::new(),
        }
    }

    /// Add an override (supports dotted keys, but most Rhelma fields are top-level, e.g. "log_level").
    pub fn with(mut self, key: &str, value: Value) -> Self {
        insert_nested(&mut self.overrides, key, value);
        self
    }

    /// Convenience: set `log_level`.
    pub fn with_log_level(self, level: &str) -> Self {
        self.with("log_level", Value::from(level))
    }

    /// Convenience: set `region`.
    pub fn with_region(self, region: &str) -> Self {
        self.with("region", Value::from(region))
    }

    /// Convenience: set OTEL endpoint.
    pub fn with_otel_endpoint(self, endpoint: &str) -> Self {
        self.with("otel_endpoint", Value::from(endpoint))
    }

    /// Convenience: enable/disable OTEL.
    pub fn with_otel_enabled(self, enabled: bool) -> Self {
        self.with("otel_enabled", Value::from(enabled))
    }

    /// Convenience: require OTEL (policy; typically production).
    pub fn with_otel_required(self, required: bool) -> Self {
        self.with("otel_required", Value::from(required))
    }

    /// Convenience: enable/disable metrics.
    pub fn with_metrics_enabled(self, enabled: bool) -> Self {
        self.with("enable_metrics", Value::from(enabled))
    }

    /// Build a unified config for a given service name.
    pub fn build(self, service: &str) -> ConfigResult<UnifiedObservabilityConfig> {
        let base = UnifiedObservabilityConfig::from_central_env(&self.central, service);
        let mut v = serde_json::to_value(base)?;
        v = deep_merge(v, Value::Object(self.overrides));
        let cfg: UnifiedObservabilityConfig = serde_json::from_value(v)?;
        Ok(cfg)
    }
}
