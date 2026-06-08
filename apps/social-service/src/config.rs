#![forbid(unsafe_code)]

use rhelma_config::{CentralEnv, CoreConfig};
use secrecy::{ExposeSecret, Secret};
use std::env;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct SocialConfig {
    /// Canonical env (region + environment + version).
    pub central: CentralEnv,
    /// Core configuration (DB/Redis).
    pub core: CoreConfig,

    /// Service name used for tracing/metrics.
    pub service_name: String,

    /// HTTP listen address, e.g. "0.0.0.0:8085".
    pub listen_addr: String,

    /// Redis URL used by rhelma-auth and token revocation checks.
    pub redis_url: Secret<String>,

    /// Default page size for feed endpoints.
    pub feed_default_limit: u32,
    /// Hard cap for feed endpoints.
    pub feed_max_limit: u32,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid configuration: {0}")]
    Invalid(String),
}

impl SocialConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let central = CentralEnv::from_env_strict()
            .map_err(|e| ConfigError::Invalid(format!("central env: {e}")))?;
        let core = CoreConfig::from_env(&central)
            .map_err(|e| ConfigError::Invalid(format!("core config: {e}")))?;

        let service_name = env::var("RHELMA_SERVICE_NAME")
            .or_else(|_| env::var("RHELMA_SOCIAL_SERVICE_NAME"))
            .or_else(|_| env::var("RHELMA_SOCIAL__SERVICE_NAME"))
            .unwrap_or_else(|_| "social-service".to_string());

        let listen_addr = env::var("RHELMA_SOCIAL_LISTEN_ADDR")
            .or_else(|_| env::var("RHELMA_SOCIAL__LISTEN_ADDR"))
            .unwrap_or_else(|_| "0.0.0.0:8085".to_string());

        let core_redis = core.redis_url.as_ref().map(|s| s.expose_secret().clone());

        let redis_url_raw = env::var("RHELMA_REDIS__URL")
            .or_else(|_| env::var("RHELMA_REDIS_URL"))
            .ok()
            .or(core_redis)
            .unwrap_or_else(|| "redis://127.0.0.1/".to_string());
        let redis_url = Secret::new(redis_url_raw);

        let feed_default_limit = env::var("RHELMA_SOCIAL_FEED_DEFAULT_LIMIT")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(20);

        let feed_max_limit = env::var("RHELMA_SOCIAL_FEED_MAX_LIMIT")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(100);

        Ok(Self {
            central,
            core,
            service_name,
            listen_addr,
            redis_url,
            feed_default_limit,
            feed_max_limit,
        })
    }

    pub fn redis_url(&self) -> &str {
        self.redis_url.expose_secret()
    }

    pub fn is_prod(&self) -> bool {
        self.central.environment.eq_ignore_ascii_case("production")
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.service_name.trim().is_empty() {
            return Err(ConfigError::Invalid(
                "service_name must not be empty".into(),
            ));
        }
        if self.central.region.trim().is_empty() {
            return Err(ConfigError::Invalid("region must not be empty".into()));
        }
        if self.feed_default_limit == 0 || self.feed_default_limit > self.feed_max_limit {
            return Err(ConfigError::Invalid(
                "feed_default_limit must be between 1 and feed_max_limit".into(),
            ));
        }
        if self.feed_max_limit == 0 || self.feed_max_limit > 500 {
            return Err(ConfigError::Invalid(
                "feed_max_limit must be between 1 and 500".into(),
            ));
        }
        Ok(())
    }
}
