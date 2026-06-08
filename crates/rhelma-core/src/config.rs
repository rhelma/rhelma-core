//! Application configuration model for Rhelma services.
//!
//! Follows Rhelma Contract v5.1 — Zero-Trust Configuration Layer.
//!
//! Key Principles:
//! - `from_env_only()` loads raw config, NO validation.
//! - `validate_all()` performs strict checks.
//! - No silent fallback for invalid region.
//! - Environment must be one of development | staging | production.
//! - Region must follow lowercase `[a-z0-9-]{3,}` pattern.

use crate::Environment;
use crate::RhelmaError;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    /// Execution environment: development | staging | production.
    pub environment: String,

    /// Deployment region (Rhelma global multi-region identity).
    pub region: String,

    /// Whether JSON logs are explicitly enabled.
    ///
    /// default = false (per v5.1 Observability Contract — JSON logs are opt-in)
    pub json_logs: Option<bool>,

    /// Logical service name (for observability & auth).
    pub service_name: Option<String>,

    /// Logical version identifier.
    pub service_version: Option<String>,

    /// Optional default tenant tier (SaaS override).
    pub default_tenant_tier: Option<String>,
}

impl AppConfig {
    // --------------------------------------------------------------------
    // LOAD ONLY — NO VALIDATION
    //
    // The purpose of from_env_only() is to load configuration values
    // exactly as supplied by the environment. Validation MUST NOT happen
    // here. This follows v5.1 Zero-Trust rules: load first → validate later.
    // --------------------------------------------------------------------
    pub fn from_env_only() -> Result<Self, RhelmaError> {
        const KEY_MACH_ENV: &str = "RHELMA_ENV";
        const KEY_MACH_ENVIRONMENT: &str = "RHELMA_ENVIRONMENT";
        const KEY_MACH_REGION: &str = "RHELMA_REGION";

        let environment = std::env::var(KEY_MACH_ENV)
            .or_else(|_| std::env::var(KEY_MACH_ENVIRONMENT))
            .unwrap_or_else(|_| "development".to_string())
            .trim()
            .to_string();

        let region = std::env::var(KEY_MACH_REGION)
            .unwrap_or_else(|_| "local".to_string())
            .trim()
            .to_string();

        let json_logs = std::env::var("RHELMA_JSON_LOGS")
            .ok()
            .and_then(|v| v.parse::<bool>().ok());

        let service_name = std::env::var("RHELMA_SERVICE_NAME").ok();
        let service_version = std::env::var("RHELMA_SERVICE_VERSION").ok();
        let default_tenant_tier = std::env::var("RHELMA_DEFAULT_TENANT_TIER").ok();

        let cfg = Self {
            environment,
            region,
            json_logs,
            service_name,
            service_version,
            default_tenant_tier,
        };

        // ❗ DO NOT VALIDATE HERE — only load raw configuration
        Ok(cfg)
    }

    // --------------------------------------------------------------------
    // JSON LOGGING DEFAULTS
    // Observability v5.1: JSON logs are opt-in
    // --------------------------------------------------------------------
    pub fn json_logs_enabled(&self) -> bool {
        self.json_logs.unwrap_or(false)
    }

    // --------------------------------------------------------------------
    // FULL VALIDATION
    // MUST BE CALLED BY THE SERVICE during initialization.
    //
    // If validation fails → startup must abort (fail-fast).
    // --------------------------------------------------------------------
    pub fn validate_all(&self) -> Result<(), RhelmaError> {
        // ------ validate environment ------
        let env = self.environment.trim();

        if env.is_empty() {
            return Err(RhelmaError::Config(
                "environment must not be empty".to_string(),
            ));
        }

        const VALID_ENVS: [&str; 3] = ["development", "staging", "production"];

        if !VALID_ENVS.contains(&env) {
            return Err(RhelmaError::Config(format!(
                "environment must be one of {:?}, got '{}'",
                VALID_ENVS, env
            )));
        }

        // ------ validate region ------
        let region = self.region.trim();

        if region.is_empty() {
            return Err(RhelmaError::Config("region must not be empty".to_string()));
        }

        // Region MUST follow `[a-z0-9-]{3,}` (Rhelma multi-region rule)
        if !Self::is_valid_region(region) {
            return Err(RhelmaError::Config(format!(
                "invalid region format: {}",
                self.region
            )));
        }

        Ok(())
    }

    // ---------------------------
    // region validation helper
    // ---------------------------
    #[inline]
    fn is_valid_region(r: &str) -> bool {
        r.len() >= 3
            && r.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    }

    /// Adapter for typed environment.
    ///
    /// WARNING:
    /// This method assumes the environment string was already
    /// validated by rhelma-config.
    pub fn environment_typed(&self) -> Result<Environment, RhelmaError> {
        match self.environment.as_str() {
            "development" => Ok(Environment::Development),
            "staging" => Ok(Environment::Staging),
            "production" => Ok(Environment::Production),
            _ => Err(RhelmaError::Config(format!(
                "invalid environment '{}'",
                self.environment
            ))),
        }
    }
}
