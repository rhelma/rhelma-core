//! rhelma-auth configuration (Rhelma v5.2 aligned).
//!
//! Security rules enforced here:
//! - Redis is mandatory (distributed sessions + rate limit).
//! - Access token TTL <= 15 minutes.
//! - Refresh token TTL <= 7 days.
//! - Session idle timeout <= 30 minutes.
//! - Session absolute timeout <= 8 hours.
//! - Secrets are never hardcoded. Keys must be provided via env/secret manager.

use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use std::time::Duration;

use crate::error::{AuthError, AuthResult};

#[derive(Debug, Clone, Deserialize)]
/// struct (documented for contract compliance).
pub struct AuthConfig {
    /// JWT issuer (e.g. "asrnegar-auth").
    pub issuer: String,

    /// JWT audience (e.g. "asrnegar-api").
    pub audience: String,

    /// Ed25519 private key (base64 DER).
    ///
    /// Store in secret manager/env var; never commit.
    pub jwt_private_key_b64: SecretString,

    /// Ed25519 public key (base64 DER).
    pub jwt_public_key_b64: SecretString,

    /// Access token TTL (seconds) — MUST be <= 900.
    #[serde(default = "default_access_ttl_secs")]
    pub access_token_ttl_secs: u64,

    /// Refresh token TTL (seconds) — MUST be <= 604800.
    #[serde(default = "default_refresh_ttl_secs")]
    pub refresh_token_ttl_secs: u64,

    /// Session idle timeout (seconds) — MUST be <= 1800.
    #[serde(default = "default_session_idle_secs")]
    pub session_idle_timeout_secs: u64,

    /// Minimum interval (seconds) between session "touch" writes.
    ///
    /// When access tokens are verified, rhelma-auth may refresh `last_seen_at` to enforce
    /// idle timeout in a sliding-window fashion. To avoid excessive Redis writes, the
    /// session will only be updated if the previous touch is older than this interval.
    ///
    /// - MUST be > 0
    /// - SHOULD be <= 300
    #[serde(default = "default_session_touch_secs")]
    pub session_touch_interval_secs: u64,

    /// Session absolute timeout (seconds) — MUST be <= 28800.
    #[serde(default = "default_session_abs_secs")]
    pub session_absolute_timeout_secs: u64,

    /// Redis URL (mandatory).
    pub redis_url: SecretString,

    /// Redis key prefix (namespacing).
    #[serde(default = "default_redis_prefix")]
    pub redis_prefix: String,

    /// Cookie security flags (for gateways that use cookies).
    #[serde(default = "default_cookie_secure")]
    pub cookie_secure: bool,

    #[serde(default = "default_cookie_same_site")]
    /// Field `cookie_same_site`.
    pub cookie_same_site: String,

    /// Password hashing cost/policy knobs.
    #[serde(default = "default_password_hash_cost")]
    pub password_hash_cost: u32,

    /// Rate limit baseline (requests per window).
    #[serde(default = "default_rate_limit_requests")]
    pub rate_limit_requests: u32,

    /// Rate limit window seconds.
    #[serde(default = "default_rate_limit_window_secs")]
    pub rate_limit_window_secs: u64,
}

impl AuthConfig {
    /// fn (documented for contract compliance).
    pub fn access_ttl(&self) -> Duration {
        Duration::from_secs(self.access_token_ttl_secs)
    }
    /// fn (documented for contract compliance).
    pub fn refresh_ttl(&self) -> Duration {
        Duration::from_secs(self.refresh_token_ttl_secs)
    }
    /// fn (documented for contract compliance).
    pub fn session_idle_ttl(&self) -> Duration {
        Duration::from_secs(self.session_idle_timeout_secs)
    }
    /// fn (documented for contract compliance).
    pub fn session_touch_interval(&self) -> Duration {
        Duration::from_secs(self.session_touch_interval_secs)
    }
    /// fn (documented for contract compliance).
    pub fn session_absolute_ttl(&self) -> Duration {
        Duration::from_secs(self.session_absolute_timeout_secs)
    }
    /// fn (documented for contract compliance).
    pub fn rate_limit_window(&self) -> Duration {
        Duration::from_secs(self.rate_limit_window_secs)
    }

    /// Validate config against Rhelma v5.2 security contract.
    pub fn validate(&self) -> AuthResult<()> {
        if self.issuer.trim().is_empty() {
            return Err(AuthError::Config {
                code: "missing_issuer",
            });
        }
        if self.audience.trim().is_empty() {
            return Err(AuthError::Config {
                code: "missing_audience",
            });
        }
        if self.redis_prefix.trim().is_empty() {
            return Err(AuthError::Config {
                code: "missing_redis_prefix",
            });
        }

        // Hard limits (security contract)
        if self.access_token_ttl_secs == 0 || self.access_token_ttl_secs > 15 * 60 {
            return Err(AuthError::Config {
                code: "access_token_ttl_invalid",
            });
        }
        if self.refresh_token_ttl_secs == 0 || self.refresh_token_ttl_secs > 7 * 24 * 60 * 60 {
            return Err(AuthError::Config {
                code: "refresh_token_ttl_invalid",
            });
        }
        if self.session_idle_timeout_secs == 0 || self.session_idle_timeout_secs > 30 * 60 {
            return Err(AuthError::Config {
                code: "session_idle_ttl_invalid",
            });
        }

        if self.session_touch_interval_secs == 0
            || self.session_touch_interval_secs > 5 * 60
            || self.session_touch_interval_secs > self.session_idle_timeout_secs
        {
            return Err(AuthError::Config {
                code: "session_touch_interval_invalid",
            });
        }
        if self.session_absolute_timeout_secs == 0
            || self.session_absolute_timeout_secs > 8 * 60 * 60
        {
            return Err(AuthError::Config {
                code: "session_absolute_ttl_invalid",
            });
        }

        // Key material must exist (do not validate content here; keys loader will).
        if self.jwt_private_key_b64.expose_secret().trim().is_empty() {
            return Err(AuthError::Config {
                code: "missing_jwt_private_key",
            });
        }
        if self.jwt_public_key_b64.expose_secret().trim().is_empty() {
            return Err(AuthError::Config {
                code: "missing_jwt_public_key",
            });
        }
        if self.redis_url.expose_secret().trim().is_empty() {
            return Err(AuthError::Config {
                code: "missing_redis_url",
            });
        }

        Ok(())
    }
}

fn default_access_ttl_secs() -> u64 {
    15 * 60
}
fn default_refresh_ttl_secs() -> u64 {
    7 * 24 * 60 * 60
}
fn default_session_idle_secs() -> u64 {
    30 * 60
}
fn default_session_touch_secs() -> u64 {
    60
}
fn default_session_abs_secs() -> u64 {
    8 * 60 * 60
}

fn default_cookie_secure() -> bool {
    true
}
fn default_cookie_same_site() -> String {
    "Lax".to_string()
}

fn default_password_hash_cost() -> u32 {
    12
}
fn default_rate_limit_requests() -> u32 {
    60
}
fn default_rate_limit_window_secs() -> u64 {
    60
}

fn default_redis_prefix() -> String {
    "rhelma:auth".to_string()
}

impl AuthConfig {
    /// Load from env with Rhelma v5.2 safe defaults.
    /// - Keys & redis_url are mandatory (unless redis_url provided explicitly).
    /// - All limits are validated via `validate()`.
    pub fn from_env(
        service_name: &str,
        environment: &str,
        redis_url_override: Option<String>,
    ) -> AuthResult<Self> {
        fn get(name: &str) -> Option<String> {
            std::env::var(name)
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        }
        fn parse_u64(name: &str) -> AuthResult<Option<u64>> {
            match get(name) {
                None => Ok(None),
                Some(v) => v.parse::<u64>().map(Some).map_err(|_| AuthError::Config {
                    code: "invalid_number",
                }),
            }
        }
        fn parse_u32(name: &str) -> AuthResult<Option<u32>> {
            match get(name) {
                None => Ok(None),
                Some(v) => v.parse::<u32>().map(Some).map_err(|_| AuthError::Config {
                    code: "invalid_number",
                }),
            }
        }
        fn parse_bool(name: &str) -> AuthResult<Option<bool>> {
            match get(name) {
                None => Ok(None),
                Some(v) => v.parse::<bool>().map(Some).map_err(|_| AuthError::Config {
                    code: "invalid_bool",
                }),
            }
        }

        let issuer = get("RHELMA_AUTH_ISSUER").unwrap_or_else(|| format!("{service_name}-auth"));

        let audience = get("RHELMA_AUTH_AUDIENCE").unwrap_or_else(|| service_name.to_string());

        let jwt_private_key_b64 =
            get("RHELMA_AUTH_JWT_PRIVATE_KEY_B64").ok_or(AuthError::Config {
                code: "missing_jwt_private_key",
            })?;

        let jwt_public_key_b64 =
            get("RHELMA_AUTH_JWT_PUBLIC_KEY_B64").ok_or(AuthError::Config {
                code: "missing_jwt_public_key",
            })?;

        // redis_url: env wins, else explicit override, else error
        let redis_url =
            get("RHELMA_AUTH_REDIS_URL")
                .or(redis_url_override)
                .ok_or(AuthError::Config {
                    code: "missing_redis_url",
                })?;

        let access_token_ttl_secs =
            parse_u64("RHELMA_AUTH_ACCESS_TTL_SECS")?.unwrap_or_else(default_access_ttl_secs);

        let refresh_token_ttl_secs =
            parse_u64("RHELMA_AUTH_REFRESH_TTL_SECS")?.unwrap_or_else(default_refresh_ttl_secs);

        let session_idle_timeout_secs =
            parse_u64("RHELMA_AUTH_SESSION_IDLE_SECS")?.unwrap_or_else(default_session_idle_secs);

        let session_touch_interval_secs =
            parse_u64("RHELMA_AUTH_SESSION_TOUCH_SECS")?.unwrap_or_else(default_session_touch_secs);

        let session_absolute_timeout_secs =
            parse_u64("RHELMA_AUTH_SESSION_ABS_SECS")?.unwrap_or_else(default_session_abs_secs);

        let redis_prefix = get("RHELMA_AUTH_REDIS_PREFIX")
            .unwrap_or_else(|| format!("{service_name}:{environment}:auth"));

        // sensible default: in dev -> false, else true (unless env provided)
        let cookie_secure =
            parse_bool("RHELMA_AUTH_COOKIE_SECURE")?.unwrap_or(environment != "development");

        let cookie_same_site =
            get("RHELMA_AUTH_COOKIE_SAME_SITE").unwrap_or_else(default_cookie_same_site);

        let password_hash_cost =
            parse_u32("RHELMA_AUTH_PASSWORD_HASH_COST")?.unwrap_or_else(default_password_hash_cost);

        let rate_limit_requests = parse_u32("RHELMA_AUTH_RATE_LIMIT_REQUESTS")?
            .unwrap_or_else(default_rate_limit_requests);

        let rate_limit_window_secs = parse_u64("RHELMA_AUTH_RATE_LIMIT_WINDOW_SECS")?
            .unwrap_or_else(default_rate_limit_window_secs);

        let cfg = Self {
            issuer,
            audience,
            jwt_private_key_b64: SecretString::new(jwt_private_key_b64),
            jwt_public_key_b64: SecretString::new(jwt_public_key_b64),
            access_token_ttl_secs,
            refresh_token_ttl_secs,
            session_idle_timeout_secs,
            session_touch_interval_secs,
            session_absolute_timeout_secs,
            redis_url: SecretString::new(redis_url),
            redis_prefix,
            cookie_secure,
            cookie_same_site,
            password_hash_cost,
            rate_limit_requests,
            rate_limit_window_secs,
        };

        cfg.validate()?;
        Ok(cfg)
    }
}
