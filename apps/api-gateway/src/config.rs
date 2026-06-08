#![forbid(unsafe_code)]

use secrecy::{ExposeSecret, Secret};
use std::env;
use std::fs;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::time::Duration;

use rhelma_config::{CentralEnv, CoreConfig};
use rhelma_core::RhelmaError;

use crate::error::GatewayError;

#[derive(Debug, Clone)]
pub struct CorsConfig {
    /// Field `allow_origins`.
    pub allow_origins: Vec<String>,
    /// Field `allow_credentials`.
    pub allow_credentials: bool,
}

#[derive(Debug, Clone)]
pub struct TimeoutsConfig {
    /// Field `global`.
    pub global: Duration,
    /// Field `upstream`.
    pub upstream: Duration,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ServiceEndpoints {
    /// Field `auth_service_url`.
    pub auth_service_url: String,
    /// Field `search_service_url`.
    pub search_service_url: String,
    /// Field `social_service_url`.
    pub social_service_url: String,
    /// Field `user_service_url`.
    pub user_service_url: String,
    /// Field `ai_service_url`.
    pub ai_service_url: String,
    /// Field `control_service_url`.
    pub control_service_url: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RegionRoutingConfig {
    /// Enables multi-region routing decisions (health checks + failover metadata).
    pub enabled: bool,
    /// JSON config for regions and failover (see deploy/rhelma6/k8s/multi-region/routing-config.yaml).
    pub config_json: String,
    /// Interval between health checks.
    pub health_interval: Duration,
    /// Per-request timeout for health checks.
    pub health_timeout: Duration,
    /// Health endpoint path appended to each region base URL (default: /healthz).
    pub health_path: String,

    /// Optional URL of an external region health aggregator.
    ///
    /// When set, api-gateway will **poll** the aggregator's `/v1/regions/health` endpoint
    /// instead of probing each region directly.
    pub aggregator_url: Option<String>,

    /// Optional event-driven input for region health/failover.
    ///
    /// When enabled (and built with feature `kafka-events`), api-gateway will subscribe to
    /// `obs.region_health` and `obs.region_failover` and update the router from those events.
    ///
    /// This is complementary to polling/direct health checks: whichever source reports first
    /// will update the shared router state.
    pub event_input_enabled: bool,

    /// Kafka consumer group id used for region routing event input.
    pub event_input_group_id: String,

    /// TTL for applying an upstream-specific failover override based on `obs.region_failover`.
    pub failover_override_ttl: Duration,

    /// Optional allowlist of upstream service names that may receive failover overrides from events.
    ///
    /// If set, any `obs.region_failover` for an upstream not in this list will be ignored.
    pub failover_override_upstream_allowlist: Option<Vec<String>>,

    /// Optional allowlist of **event source services** that may emit `obs.region_failover`
    /// events which are trusted to apply upstream failover overrides.
    ///
    /// If set, api-gateway will ignore failover events whose `envelope.source.service` is
    /// not in this list. This is a safety guard against untrusted/poisoned inputs.
    pub failover_override_event_source_allowlist: Option<Vec<String>>,

    /// Maximum TTL (cap) for failover overrides, regardless of `failover_override_ttl`.
    ///
    /// This is a safety guard against unbounded overrides from untrusted inputs.
    pub failover_override_max_ttl: Duration,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    /// Canonical deployment env (region + environment + version + optional tenant).
    /// This is the single source of truth for env/region across Rhelma services.
    pub central: CentralEnv,

    /// Core configuration derived from CentralEnv (DB/Redis/etc).
    pub core: CoreConfig,

    /// Field `service_name`.
    pub service_name: String,

    /// Field `bind_host`.
    pub bind_host: String,
    /// Field `bind_port`.
    pub bind_port: u16,

    /// Field `cors`.
    pub cors: CorsConfig,
    /// Field `timeouts`.
    pub timeouts: TimeoutsConfig,
    /// Field `services`.
    pub services: ServiceEndpoints,

    /// TTL for discovery results cached in Redis.
    pub discovery_cache_ttl: Duration,

    /// Optional multi-region routing config (Phase 2).
    pub region_routing: Option<RegionRoutingConfig>,

    /// Kafka brokers for event publishing ("noop" disables).
    pub kafka_brokers: String,
    /// Kafka topic prefix (e.g. "rhelma6").
    pub kafka_topic_prefix: String,
    /// Whether to publish region health/failover events.
    pub publish_region_events: bool,

    /// Field `redis_url`.
    pub redis_url: Secret<String>,

    // Rate limiting (application layer)
    // Fixed-window in Redis (baseline); "burst" is added to max for the window.
    /// Field `rate_limit_requests_per_minute`.
    pub rate_limit_requests_per_minute: u32,
    /// Field `rate_limit_burst`.
    pub rate_limit_burst: u32,
}

impl GatewayConfig {
    pub fn load() -> Result<Self, GatewayError> {
        Self::from_env()
    }

    pub fn redis_url(&self) -> &str {
        self.redis_url.expose_secret()
    }

    pub fn env_name(&self) -> &str {
        self.central.environment.as_str()
    }

    pub fn is_prod(&self) -> bool {
        self.central.environment.eq_ignore_ascii_case("production")
    }

    pub fn from_env() -> Result<Self, GatewayError> {
        // CentralEnv is strict & contract-aligned:
        // - RHELMA_ENV or RHELMA_ENVIRONMENT (required)
        // - RHELMA_REGION (required)
        // - RHELMA_SERVICE_VERSION (required)
        let central = CentralEnv::from_env_strict()
            .map_err(|e| GatewayError::from(RhelmaError::Config(e.to_string())))?;
        let core = CoreConfig::from_env(&central)
            .map_err(|e| GatewayError::from(RhelmaError::Config(e.to_string())))?;

        // Helper: prefer unified CoreConfig, then gateway override env, then default.
        let core_redis = core.redis_url.as_ref().map(|s| s.expose_secret().clone());

        let service_name = env::var("RHELMA_SERVICE_NAME").unwrap_or_else(|_| "api-gateway".into());

        let bind_host = env::var("RHELMA_BIND_HOST").unwrap_or_else(|_| "0.0.0.0".into());
        let bind_port = env::var("RHELMA_BIND_PORT")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(8080);

        let global = env::var("RHELMA_GATEWAY_TIMEOUT_GLOBAL")
            .ok()
            .and_then(|v| humantime::parse_duration(&v).ok())
            .unwrap_or(Duration::from_secs(10));

        let upstream = env::var("RHELMA_GATEWAY_TIMEOUT_UPSTREAM")
            .ok()
            .and_then(|v| humantime::parse_duration(&v).ok())
            .unwrap_or(Duration::from_secs(5));

        let allow_origins = env::var("RHELMA_GATEWAY_CORS_ALLOWED_ORIGINS")
            .ok()
            .map(|s| {
                s.split(',')
                    .map(|x| x.trim().to_string())
                    .filter(|x| !x.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| vec!["*".to_string()]);

        let allow_credentials = env::var("RHELMA_GATEWAY_CORS_ALLOW_CREDENTIALS")
            .ok()
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(false);

        // ✅ Fail-closed CORS validation (especially in production)
        let allow_any = allow_origins.iter().any(|o| o == "*");
        if allow_credentials && allow_any {
            return Err(GatewayError::from(RhelmaError::Config(
                "CORS misconfiguration: allow_credentials=true is not allowed when allow_origins contains '*'"
                    .into(),
            )));
        }
        if central.environment.eq_ignore_ascii_case("production") && allow_any {
            return Err(GatewayError::from(RhelmaError::Config(
                "CORS misconfiguration: wildcard '*' is not allowed in production".into(),
            )));
        }

        // Rate limit knobs
        // Prefer explicit gateway vars; keep backwards-compat with older env naming.
        let rate_limit_requests_per_minute =
            env::var("RHELMA_GATEWAY_RATE_LIMIT_REQUESTS_PER_MINUTE")
                .or_else(|_| env::var("RATE_LIMIT_REQUESTS_PER_MINUTE"))
                .ok()
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(1000);

        let rate_limit_burst = env::var("RHELMA_GATEWAY_RATE_LIMIT_BURST")
            .or_else(|_| env::var("RATE_LIMIT_BURST"))
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(100);

        // Prefer RHELMA_REDIS__URL (core-style), then RHELMA_REDIS_URL (gateway legacy),
        // then CoreConfig.redis_url, then localhost default.
        let redis_url_raw = env::var("RHELMA_REDIS__URL")
            .or_else(|_| env::var("RHELMA_REDIS_URL"))
            .ok()
            .or(core_redis)
            .unwrap_or_else(|| "redis://127.0.0.1/".into());
        let redis_url = Secret::new(redis_url_raw);

        let services = ServiceEndpoints {
            auth_service_url: env::var("RHELMA_AUTH_SERVICE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8081".into()),
            search_service_url: env::var("RHELMA_SEARCH_SERVICE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8082".into()),
            social_service_url: env::var("RHELMA_SOCIAL_SERVICE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8085".into()),
            user_service_url: env::var("RHELMA_USER_SERVICE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8083".into()),
            ai_service_url: env::var("RHELMA_AI_SERVICE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8084".into()),
            control_service_url: env::var("RHELMA_CONTROL_SERVICE_URL").ok(),
        };

        let discovery_cache_ttl = env::var("RHELMA_GATEWAY_DISCOVERY_CACHE_TTL_SECONDS")
            .or_else(|_| env::var("RHELMA_API_GATEWAY__DISCOVERY_CACHE_TTL_SECONDS"))
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or_else(|| Duration::from_secs(30));

        // -----------------------------------------------------------------
        // Event publishing (Kafka adapter behind feature flag)
        // -----------------------------------------------------------------
        let kafka_brokers = env::var("RHELMA_GATEWAY_KAFKA_BROKERS")
            .or_else(|_| env::var("RHELMA_API_GATEWAY__KAFKA_BROKERS"))
            .unwrap_or_else(|_| "noop".to_string());

        let kafka_topic_prefix = env::var("RHELMA_GATEWAY_KAFKA_TOPIC_PREFIX")
            .or_else(|_| env::var("RHELMA_API_GATEWAY__KAFKA_TOPIC_PREFIX"))
            .unwrap_or_else(|_| "rhelma6".to_string());

        let publish_region_events = env::var("RHELMA_GATEWAY_PUBLISH_REGION_EVENTS")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        // -----------------------------------------------------------------
        // Multi-region routing (Phase 2) - optional
        // -----------------------------------------------------------------
        let region_routing_enabled = env::var("RHELMA_GATEWAY_REGION_ROUTING_ENABLED")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let region_routing = if region_routing_enabled {
            let config_json = if let Ok(path) =
                env::var("RHELMA_GATEWAY_REGION_ROUTING_CONFIG_PATH")
            {
                fs::read_to_string(path.trim()).map_err(|e| {
                    GatewayError::from(RhelmaError::Config(format!(
                        "failed to read RHELMA_GATEWAY_REGION_ROUTING_CONFIG_PATH file: {e}"
                    )))
                })?
            } else {
                env::var("RHELMA_GATEWAY_REGION_ROUTING_CONFIG_JSON").map_err(|_| {
                    GatewayError::from(RhelmaError::Config(
                        "RHELMA_GATEWAY_REGION_ROUTING_CONFIG_JSON (or RHELMA_GATEWAY_REGION_ROUTING_CONFIG_PATH) is required when RHELMA_GATEWAY_REGION_ROUTING_ENABLED=1"
                            .into(),
                    ))
                })?
            };

            let health_interval = env::var("RHELMA_GATEWAY_REGION_ROUTING_HEALTH_INTERVAL")
                .ok()
                .and_then(|v| humantime::parse_duration(&v).ok())
                .unwrap_or(Duration::from_secs(15));

            let health_timeout = env::var("RHELMA_GATEWAY_REGION_ROUTING_HEALTH_TIMEOUT")
                .ok()
                .and_then(|v| humantime::parse_duration(&v).ok())
                .unwrap_or(Duration::from_secs(2));

            let health_path = env::var("RHELMA_GATEWAY_REGION_ROUTING_HEALTH_PATH")
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "/healthz".to_string());

            let aggregator_url = env::var("RHELMA_GATEWAY_REGION_ROUTING_AGGREGATOR_URL")
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty());

            let event_input_enabled = env::var("RHELMA_GATEWAY_REGION_ROUTING_EVENT_INPUT_ENABLED")
                .ok()
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);

            let event_input_group_id =
                env::var("RHELMA_GATEWAY_REGION_ROUTING_EVENT_INPUT_GROUP_ID")
                    .ok()
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty())
                    .unwrap_or_else(|| format!("{}-region-routing", service_name));

            let failover_override_ttl =
                env::var("RHELMA_GATEWAY_REGION_ROUTING_FAILOVER_OVERRIDE_TTL")
                    .ok()
                    .and_then(|v| humantime::parse_duration(&v).ok())
                    .unwrap_or(Duration::from_secs(120));

            let failover_override_upstream_allowlist =
                env::var("RHELMA_GATEWAY_REGION_ROUTING_FAILOVER_OVERRIDE_UPSTREAM_ALLOWLIST")
                    .ok()
                    .map(|v| {
                        v.split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect::<Vec<_>>()
                    })
                    .filter(|v| !v.is_empty());

            let failover_override_event_source_allowlist =
                env::var("RHELMA_GATEWAY_REGION_ROUTING_FAILOVER_EVENT_SOURCE_ALLOWLIST")
                    .ok()
                    .map(|v| {
                        v.split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect::<Vec<_>>()
                    })
                    .filter(|v| !v.is_empty());

            let failover_override_max_ttl =
                env::var("RHELMA_GATEWAY_REGION_ROUTING_FAILOVER_OVERRIDE_MAX_TTL")
                    .ok()
                    .and_then(|v| humantime::parse_duration(&v).ok())
                    .unwrap_or(Duration::from_secs(600));

            Some(RegionRoutingConfig {
                enabled: true,
                config_json,
                health_interval,
                health_timeout,
                health_path,
                aggregator_url,
                event_input_enabled,
                event_input_group_id,
                failover_override_ttl,
                failover_override_upstream_allowlist,
                failover_override_event_source_allowlist,
                failover_override_max_ttl,
            })
        } else {
            None
        };

        Ok(Self {
            central,
            core,
            service_name,
            bind_host,
            bind_port,
            cors: CorsConfig {
                allow_origins,
                allow_credentials,
            },
            timeouts: TimeoutsConfig { global, upstream },
            services,
            discovery_cache_ttl,
            region_routing,

            kafka_brokers,
            kafka_topic_prefix,
            publish_region_events,

            redis_url,
            rate_limit_requests_per_minute,
            rate_limit_burst,
        })
    }

    pub fn bind_addr(&self) -> Result<SocketAddr, GatewayError> {
        let ip = IpAddr::from_str(self.bind_host.trim()).map_err(|e| {
            GatewayError::from(RhelmaError::Config(format!("invalid bind_host: {e}")))
        })?;
        Ok(SocketAddr::from((ip, self.bind_port)))
    }
}
