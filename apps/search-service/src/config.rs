use rhelma_config::CentralEnv;
use serde::Deserialize;
use std::env;
use thiserror::Error;

/// Configuration for the search-service.
///
/// This configuration is intentionally simple but aligned with Rhelma contract
/// expectations: explicit service identity, region, and backend endpoints.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchConfig {
    /// Service name used in traces, metrics, and events.
    pub service_name: String,
    /// Deployment environment (e.g. "dev", "staging", "prod").
    pub environment: String,
    /// Region identifier (e.g. "eu-west-1").
    pub region: String,

    /// HTTP listen address, e.g. "0.0.0.0:8080".
    pub listen_addr: String,

    /// Qdrant endpoint URL.
    pub qdrant_url: String,
    /// Meilisearch endpoint URL.
    pub meili_url: String,

    /// Default index name for document search.
    pub default_index: String,

    /// Optional Redis/cache endpoint for query/embedding caching.
    pub redis_url: Option<String>,

    /// Name of the embedding model used.
    pub embedding_model: String,

    /// Maximum page size for search results.
    pub max_page_size: u32,

    /// Whether enhanced hybrid search is enabled by default.
    pub enhanced_hybrid_default: bool,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required environment variable: {0}")]
    /// Variant `MissingEnv`.
    MissingEnv(&'static str),

    #[error("invalid configuration: {0}")]
    /// Variant `Invalid`.
    Invalid(String),
}

impl SearchConfig {
    /// Load configuration from environment variables.
    ///
    /// This keeps the service usable standalone; in a full Rhelma deployment,
    /// this can be wired through rhelma-config's UnifiedAppConfig.
    pub fn from_env() -> Result<Self, ConfigError> {
        let central = CentralEnv::from_env_strict()
            .map_err(|e| ConfigError::Invalid(format!("central env: {e}")))?;

        let service_name = env::var("RHELMA_SERVICE_NAME")
            .or_else(|_| env::var("RHELMA_SEARCH_SERVICE_NAME"))
            .unwrap_or_else(|_| "search-service".into());

        Ok(Self {
            service_name,
            environment: central.environment,
            region: central.region,

            listen_addr: env_var("RHELMA_SEARCH_LISTEN_ADDR")
                .unwrap_or_else(|| "0.0.0.0:8080".into()),
            qdrant_url: env_req("RHELMA_SEARCH_QDRANT_URL")?,
            meili_url: env_req("RHELMA_SEARCH_MEILI_URL")?,
            default_index: env_var("RHELMA_SEARCH_DEFAULT_INDEX")
                .unwrap_or_else(|| "documents".into()),
            redis_url: env_var("RHELMA_SEARCH_REDIS_URL"),
            embedding_model: env_var("RHELMA_SEARCH_EMBEDDING_MODEL")
                .unwrap_or_else(|| "bge-small-en".into()),
            max_page_size: env_var("RHELMA_SEARCH_MAX_PAGE_SIZE")
                .and_then(|v| v.parse().ok())
                .unwrap_or(50),
            enhanced_hybrid_default: env_var("RHELMA_SEARCH_ENHANCED_HYBRID_DEFAULT")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(true),
        })
    }

    /// Validate config according to basic Rhelma contract expectations.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.service_name.trim().is_empty() {
            return Err(ConfigError::Invalid(
                "service_name must not be empty".into(),
            ));
        }
        if self.region.trim().is_empty() {
            return Err(ConfigError::Invalid("region must not be empty".into()));
        }
        if self.max_page_size == 0 || self.max_page_size > 1000 {
            return Err(ConfigError::Invalid(
                "max_page_size must be between 1 and 1000".into(),
            ));
        }
        Ok(())
    }
}

fn env_req(key: &'static str) -> Result<String, ConfigError> {
    env::var(key).map_err(|_| ConfigError::MissingEnv(key))
}

fn env_var(key: &str) -> Option<String> {
    std::env::var(key).ok()
}
