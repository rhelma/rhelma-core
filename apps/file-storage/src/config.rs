#![forbid(unsafe_code)]

use secrecy::Secret;
use serde::Deserialize;
use std::net::SocketAddr;

use rhelma_config::prelude::CentralEnv;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum StorageProviderKind {
    /// Variant `LocalFs`.
    LocalFs,
    /// Variant `S3`.
    S3,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileStorageConfig {
    // ---------------------------------------------------------------------
    // Central identity (must align with Rhelma core conventions)
    // ---------------------------------------------------------------------
    /// Field `service_name`.
    pub service_name: String,
    /// Field `environment`.
    pub environment: String,
    /// Field `region`.
    pub region: String,

    // ---------------------------------------------------------------------
    // Server
    // ---------------------------------------------------------------------
    /// Field `listen_addr`.
    pub listen_addr: SocketAddr,

    // ---------------------------------------------------------------------
    // Database
    // ---------------------------------------------------------------------
    /// Field `database_url`.
    pub database_url: String,
    /// Field `database_max_connections`.
    pub database_max_connections: u32,

    // ---------------------------------------------------------------------
    // Storage policy
    // ---------------------------------------------------------------------
    /// Field `max_file_size_bytes`.
    pub max_file_size_bytes: u64,
    /// Field `allowed_mime_prefixes`.
    pub allowed_mime_prefixes: Vec<String>,
    /// Field `antivirus_enabled`.
    pub antivirus_enabled: bool,
    /// Field `encryption_at_rest`.
    pub encryption_at_rest: bool,
    /// Field `retention_days`.
    pub retention_days: Option<u32>,

    // ---------------------------------------------------------------------
    // Rate limiting (defense in depth; edge/gateway should still enforce)
    // ---------------------------------------------------------------------
    /// Field `rate_limit_read_rpm`.
    pub rate_limit_read_rpm: u32,
    /// Field `rate_limit_write_rpm`.
    pub rate_limit_write_rpm: u32,
    /// Field `rate_limit_burst`.
    pub rate_limit_burst: u32,

    // ---------------------------------------------------------------------
    // Providers
    // ---------------------------------------------------------------------
    /// Field `default_provider`.
    pub default_provider: StorageProviderKind,

    // LocalFS
    /// Field `local_root`.
    pub local_root: String,

    // S3
    /// Field `s3_endpoint`.
    pub s3_endpoint: Option<String>,
    /// Field `s3_region`.
    pub s3_region: Option<String>,
    /// Field `s3_bucket`.
    pub s3_bucket: Option<String>,
    /// Field `s3_access_key`.
    pub s3_access_key: Option<Secret<String>>,
    /// Field `s3_secret_key`.
    pub s3_secret_key: Option<Secret<String>>,

    // ---------------------------------------------------------------------
    // CORS
    // ---------------------------------------------------------------------
    /// Field `cors_allowed_origins`.
    pub cors_allowed_origins: Vec<String>,
}

impl FileStorageConfig {
    pub fn is_prod(&self) -> bool {
        self.environment.eq_ignore_ascii_case("production")
            || self.environment.eq_ignore_ascii_case("prod")
    }

    pub fn bind_addr(&self) -> SocketAddr {
        self.listen_addr
    }

    /// Strict env loader.
    ///
    /// Uses CentralEnv as the single source of truth for environment/region.
    pub fn from_env_strict() -> Result<Self, String> {
        let central = CentralEnv::from_env_strict().map_err(|e| format!("central env: {e}"))?;

        let service_name = env_opt("RHELMA_SERVICE_NAME")
            .or_else(|| env_opt("RHELMA_FILE_STORAGE__SERVICE_NAME"))
            .unwrap_or_else(|| "file-storage".to_string());

        let listen_raw = env_opt("RHELMA_FILE_STORAGE__LISTEN_ADDR")
            .unwrap_or_else(|| "0.0.0.0:3005".to_string());
        let listen_addr: SocketAddr = listen_raw
            .parse()
            .map_err(|e| format!("RHELMA_FILE_STORAGE__LISTEN_ADDR invalid: {e}"))?;

        let database_url = env_opt("RHELMA_FILE_STORAGE__DATABASE_URL")
            .or_else(|| env_opt("RHELMA_DATABASE_URL"))
            .ok_or_else(|| {
                "missing RHELMA_FILE_STORAGE__DATABASE_URL (or RHELMA_DATABASE_URL)".to_string()
            })?;

        let database_max_connections =
            env_u32("RHELMA_FILE_STORAGE__DATABASE_MAX_CONNECTIONS", 10)?;

        let max_file_size_bytes = env_u64(
            "RHELMA_FILE_STORAGE__MAX_FILE_SIZE_BYTES",
            1024 * 1024 * 1024,
        )?;

        let allowed_mime_prefixes = env_csv("RHELMA_FILE_STORAGE__ALLOWED_MIME_PREFIXES")
            .unwrap_or_else(|| {
                vec![
                    "image/".into(),
                    "video/".into(),
                    "application/pdf".into(),
                    "text/".into(),
                ]
            });

        let antivirus_enabled = env_bool("RHELMA_FILE_STORAGE__ANTIVIRUS_ENABLED", false)?;
        let encryption_at_rest = env_bool("RHELMA_FILE_STORAGE__ENCRYPTION_AT_REST", false)?;

        let retention_days = env_opt("RHELMA_FILE_STORAGE__RETENTION_DAYS")
            .map(|s| {
                s.parse::<u32>()
                    .map_err(|e| format!("RHELMA_FILE_STORAGE__RETENTION_DAYS invalid: {e}"))
            })
            .transpose()?;

        // Rate limiting defaults are intentionally conservative.
        // This is a *service-level* safety net; the edge/gateway should still enforce
        // the primary/global rate limits.
        let rate_limit_read_rpm = env_u32("RHELMA_FILE_STORAGE__RATE_LIMIT_READ_RPM", 600)?;
        let rate_limit_write_rpm = env_u32("RHELMA_FILE_STORAGE__RATE_LIMIT_WRITE_RPM", 60)?;
        let rate_limit_burst = env_u32("RHELMA_FILE_STORAGE__RATE_LIMIT_BURST", 30)?;

        let provider_raw = env_opt("RHELMA_FILE_STORAGE__PROVIDER")
            .unwrap_or_else(|| "local".to_string())
            .to_ascii_lowercase();

        let default_provider = match provider_raw.as_str() {
            "local" | "localfs" | "fs" => StorageProviderKind::LocalFs,
            "s3" => StorageProviderKind::S3,
            other => return Err(format!("RHELMA_FILE_STORAGE__PROVIDER invalid: {other}")),
        };

        let local_root =
            env_opt("RHELMA_FILE_STORAGE__LOCAL_ROOT").unwrap_or_else(|| "./data".to_string());

        let s3_endpoint = env_opt("RHELMA_FILE_STORAGE__S3_ENDPOINT");
        let s3_region = env_opt("RHELMA_FILE_STORAGE__S3_REGION");
        let s3_bucket = env_opt("RHELMA_FILE_STORAGE__S3_BUCKET");
        let s3_access_key = env_opt("RHELMA_FILE_STORAGE__S3_ACCESS_KEY").map(Secret::new);
        let s3_secret_key = env_opt("RHELMA_FILE_STORAGE__S3_SECRET_KEY").map(Secret::new);

        if default_provider == StorageProviderKind::S3
            && (s3_endpoint.is_none()
                || s3_bucket.is_none()
                || s3_access_key.is_none()
                || s3_secret_key.is_none())
        {
            return Err(
                "S3 provider selected but required settings are missing (S3_ENDPOINT, S3_BUCKET, S3_ACCESS_KEY, S3_SECRET_KEY)".into(),
            );
        }

        let cors_allowed_origins =
            env_csv("RHELMA_FILE_STORAGE__CORS_ALLOWED_ORIGINS").unwrap_or_default();

        Ok(Self {
            service_name,
            environment: central.environment,
            region: central.region,

            listen_addr,

            database_url,
            database_max_connections,

            max_file_size_bytes,
            allowed_mime_prefixes,
            antivirus_enabled,
            encryption_at_rest,
            retention_days,

            rate_limit_read_rpm,
            rate_limit_write_rpm,
            rate_limit_burst,

            default_provider,
            local_root,

            s3_endpoint,
            s3_region,
            s3_bucket,
            s3_access_key,
            s3_secret_key,

            cors_allowed_origins,
        })
    }
}

fn env_opt(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn env_u32(key: &str, default: u32) -> Result<u32, String> {
    match env_opt(key) {
        Some(raw) => raw
            .parse::<u32>()
            .map_err(|e| format!("{key} invalid: {e}")),
        None => Ok(default),
    }
}

fn env_u64(key: &str, default: u64) -> Result<u64, String> {
    match env_opt(key) {
        Some(raw) => raw
            .parse::<u64>()
            .map_err(|e| format!("{key} invalid: {e}")),
        None => Ok(default),
    }
}

fn env_bool(key: &str, default: bool) -> Result<bool, String> {
    match env_opt(key) {
        Some(raw) => {
            let raw = raw.to_ascii_lowercase();
            match raw.as_str() {
                "1" | "true" | "yes" | "y" | "on" => Ok(true),
                "0" | "false" | "no" | "n" | "off" => Ok(false),
                other => Err(format!("{key} invalid boolean: {other}")),
            }
        }
        None => Ok(default),
    }
}

fn env_csv(key: &str) -> Option<Vec<String>> {
    env_opt(key).map(|raw| {
        raw.split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    })
}
