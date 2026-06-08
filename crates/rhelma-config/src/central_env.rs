//! Central environment configuration for Rhelma platform services.

use serde::{Deserialize, Serialize};

use crate::errors::{ConfigError, ConfigResult};
use crate::models::Environment;

use rhelma_core::types::{RegionId, TenantId};

/// Returns true if the unified Rhelma env model sentinel is enabled.
///
/// Convention: `RHELMA_ENV_MODEL_v1=1|true`.
pub fn is_env_model_v1_enabled() -> bool {
    matches!(
        std::env::var("RHELMA_ENV_MODEL_v1")
            .ok()
            .map(|s| s.trim().to_ascii_lowercase()),
        Some(v) if v == "1" || v == "true" || v == "yes"
    )
}

/// Central environment configuration (region/environment/version/tenant).
///
/// This struct is intentionally string-based to keep the "raw" values
/// available for debugging and for compatibility with existing consumers.
/// Use [`CentralEnv::to_typed`] for a strongly-typed view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CentralEnv {
    /// Logical region for this process (e.g. "eu-west-1", "local").
    pub region: String,
    /// Deployment environment (e.g. "development", "staging", "production").
    pub environment: String,
    /// Service version string (e.g. git SHA or semver).
    pub service_version: String,
    /// Optional tenant identifier for multi-tenant deployments.
    pub tenant_id: Option<String>,
}

/// Strongly-typed view of [`CentralEnv`].
///
/// - `region` and `tenant_id` are validated using rhelma-core strong IDs.
/// - `environment` is mapped to rhelma-config's typed [`Environment`].
#[derive(Debug, Clone)]
pub struct CentralEnvTyped {
    /// Field `region`.
    pub region: RegionId,
    /// Field `environment`.
    pub environment: Environment,
    /// Field `service_version`.
    pub service_version: String,
    /// Field `tenant_id`.
    pub tenant_id: Option<TenantId>,
}

impl CentralEnv {
    /// Build from process environment (best-effort).
    ///
    /// - RHELMA_REGION (default: "local")
    /// - RHELMA_ENV (default: "development")
    /// - RHELMA_SERVICE_VERSION (default: "0.0.0-dev")
    /// - RHELMA_TENANT_ID (optional)
    pub fn from_env() -> Self {
        Self {
            region: std::env::var("RHELMA_REGION").unwrap_or_else(|_| "local".into()),
            environment: std::env::var("RHELMA_ENV").unwrap_or_else(|_| "development".into()),
            service_version: std::env::var("RHELMA_SERVICE_VERSION")
                .unwrap_or_else(|_| "0.0.0-dev".into()),
            tenant_id: std::env::var("RHELMA_TENANT_ID").ok(),
        }
    }

    /// Build from process environment using caller-provided defaults (best-effort).
    ///
    /// This is useful for legacy binaries that historically defaulted to a specific
    /// region or environment when RHELMA_* variables are absent, while still routing
    /// all env access through the CentralEnv abstraction.
    pub fn from_env_with_defaults(
        region_default: &str,
        environment_default: &str,
        service_version_default: &str,
    ) -> Self {
        Self {
            region: std::env::var("RHELMA_REGION").unwrap_or_else(|_| region_default.to_string()),
            environment: std::env::var("RHELMA_ENV")
                .or_else(|_| std::env::var("RHELMA_ENVIRONMENT"))
                .unwrap_or_else(|_| environment_default.to_string()),
            service_version: std::env::var("RHELMA_SERVICE_VERSION")
                .unwrap_or_else(|_| service_version_default.to_string()),
            tenant_id: std::env::var("RHELMA_TENANT_ID").ok(),
        }
    }

    /// Build from process environment (strict / contract-aligned).
    ///
    /// # Rules
    /// - `RHELMA_ENV` (or legacy `RHELMA_ENVIRONMENT`) is required and must be one of:
    ///   `local`, `development`, `staging`, `production`, `test`
    /// - `RHELMA_REGION` is required and must satisfy rhelma-core `RegionId` rules.
    /// - `RHELMA_SERVICE_VERSION` is required and must not be empty.
    /// - `RHELMA_TENANT_ID` is optional but must satisfy rhelma-core `TenantId` rules if present.
    ///
    /// # Production requirements (fail-closed)
    /// - `RHELMA_REGION` must NOT be `local` when `RHELMA_ENV=production`
    pub fn from_env_strict() -> ConfigResult<Self> {
        let env_raw = std::env::var("RHELMA_ENV")
            .or_else(|_| std::env::var("RHELMA_ENVIRONMENT"))
            .map_err(|_| ConfigError::MissingField("environment"))?;
        let env_norm = validate_env_name(&env_raw)?;

        let region_raw =
            std::env::var("RHELMA_REGION").map_err(|_| ConfigError::MissingField("region"))?;
        let region = region_raw.trim();
        if region.is_empty() {
            return Err(ConfigError::InvalidValue {
                field: "region",
                message: "RHELMA_REGION must not be empty".into(),
            });
        }
        // validate format with rhelma-core
        RegionId::parse(region).map_err(|e| ConfigError::InvalidValue {
            field: "region",
            message: e.to_string(),
        })?;

        if env_norm == "production" && region == "local" {
            return Err(ConfigError::InvalidValue {
                field: "region",
                message: "RHELMA_REGION must not be 'local' in production".into(),
            });
        }

        let service_version_raw = std::env::var("RHELMA_SERVICE_VERSION")
            .map_err(|_| ConfigError::MissingField("service_version"))?;
        let service_version = service_version_raw.trim().to_string();
        if service_version.is_empty() {
            return Err(ConfigError::InvalidValue {
                field: "service_version",
                message: "RHELMA_SERVICE_VERSION must not be empty".into(),
            });
        }

        let tenant_id = std::env::var("RHELMA_TENANT_ID")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        if let Some(ref t) = tenant_id {
            TenantId::parse(t).map_err(|e| ConfigError::InvalidValue {
                field: "tenant_id",
                message: e.to_string(),
            })?;
        }

        Ok(Self {
            region: region.to_string(),
            environment: env_norm,
            service_version,
            tenant_id,
        })
    }

    /// Contract entrypoint for services that explicitly opt into the unified env model.
    ///
    /// Fails closed if `RHELMA_ENV_MODEL_v1` is missing or not truthy.
    pub fn from_env_model_v1_strict() -> ConfigResult<Self> {
        if !is_env_model_v1_enabled() {
            return Err(ConfigError::MissingField("RHELMA_ENV_MODEL_v1"));
        }
        Self::from_env_strict()
    }

    /// Convert to a strongly-typed view (`RegionId`, `TenantId`, typed `Environment`).
    pub fn to_typed(&self) -> ConfigResult<CentralEnvTyped> {
        let region =
            RegionId::parse(self.region.trim()).map_err(|e| ConfigError::InvalidValue {
                field: "region",
                message: e.to_string(),
            })?;

        let env = map_environment(self.environment.trim());

        let tenant_id =
            match self.tenant_id.as_ref() {
                Some(t) if !t.trim().is_empty() => Some(TenantId::parse(t.trim()).map_err(
                    |e| ConfigError::InvalidValue {
                        field: "tenant_id",
                        message: e.to_string(),
                    },
                )?),
                _ => None,
            };

        Ok(CentralEnvTyped {
            region,
            environment: env,
            service_version: self.service_version.trim().to_string(),
            tenant_id,
        })
    }
}

fn validate_env_name(raw: &str) -> ConfigResult<String> {
    let norm = raw.trim().to_ascii_lowercase();
    match norm.as_str() {
        "local" | "development" | "staging" | "production" | "test" => Ok(norm),
        other => Err(ConfigError::InvalidValue {
            field: "environment",
            message: format!("unsupported RHELMA_ENV value: {other}"),
        }),
    }
}

fn map_environment(raw: &str) -> Environment {
    match raw.to_ascii_lowercase().as_str() {
        "local" => Environment::Local,
        "development" => Environment::Development,
        "staging" => Environment::Staging,
        "production" => Environment::Production,
        "test" => Environment::Test,
        other => Environment::Custom(other.to_string()),
    }
}
