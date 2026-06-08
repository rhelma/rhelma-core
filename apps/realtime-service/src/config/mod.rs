#![forbid(unsafe_code)]

use serde::Deserialize;
use std::env;
use std::time::Duration;
use thiserror::Error;

use rhelma_config::CentralEnv;

#[derive(Debug, Clone, Deserialize)]
pub struct RealtimeConfig {
    /// Field `service_name`.
    pub service_name: String,
    /// Field `environment`.
    pub environment: String,
    /// Field `region`.
    pub region: String,

    /// Example: "0.0.0.0:9000"
    pub listen_addr: String,

    /// Zero-trust: in prod SHOULD be false
    pub allow_anonymous: bool,

    /// WebSocket limits/keepalive
    pub ws_max_message_bytes: usize,
    /// Field `ws_ping_interval`.
    pub ws_ping_interval: Duration,
    /// Field `ws_pong_timeout`.
    pub ws_pong_timeout: Duration,

    /// Per-connection rate limit (token bucket)
    pub ws_msgs_per_sec: u32,
    /// Field `ws_msg_burst`.
    pub ws_msg_burst: u32,

    /// Hard limits (guards)
    pub max_connections_per_user: u32,
    /// Field `max_rooms_per_connection`.
    pub max_rooms_per_connection: u32,

    /// Optional override for rhelma-auth redis url (service-scoped)
    pub auth_redis_url_override: Option<String>,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing environment variable: {0}")]
    /// Variant `MissingEnv`.
    MissingEnv(&'static str),
    #[error("invalid configuration: {0}")]
    /// Variant `Invalid`.
    Invalid(String),
}

impl RealtimeConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let central = CentralEnv::from_env_strict()
            .map_err(|e| ConfigError::Invalid(format!("central env: {e}")))?;

        let service_name = env::var("RHELMA_SERVICE_NAME")
            .or_else(|_| env::var("RHELMA_RT_SERVICE_NAME"))
            .unwrap_or_else(|_| "realtime-service".into());

        let environment = central.environment.clone();
        let region = central.region.clone();

        let listen_addr = env_var("RHELMA_RT_LISTEN_ADDR").unwrap_or_else(|| "0.0.0.0:9000".into());

        // Safe default: allow anonymous ONLY in development unless explicitly enabled
        let allow_anonymous = match env_var("REALTIME_ALLOW_ANONYMOUS") {
            Some(v) => parse_bool_loose(&v).unwrap_or(false),
            None => environment == "development",
        };

        Ok(Self {
            service_name,
            environment,
            region,
            listen_addr,

            allow_anonymous,

            ws_max_message_bytes: env_usize("REALTIME_WS_MAX_MESSAGE_BYTES", 256 * 1024),
            ws_ping_interval: Duration::from_secs(env_u64("REALTIME_WS_PING_INTERVAL_SECS", 30)),
            ws_pong_timeout: Duration::from_secs(env_u64("REALTIME_WS_PONG_TIMEOUT_SECS", 10)),

            ws_msgs_per_sec: env_u32("REALTIME_WS_MSGS_PER_SEC", 30),
            ws_msg_burst: env_u32("REALTIME_WS_MSG_BURST", 60),

            max_connections_per_user: env_u32("REALTIME_MAX_CONNECTIONS_PER_USER", 10),
            max_rooms_per_connection: env_u32("REALTIME_MAX_ROOMS_PER_CONN", 10),

            auth_redis_url_override: env_var("REALTIME_AUTH_REDIS_URL_OVERRIDE"),
        })
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.service_name.trim().is_empty() {
            return Err(ConfigError::Invalid(
                "service_name must not be empty".into(),
            ));
        }
        if self.region.trim().is_empty() {
            return Err(ConfigError::Invalid("region must not be empty".into()));
        }
        if self.ws_max_message_bytes == 0 {
            return Err(ConfigError::Invalid(
                "ws_max_message_bytes must be > 0".into(),
            ));
        }
        if self.ws_ping_interval.is_zero() {
            return Err(ConfigError::Invalid("ws_ping_interval must be > 0".into()));
        }
        if self.ws_pong_timeout.is_zero() {
            return Err(ConfigError::Invalid("ws_pong_timeout must be > 0".into()));
        }
        if self.ws_msgs_per_sec == 0 {
            return Err(ConfigError::Invalid("ws_msgs_per_sec must be > 0".into()));
        }
        if self.ws_msg_burst == 0 {
            return Err(ConfigError::Invalid("ws_msg_burst must be > 0".into()));
        }
        if self.max_connections_per_user == 0 {
            return Err(ConfigError::Invalid(
                "max_connections_per_user must be > 0".into(),
            ));
        }
        if self.max_rooms_per_connection == 0 {
            return Err(ConfigError::Invalid(
                "max_rooms_per_connection must be > 0".into(),
            ));
        }
        Ok(())
    }
}

fn env_var(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn env_u64(key: &str, default: u64) -> u64 {
    env_var(key)
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_u32(key: &str, default: u32) -> u32 {
    env_var(key)
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(default)
}

fn env_usize(key: &str, default: usize) -> usize {
    env_var(key)
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(default)
}

fn parse_bool_loose(v: &str) -> Option<bool> {
    match v.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "y" | "on" => Some(true),
        "0" | "false" | "no" | "n" | "off" => Some(false),
        _ => None,
    }
}

impl RealtimeConfig {
    /// A safe default config for tests and local dev sandboxes.
    ///
    /// Notes:
    /// - `allow_anonymous=true` allows running without a working rhelma-auth env.
    /// - `listen_addr` is typically overridden by binding to `127.0.0.1:0` in tests.
    pub fn for_tests() -> Self {
        Self {
            service_name: "realtime-service".to_string(),
            environment: "test".to_string(),
            region: "test".to_string(),
            listen_addr: "127.0.0.1:0".to_string(),
            allow_anonymous: true,

            ws_max_message_bytes: 64 * 1024,
            ws_ping_interval: Duration::from_secs(15),
            ws_pong_timeout: Duration::from_secs(45),

            ws_msgs_per_sec: 25,
            ws_msg_burst: 50,

            max_connections_per_user: 4,
            max_rooms_per_connection: 50,

            auth_redis_url_override: None,
        }
    }
}
