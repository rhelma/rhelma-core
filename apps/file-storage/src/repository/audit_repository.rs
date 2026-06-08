use crate::error::FileStorageResult;
use chrono::Utc;

#[derive(Clone)]
pub struct PgAuditRepository {
    pool: sqlx::PgPool,
}

impl PgAuditRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub async fn record_event(
        &self,
        tenant_id: &str,
        file_id: &str,
        event_type: &str,
        actor: Option<String>,
        details: Option<String>,
    ) -> FileStorageResult<()> {
        let query = r#"
            INSERT INTO file_audit_log (
                tenant_id, file_id, event_type, actor, details, created_at
            )
            VALUES ($1,$2,$3,$4,$5,$6)
        "#;

        sqlx::query(query)
            .bind(tenant_id)
            .bind(file_id)
            .bind(event_type)
            .bind(actor)
            .bind(details)
            .bind(Utc::now())
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
