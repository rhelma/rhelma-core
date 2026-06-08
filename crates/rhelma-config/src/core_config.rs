//! Core (cross-service) configuration for Rhelma platform services.

use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;

use crate::errors::{ConfigError, ConfigResult};
use crate::sources::obs_var;
use crate::CentralEnv;

/// Supported file storage backends.
#[derive(Debug, Clone, Deserialize)]
pub enum FileBackend {
    /// Variant `Local`.
    Local,
    /// Variant `S3`.
    S3,
    /// Variant `Blob`.
    Blob,
}

/// Core (cross-service) infrastructure configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct CoreConfig {
    // Database
    /// Field `db_url`.
    pub db_url: Secret<String>,
    /// Field `db_read_replica_url`.
    pub db_read_replica_url: Option<Secret<String>>,
    /// Field `db_max_connections`.
    pub db_max_connections: Option<u32>,
    /// Field `db_min_connections`.
    pub db_min_connections: Option<u32>,

    // Redis / cache
    /// Field `redis_url`.
    pub redis_url: Option<Secret<String>>,
    /// Field `redis_default_ttl_secs`.
    pub redis_default_ttl_secs: Option<u64>,

    // File storage
    /// Field `file_backend`.
    pub file_backend: FileBackend,
    /// Field `file_local_root`.
    pub file_local_root: Option<String>,
    /// Field `file_s3_endpoint`.
    pub file_s3_endpoint: Option<String>,
    /// Field `file_s3_region`.
    pub file_s3_region: Option<String>,
    /// Field `file_s3_bucket`.
    pub file_s3_bucket: Option<String>,

    // Observability knobs
    /// Field `obs_json_logs`.
    pub obs_json_logs: bool,
    /// Field `obs_log_level`.
    pub obs_log_level: Option<String>,
    /// Field `obs_otel_endpoint`.
    pub obs_otel_endpoint: Option<String>,
    /// Field `obs_prometheus_port`.
    pub obs_prometheus_port: Option<u16>,
}

impl CoreConfig {
    /// Build CoreConfig from the RHELMA_* environment model.
    pub fn from_env(_central: &CentralEnv) -> ConfigResult<Self> {
        // Local helpers for strict numeric parsing.
        fn parse_opt_u32(env_name: &str, field: &'static str) -> ConfigResult<Option<u32>> {
            match std::env::var(env_name) {
                Ok(raw) => {
                    let v = raw.parse::<u32>().map_err(|_| ConfigError::InvalidValue {
                        field,
                        message: format!("invalid integer in {env_name}: {:?}", raw),
                    })?;
                    Ok(Some(v))
                }
                Err(std::env::VarError::NotPresent) => Ok(None),
                Err(_) => Err(ConfigError::InvalidValue {
                    field,
                    message: format!("unable to read env {env_name}"),
                }),
            }
        }

        fn parse_opt_u64(env_name: &str, field: &'static str) -> ConfigResult<Option<u64>> {
            match std::env::var(env_name) {
                Ok(raw) => {
                    let v = raw.parse::<u64>().map_err(|_| ConfigError::InvalidValue {
                        field,
                        message: format!("invalid integer in {env_name}: {:?}", raw),
                    })?;
                    Ok(Some(v))
                }
                Err(std::env::VarError::NotPresent) => Ok(None),
                Err(_) => Err(ConfigError::InvalidValue {
                    field,
                    message: format!("unable to read env {env_name}"),
                }),
            }
        }

        // Database
        let db_url = std::env::var("RHELMA_DB__URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .map_err(|_| ConfigError::MissingField("db_url"))?;

        let db_read_replica_url = std::env::var("RHELMA_DB__READ_REPLICA_URL").ok();
        let db_max_connections = parse_opt_u32("RHELMA_DB__MAX_CONNECTIONS", "db_max_connections")?;
        let db_min_connections = parse_opt_u32("RHELMA_DB__MIN_CONNECTIONS", "db_min_connections")?;

        // Redis
        let redis_url = std::env::var("RHELMA_REDIS__URL").ok();
        let redis_default_ttl_secs =
            parse_opt_u64("RHELMA_REDIS__DEFAULT_TTL_SECS", "redis_default_ttl_secs")?;

        // File storage
        let file_backend = match std::env::var("RHELMA_FILE__BACKEND") {
            Ok(raw) => match raw.as_str() {
                "s3" | "S3" => FileBackend::S3,
                "blob" | "BLOB" => FileBackend::Blob,
                "local" | "LOCAL" => FileBackend::Local,
                other => {
                    return Err(ConfigError::InvalidValue {
                        field: "file_backend",
                        message: format!("unsupported RHELMA_FILE__BACKEND value: {other:?}"),
                    });
                }
            },
            Err(std::env::VarError::NotPresent) => FileBackend::Local,
            Err(_) => {
                return Err(ConfigError::InvalidValue {
                    field: "file_backend",
                    message: "unable to read RHELMA_FILE__BACKEND".to_string(),
                });
            }
        };

        let file_local_root = std::env::var("RHELMA_FILE__LOCAL_ROOT").ok();
        let file_s3_endpoint = std::env::var("RHELMA_FILE__S3_ENDPOINT").ok();
        let file_s3_region = std::env::var("RHELMA_FILE__S3_REGION").ok();
        let file_s3_bucket = std::env::var("RHELMA_FILE__S3_BUCKET").ok();

        // Observability knobs
        let obs_json_logs = obs_var("RHELMA_OBS__JSON_LOGS", "RHELMA_OBSERVABILITY__JSON_LOGS")
            .map(|s| {
                let l = s.to_ascii_lowercase();
                matches!(l.as_str(), "1" | "true" | "yes" | "on")
            })
            .unwrap_or(true);

        let obs_log_level = obs_var("RHELMA_OBS__LOG_LEVEL", "RHELMA_OBSERVABILITY__LOG_LEVEL");

        let obs_otel_endpoint = obs_var(
            "RHELMA_OBS__OTEL_ENDPOINT",
            "RHELMA_OBSERVABILITY__OTEL_ENDPOINT",
        );

        let obs_prometheus_port = match obs_var(
            "RHELMA_OBS__PROMETHEUS_PORT",
            "RHELMA_OBSERVABILITY__PROMETHEUS_PORT",
        ) {
            Some(raw) => {
                let v = raw.parse::<u16>().map_err(|_| ConfigError::InvalidValue {
                    field: "obs_prometheus_port",
                    message: format!("invalid prometheus port: {:?}", raw),
                })?;
                Some(v)
            }
            None => None,
        };

        Ok(Self {
            db_url: Secret::new(db_url),
            db_read_replica_url: db_read_replica_url.map(Secret::new),
            db_max_connections,
            db_min_connections,
            redis_url: redis_url.map(Secret::new),
            redis_default_ttl_secs,
            file_backend,
            file_local_root,
            file_s3_endpoint,
            file_s3_region,
            file_s3_bucket,
            obs_json_logs,
            obs_log_level,
            obs_otel_endpoint,
            obs_prometheus_port,
        })
    }

    /// Tenant- and region-aware validation for CoreConfig.
    pub fn validate(&self, central: &CentralEnv) -> ConfigResult<()> {
        if self.db_url.expose_secret().is_empty() {
            return Err(ConfigError::InvalidValue {
                field: "db_url",
                message: "database URL must not be empty".to_string(),
            });
        }

        if let (Some(min), Some(max)) = (self.db_min_connections, self.db_max_connections) {
            if min > max {
                return Err(ConfigError::InvalidValue {
                    field: "db_min_connections",
                    message: "min_connections must be <= max_connections".to_string(),
                });
            }
        }

        if matches!(self.file_backend, FileBackend::S3)
            && central.tenant_id.is_some()
            && self.file_s3_bucket.as_deref().unwrap_or("").is_empty()
        {
            return Err(ConfigError::InvalidValue {
                field: "file_s3_bucket",
                message: "for multi-tenant S3 backend, RHELMA_FILE__S3_BUCKET is required"
                    .to_string(),
            });
        }

        Ok(())
    }
}
