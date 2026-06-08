use crate::domain::{FileId, FileRecord, FileStatus, StorageBackendKind};
use crate::error::FileStorageResult;
use chrono::Utc;
use sqlx::FromRow;

#[derive(Clone)]
pub struct PgFileRepository {
    pool: sqlx::PgPool,
}

impl PgFileRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, rec: &FileRecord) -> FileStorageResult<()> {
        let query = r#"
            INSERT INTO files (
                id, tenant_id, region, original_name, content_type,
                size_bytes, checksum, storage_backend, storage_path,
                status, created_at, created_by, deleted_at
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
        "#;

        sqlx::query(query)
            .bind(rec.id.0)
            .bind(&rec.tenant_id)
            .bind(&rec.region)
            .bind(&rec.original_name)
            .bind(&rec.content_type)
            .bind(rec.size_bytes)
            .bind(&rec.checksum)
            .bind(match rec.storage_backend {
                StorageBackendKind::LocalFs => "local",
                StorageBackendKind::S3 => "s3",
            })
            .bind(&rec.storage_path)
            .bind(match rec.status {
                FileStatus::Active => "active",
                FileStatus::Deleted => "deleted",
                FileStatus::Archived => "archived",
            })
            .bind(rec.created_at)
            .bind(&rec.created_by)
            .bind(rec.deleted_at)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn find_by_id(
        &self,
        tenant_id: &str,
        id: FileId,
    ) -> FileStorageResult<Option<FileRecord>> {
        let query = r#"
            SELECT id, tenant_id, region, original_name, content_type,
                   size_bytes, checksum, storage_backend, storage_path,
                   status, created_at, created_by, deleted_at
            FROM files
            WHERE id = $1 AND tenant_id = $2
        "#;

        let row = sqlx::query_as::<_, FileRow>(query)
            .bind(id.0)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| r.into_record()))
    }

    pub async fn soft_delete(&self, tenant_id: &str, id: FileId) -> FileStorageResult<()> {
        let query = r#"
            UPDATE files
            SET status = 'deleted', deleted_at = $1
            WHERE id = $2 AND tenant_id = $3
        "#;

        sqlx::query(query)
            .bind(Utc::now())
            .bind(id.0)
            .bind(tenant_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[derive(Debug, FromRow)]
struct FileRow {
    pub id: uuid::Uuid,
    pub tenant_id: String,
    pub region: String,
    pub original_name: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub checksum: String,
    pub storage_backend: String,
    pub storage_path: String,
    pub status: String,
    pub created_at: chrono::DateTime<Utc>,
    pub created_by: Option<String>,
    pub deleted_at: Option<chrono::DateTime<Utc>>,
}

impl FileRow {
    fn into_record(self) -> FileRecord {
        FileRecord {
            id: FileId(self.id),
            tenant_id: self.tenant_id,
            region: self.region,
            original_name: self.original_name,
            content_type: self.content_type,
            size_bytes: self.size_bytes,
            checksum: self.checksum,
            storage_backend: match self.storage_backend.as_str() {
                "s3" => StorageBackendKind::S3,
                _ => StorageBackendKind::LocalFs,
            },
            storage_path: self.storage_path,
            status: match self.status.as_str() {
                "deleted" => FileStatus::Deleted,
                "archived" => FileStatus::Archived,
                _ => FileStatus::Active,
            },
            created_at: self.created_at,
            created_by: self.created_by,
            deleted_at: self.deleted_at,
        }
    }
}
