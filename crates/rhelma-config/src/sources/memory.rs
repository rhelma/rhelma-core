//! In-memory config sources used mostly for tests and examples.

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::errors::ConfigResult;
use crate::provider::{AsyncConfigProvider, SyncConfigProvider};

/// Simple in-memory config storage that can be used with both
/// sync and async resolvers.
#[derive(Default, Clone)]
pub struct MemoryConfig {
    /// Field `defaults`.
    pub defaults: Option<Value>,
    /// Field `regions`.
    pub regions: HashMap<String, Value>,
    /// Field `services`.
    pub services: HashMap<(String, String), Value>,
}

impl MemoryConfig {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn with_defaults(mut self, v: Value) -> Self {
        self.defaults = Some(v);
        self
    }

    pub fn with_region(mut self, region: &str, v: Value) -> Self {
        self.regions.insert(region.to_string(), v);
        self
    }

    pub fn with_service(mut self, region: &str, service: &str, v: Value) -> Self {
        self.services
            .insert((region.to_string(), service.to_string()), v);
        self
    }

    /// Preloaded development defaults.
    pub fn with_defaults_dev() -> Self {
        Self::empty().with_defaults(json!({
            "log_level": "debug",
            "prometheus_port": 9090
        }))
    }

    /// Preloaded production defaults.
    pub fn with_defaults_prod() -> Self {
        Self::empty().with_defaults(json!({
            "log_level": "info",
            "prometheus_port": 9090
        }))
    }
}

impl SyncConfigProvider for MemoryConfig {
    fn load_defaults(&self) -> ConfigResult<Option<Value>> {
        Ok(self.defaults.clone())
    }

    fn load_region_config(&self, region: &str) -> ConfigResult<Option<Value>> {
        Ok(self.regions.get(region).cloned())
    }

    fn load_service_config(&self, region: &str, service: &str) -> ConfigResult<Option<Value>> {
        Ok(self
            .services
            .get(&(region.to_string(), service.to_string()))
            .cloned())
    }
}

#[async_trait::async_trait]
impl AsyncConfigProvider for MemoryConfig {
    async fn load_defaults(&self) -> ConfigResult<Option<Value>> {
        Ok(self.defaults.clone())
    }

    async fn load_region_config(&self, region: &str) -> ConfigResult<Option<Value>> {
        Ok(self.regions.get(region).cloned())
    }

    async fn load_service_config(
        &self,
        region: &str,
        service: &str,
    ) -> ConfigResult<Option<Value>> {
        Ok(self
            .services
            .get(&(region.to_string(), service.to_string()))
            .cloned())
    }
}
