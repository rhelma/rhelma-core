#![forbid(unsafe_code)]

use redis::aio::ConnectionManager;
use std::sync::Arc;
use tracing::warn;

use rhelma_auth::config::AuthConfig as RhelmaAuthConfig;
use rhelma_auth::AuthService;
use rhelma_core::RhelmaError;

use crate::config::SocialConfig;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<SocialConfig>,
    pub database: sqlx::PgPool,
    pub redis: ConnectionManager,
    pub auth_service: AuthService,
}

impl AppState {
    pub async fn initialize(cfg: SocialConfig) -> anyhow::Result<Self> {
        let config = Arc::new(cfg);

        // --- DB from core config ---
        let db_cfg = rhelma_db::DbConnectConfig::from_core(&config.core);
        let db = rhelma_db::DatabaseBuilder::new(db_cfg)
            .pool_name("social")
            .build()
            .await
            .map_err(|e| RhelmaError::Database(e.to_string()))?;

        let database = db.pool().clone();

        // Best-effort auto-migrate in dev/test (safe default: OFF in prod)
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
                    return Err(anyhow::anyhow!("migrations failed: {e}"));
                }
                Err(e) => {
                    warn!(error = %e, "social-service: migrations failed (continuing)");
                }
            }
        }

        // --- Redis connection manager ---
        let client = redis::Client::open(config.redis_url().to_string())
            .map_err(|e| RhelmaError::Dependency(format!("redis open: {e}")))?;

        let redis = client
            .get_connection_manager()
            .await
            .map_err(|e| RhelmaError::Dependency(format!("redis manager: {e}")))?;

        // --- AuthService (rhelma-auth) ---
        let auth_cfg = RhelmaAuthConfig::from_env(
            &config.service_name,
            &config.central.environment,
            Some(config.redis_url().to_string()),
        )
        .map_err(|e| RhelmaError::Dependency(format!("auth cfg: {e}")))?;

        let auth_service = AuthService::new(auth_cfg)
            .await
            .map_err(|e| RhelmaError::Dependency(format!("auth init: {e}")))?;

        Ok(Self {
            config,
            database,
            redis,
            auth_service,
        })
    }
}
