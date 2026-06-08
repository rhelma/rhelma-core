#![forbid(unsafe_code)]

use redis::aio::ConnectionManager;
use std::sync::Arc;
use std::time::Duration;
use tracing::warn;

use crate::config::GatewayConfig;
use crate::error::GatewayError;
use crate::eventing::{build_event_bus, GatewayEventPublisher};
use crate::services::{BaseRepo, SearchService};

use crate::region_routing::RegionRoutingHandle;
use rhelma_auth::config::AuthConfig as RhelmaAuthConfig;
use rhelma_auth::AuthService;
use rhelma_core::RhelmaError;

use rhelma_event::EventBus;

#[derive(Clone)]
pub struct AppState {
    /// Field `config`.
    pub config: Arc<GatewayConfig>,

    /// Field `http`.
    pub http: reqwest::Client,

    // health.rs expects PgPool with .acquire()
    /// Field `database`.
    pub database: sqlx::PgPool,

    /// Field `redis`.
    pub redis: ConnectionManager,

    /// Field `base_repo`.
    pub base_repo: BaseRepo,

    // auth_extractor expects Extension<AuthService>
    /// Field `auth_service`.
    pub auth_service: AuthService,

    // routes/search.rs expects state.search_service
    /// Field `search_service`.
    pub search_service: SearchService,

    /// Optional multi-region router (health/latency snapshot).
    pub region_router: Option<Arc<RegionRoutingHandle>>,

    /// Best-effort event publisher (region health / failover) (Phase 3).
    pub event_publisher: Arc<GatewayEventPublisher>,
}

impl AppState {
    pub async fn new(cfg: GatewayConfig) -> Result<Self, GatewayError> {
        let config = Arc::new(cfg);

        // --- Event bus (Kafka or Noop) ---
        let bus: Arc<dyn EventBus> = build_event_bus(
            &config.service_name,
            &config.kafka_brokers,
            &config.kafka_topic_prefix,
        );
        let event_publisher = Arc::new(GatewayEventPublisher::new(
            config.publish_region_events,
            config.service_name.clone(),
            config.central.region.clone(),
            bus,
        ));

        // --- DB from core config ---
        let db_cfg = rhelma_db::DbConnectConfig::from_core(&config.core);

        let db = rhelma_db::DatabaseBuilder::new(db_cfg)
            .build()
            .await
            .map_err(|e| GatewayError::from(RhelmaError::Database(e.to_string())))?;

        let database = db.pool().clone();

        // -----------------------------------------------------------------
        // Best-effort auto-migrate in dev/test (safe default: OFF in prod)
        // -----------------------------------------------------------------
        let auto_migrate = std::env::var("RHELMA_DB__AUTO_MIGRATE")
            .ok()
            .map(|v| {
                let v = v.trim();
                !(v.is_empty() || v == "0" || v.eq_ignore_ascii_case("false"))
            })
            .unwrap_or_else(|| !config.is_prod());

        if auto_migrate {
            let strict = std::env::var("RHELMA_DB__AUTO_MIGRATE_STRICT")
                .ok()
                .map(|v| {
                    let v = v.trim();
                    !(v.is_empty() || v == "0" || v.eq_ignore_ascii_case("false"))
                })
                .unwrap_or(false);

            match rhelma_db::migrations::migrator().run(&database).await {
                Ok(()) => {}
                Err(e) if strict => {
                    return Err(GatewayError::from(RhelmaError::Database(format!(
                        "migrations failed: {e}"
                    ))));
                }
                Err(e) => {
                    warn!(error = %e, "api-gateway: migrations failed (continuing)");
                }
            }
        }

        // --- Redis connection manager ---
        let client = redis::Client::open(config.redis_url().to_string())
            .map_err(|e| GatewayError::from(RhelmaError::Dependency(format!("redis open: {e}"))))?;

        let redis = client.get_connection_manager().await.map_err(|e| {
            GatewayError::from(RhelmaError::Dependency(format!("redis manager: {e}")))
        })?;

        // --- HTTP client ---
        let http = reqwest::Client::builder()
            .timeout(config.timeouts.upstream)
            .pool_idle_timeout(Duration::from_secs(30))
            .tcp_keepalive(Duration::from_secs(30))
            .build()
            .map_err(|e| {
                GatewayError::from(RhelmaError::Dependency(format!("reqwest build: {e}")))
            })?;

        // --- BaseRepo ---
        let base_repo = BaseRepo::new(database.clone());

        // --- AuthService (rhelma-auth) ---
        let auth_cfg = RhelmaAuthConfig::from_env(
            &config.service_name,
            &config.central.environment,
            Some(config.redis_url().to_string()),
        )
        .map_err(|e| GatewayError::from(RhelmaError::Dependency(format!("auth cfg: {e}"))))?;

        let auth_service = AuthService::new(auth_cfg)
            .await
            .map_err(|e| GatewayError::from(RhelmaError::Dependency(format!("auth init: {e}"))))?;

        // --- Optional multi-region routing (health checks update rhelma-core MultiRegionRouter) ---
        let region_router = crate::region_routing::spawn_if_enabled(
            config.as_ref(),
            http.clone(),
            event_publisher.clone(),
        )?;

        // --- SearchService (wrapper expects cfg + http, optionally with multi-region router) ---
        let search_service = SearchService::new(
            config.clone(),
            http.clone(),
            region_router.clone(),
            event_publisher.clone(),
        );

        Ok(Self {
            config,
            http,
            database,
            redis,
            base_repo,
            auth_service,
            search_service,
            region_router,
            event_publisher,
        })
    }

    pub async fn redis_ready(&self) -> bool {
        let mut con = self.redis.clone();

        // ✅ Correct generic usage: only return type T
        let pong = redis::cmd("PING").query_async::<String>(&mut con).await;

        matches!(pong.as_deref(), Ok("PONG"))
    }
}
