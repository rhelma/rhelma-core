#![forbid(unsafe_code)]

use std::sync::Arc;

use sqlx::postgres::PgPoolOptions;

use crate::config::FileStorageConfig;
use crate::repository::{audit_repository::PgAuditRepository, file_repository::PgFileRepository};
use crate::services::storage_backend::StorageBackendManager;

#[derive(Clone)]
pub struct AppState {
    /// Field `pool`.
    pub pool: sqlx::PgPool,
    /// Field `file_repo`.
    pub file_repo: PgFileRepository,
    /// Field `audit_repo`.
    pub audit_repo: PgAuditRepository,
    /// Field `storage_backend`.
    pub storage_backend: StorageBackendManager,
}

impl AppState {
    pub async fn new(cfg: Arc<FileStorageConfig>) -> Result<Self, Box<dyn std::error::Error>> {
        let pool = PgPoolOptions::new()
            .max_connections(cfg.database_max_connections)
            .connect(&cfg.database_url)
            .await?;

        // Keep DB schema in sync at startup.
        // This fails fast in production when migrations are missing or invalid.
        sqlx::migrate!("./migrations").run(&pool).await?;

        let file_repo = PgFileRepository::new(pool.clone());
        let audit_repo = PgAuditRepository::new(pool.clone());
        let storage_backend = StorageBackendManager::from_config(&cfg).await?;

        Ok(Self {
            pool,
            file_repo,
            audit_repo,
            storage_backend,
        })
    }
}
