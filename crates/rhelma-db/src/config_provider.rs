use async_trait::async_trait;
use serde_json::Value;
use sqlx::PgPool;

use rhelma_config::errors::{ConfigError, ConfigResult};
use rhelma_config::provider::AsyncConfigProvider;

use crate::models::{
    map_row_to_value, ObservabilityDefaults, ObservabilityRegionConfig, ObservabilityServiceConfig,
};

pub struct DbConfigProvider {
    pool: PgPool,
}

impl DbConfigProvider {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AsyncConfigProvider for DbConfigProvider {
    async fn load_defaults(&self) -> ConfigResult<Option<Value>> {
        let row = sqlx::query_as::<_, ObservabilityDefaults>(
            "SELECT * FROM observability_defaults ORDER BY id DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ConfigError::Source(e.to_string()))?;

        Ok(row.map(|r| map_row_to_value(&r)))
    }

    async fn load_region_config(&self, region: &str) -> ConfigResult<Option<Value>> {
        let row = sqlx::query_as::<_, ObservabilityRegionConfig>(
            "SELECT * FROM observability_regions WHERE region = $1 LIMIT 1",
        )
        .bind(region)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ConfigError::Source(e.to_string()))?;

        Ok(row.map(|r| map_row_to_value(&r)))
    }

    async fn load_service_config(
        &self,
        region: &str,
        service: &str,
    ) -> ConfigResult<Option<Value>> {
        let row = sqlx::query_as::<_, ObservabilityServiceConfig>(
            "SELECT * FROM observability_services WHERE region = $1 AND service_name = $2 LIMIT 1",
        )
        .bind(region)
        .bind(service)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ConfigError::Source(e.to_string()))?;

        Ok(row.map(|r| map_row_to_value(&r)))
    }
}
