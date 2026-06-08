#![forbid(unsafe_code)]

use axum::{
    extract::{Extension, Multipart},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use bytes::Bytes;
use chrono::Utc;
use rhelma_config::central_env::CentralEnv;
use std::sync::Arc;

use crate::{
    config::FileStorageConfig,
    domain::{FileId, FileRecord, FileStatus, StorageBackendKind, StoragePolicy},
    error::{ApiError, ApiResult},
    services::antivirus,
    state::AppState,
};
use rhelma_core::{RequestContext, RhelmaError};

#[derive(Debug, serde::Serialize)]
pub struct UploadResponse {
    /// Field `file_id`.
    pub file_id: String,
    /// Field `download_url`.
    pub download_url: String,
    /// Field `size_bytes`.
    pub size_bytes: u64,
    /// Field `content_type`.
    pub content_type: String,
    /// Field `checksum`.
    pub checksum: String,
}

pub async fn upload_file(
    Extension(state): Extension<Arc<AppState>>,
    Extension(cfg): Extension<Arc<FileStorageConfig>>,
    Extension(ctx): Extension<RequestContext>,
    mut multipart: Multipart,
) -> ApiResult<impl IntoResponse> {
    let tenant_id = ctx
        .tenant_id()
        .map(|t| t.as_str().to_string())
        .ok_or_else(|| {
            ApiError::with_ctx(RhelmaError::BadRequest("missing tenant_id".into()), &ctx)
        })?;

    let region = ctx
        .region()
        .map(|r| r.as_str().to_string())
        .unwrap_or_else(|| {
            CentralEnv::from_env_with_defaults("global", "development", "0.0.0-dev").region
        });

    // Extract the `file` field.
    //
    // Contract:
    // - multipart field name MUST be `file`
    // - other fields (e.g. metadata) are ignored for now
    let mut filename = "upload.bin".to_string();
    let mut content_type = "application/octet-stream".to_string();
    let mut file_bytes: Option<Bytes> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::with_ctx(RhelmaError::BadRequest(e.to_string()), &ctx))?
    {
        if field.name() == Some("file") {
            if let Some(f) = field.file_name() {
                filename = f.to_string();
            }
            if let Some(ct) = field.content_type() {
                content_type = ct.to_string();
            }
            let data = field
                .bytes()
                .await
                .map_err(|e| ApiError::with_ctx(RhelmaError::BadRequest(e.to_string()), &ctx))?;
            file_bytes = Some(data);
            break;
        }
    }

    let file_bytes = file_bytes
        .ok_or_else(|| ApiError::with_ctx(RhelmaError::BadRequest("missing file".into()), &ctx))?;

    // Policy enforcement
    let policy = StoragePolicy::new(&cfg);
    policy
        .validate_size(file_bytes.len() as u64)
        .map_err(|e| ApiError::with_ctx(RhelmaError::from(e), &ctx))?;
    policy
        .validate_mime(&content_type)
        .map_err(|e| ApiError::with_ctx(RhelmaError::from(e), &ctx))?;

    if policy.should_scan(&file_bytes) {
        antivirus::scan_bytes(&file_bytes)
            .await
            .map_err(|e| ApiError::with_ctx(RhelmaError::from(e), &ctx))?;
    }

    // Storage record
    let file_id = FileId::new();
    let storage_backend = match cfg.default_provider {
        crate::config::StorageProviderKind::S3 => StorageBackendKind::S3,
        _ => StorageBackendKind::LocalFs,
    };

    let storage_path = format!("{file_id}.bin");
    let checksum = blake3::hash(&file_bytes).to_hex().to_string();

    let record = FileRecord {
        id: file_id,
        tenant_id: tenant_id.clone(),
        region: region.clone(),
        original_name: filename.clone(),
        content_type: content_type.clone(),
        size_bytes: file_bytes.len() as i64,
        checksum: checksum.clone(),
        storage_backend,
        storage_path,
        status: FileStatus::Active,
        created_at: Utc::now(),
        created_by: ctx.user_id().map(|u| u.0.to_string()),
        deleted_at: None,
    };

    // DB-first to avoid orphan objects when DB insert fails.
    // If the backend upload fails, we soft-delete the record and (best-effort)
    // delete the partially uploaded object.
    state
        .file_repo
        .insert(&record)
        .await
        .map_err(|e| ApiError::with_ctx(RhelmaError::from(e), &ctx))?;

    let backend = state
        .storage_backend
        .backend_for(record.storage_backend.clone());
    if let Err(e) = backend.put(&record, file_bytes).await {
        // Best-effort cleanup.
        let _ = backend.delete(&record).await;
        let _ = state.file_repo.soft_delete(&tenant_id, record.id).await;
        return Err(ApiError::with_ctx(RhelmaError::from(e), &ctx));
    }

    // Audit (best-effort)
    let _ = state
        .audit_repo
        .record_event(
            &tenant_id,
            &file_id.to_string(),
            "upload",
            ctx.user_id().map(|u| u.0.to_string()),
            Some("uploaded via multipart".to_string()),
        )
        .await;

    Ok((
        StatusCode::CREATED,
        Json(UploadResponse {
            file_id: file_id.to_string(),
            download_url: format!("/files/{file_id}"),
            size_bytes: record.size_bytes as u64,
            content_type: record.content_type,
            checksum,
        }),
    ))
}
