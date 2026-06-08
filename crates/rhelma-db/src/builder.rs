use crate::types::RegionId;
use crate::{Database, DbError, DbResult};
use sqlx::postgres::PgConnectOptions;
use std::str::FromStr;
use tokio::time::timeout;

use rhelma_config::CoreConfig;
use secrecy::ExposeSecret;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct DbConnectConfig {
    /// Field `url`.
    pub url: String,
    /// Field `max_connections`.
    pub max_connections: u32,
    /// Field `min_connections`.
    pub min_connections: u32,

    /// Field `connect_timeout`.
    pub connect_timeout: Duration,
    /// Field `acquire_timeout`.
    pub acquire_timeout: Duration,
    /// Field `idle_timeout`.
    pub idle_timeout: Duration,
    /// Field `max_lifetime`.
    pub max_lifetime: Duration,

    /// Field `statement_timeout_ms`.
    pub statement_timeout_ms: Option<u64>,
    /// Field `application_name`.
    pub application_name: Option<String>,
}

impl DbConnectConfig {
    pub fn from_core(core: &CoreConfig) -> Self {
        Self {
            url: core.db_url.expose_secret().clone(),
            max_connections: core.db_max_connections.unwrap_or(10),
            min_connections: core.db_min_connections.unwrap_or(0),

            connect_timeout: Duration::from_secs(5),
            acquire_timeout: Duration::from_secs(5),
            idle_timeout: Duration::from_secs(300),
            max_lifetime: Duration::from_secs(1800),

            statement_timeout_ms: Some(10_000), // پیش‌فرض امن
            application_name: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DatabaseBuilder {
    cfg: DbConnectConfig,
    pool_name: &'static str,
    region: Option<RegionId>,
    enforce_residency: bool,
}

impl DatabaseBuilder {
    pub fn new(cfg: DbConnectConfig) -> Self {
        Self {
            cfg,
            pool_name: "main",
            region: None,
            enforce_residency: false,
        }
    }

    pub fn pool_name(mut self, name: &'static str) -> Self {
        self.pool_name = name;
        self
    }

    pub fn region(mut self, region: RegionId, enforce: bool) -> Self {
        self.region = Some(region);
        self.enforce_residency = enforce;
        self
    }

    pub async fn build(self) -> DbResult<Database> {
        let mut opts = PgPoolOptions::new();

        opts = opts
            .max_connections(self.cfg.max_connections)
            .min_connections(self.cfg.min_connections)
            .acquire_timeout(self.cfg.acquire_timeout)
            .idle_timeout(self.cfg.idle_timeout)
            .max_lifetime(self.cfg.max_lifetime);

        let conn_opts =
            PgConnectOptions::from_str(&self.cfg.url).map_err(|_| DbError::Connection {
                code: Some("invalid_db_url".into()),
            })?;

        let pool = timeout(self.cfg.connect_timeout, opts.connect_with(conn_opts))
            .await
            .map_err(|_| DbError::Connection {
                code: Some("connect_timeout".into()),
            })?
            .map_err(DbError::from_sqlx)?;

        let mut db = Database::new(pool).with_pool_name(self.pool_name);

        if let Some(r) = self.region {
            db = db.with_region(r, self.enforce_residency);
        }

        Ok(db)
    }
}
