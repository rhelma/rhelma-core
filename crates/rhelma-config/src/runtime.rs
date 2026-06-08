//! Runtime identity helpers (CentralEnv + service identity) for Rhelma v5.2.

use serde::{Deserialize, Serialize};

use crate::errors::{ConfigError, ConfigResult};
use crate::CentralEnv;

/// Central runtime identity for a service.
///
/// This provides a single place to read and enforce:
/// - `CentralEnv` (region/environment/version/tenant)
/// - `RHELMA_SERVICE_NAME` (required in production)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CentralRuntime {
    /// Field `central`.
    pub central: CentralEnv,
    /// Field `service_name`.
    pub service_name: String,
}

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

impl CentralRuntime {
    /// Best-effort identity: uses `CentralEnv::from_env()` and defaults service_name to `"unknown-service"`.
    pub fn from_env() -> Self {
        let central = CentralEnv::from_env();
        let service_name =
            env_nonempty("RHELMA_SERVICE_NAME").unwrap_or_else(|| "unknown-service".to_string());
        Self {
            central,
            service_name,
        }
    }

    /// Strict identity:
    /// - uses `CentralEnv::from_env_strict()`
    /// - requires `RHELMA_SERVICE_NAME` in production (fail-closed)
    pub fn from_env_strict() -> ConfigResult<Self> {
        let central = CentralEnv::from_env_strict()?;

        let service_name =
            env_nonempty("RHELMA_SERVICE_NAME").ok_or(ConfigError::MissingField("service_name"))?;

        if central.environment == "production" && service_name == "unknown-service" {
            return Err(ConfigError::MissingField("service_name"));
        }

        Ok(Self {
            central,
            service_name,
        })
    }

    /// Strict identity + requires `RHELMA_ENV_MODEL_v1` opt-in.
    pub fn from_env_model_v1_strict() -> ConfigResult<Self> {
        let central = CentralEnv::from_env_model_v1_strict()?;
        let service_name =
            env_nonempty("RHELMA_SERVICE_NAME").ok_or(ConfigError::MissingField("service_name"))?;
        Ok(Self {
            central,
            service_name,
        })
    }
}
