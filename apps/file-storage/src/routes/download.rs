#![forbid(unsafe_code)]

use axum::{
    extract::{Extension, Path},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use rhelma_core::{RequestContext, RhelmaError};

use crate::{
    error::{ApiError, ApiResult},
    routes::helpers::{sanitize_filename, tenant_and_file_id},
    state::AppState,
};

pub async fn download_file(
    Path(file_id): Path<String>,
    Extension(state): Extension<Arc<AppState>>,
    Extension(ctx): Extension<RequestContext>,
) -> ApiResult<Response> {
    let (tenant_id, id) = tenant_and_file_id(&ctx, &file_id)?;

    let rec = state
        .file_repo
        .find_by_id(&tenant_id, id)
        .await
        .map_err(|e| ApiError::with_ctx(RhelmaError::from(e), &ctx))?
        .ok_or_else(|| ApiError::with_ctx(RhelmaError::NotFound("file not found".into()), &ctx))?;

    let backend = state
        .storage_backend
        .backend_for(rec.storage_backend.clone());
    let bytes = backend
        .get(&rec)
        .await
        .map_err(|e| ApiError::with_ctx(RhelmaError::from(e), &ctx))?;

    // Defensive: prevent header injection / invalid header values.
    let safe_name = sanitize_filename(&rec.original_name);
    let mut resp = (StatusCode::OK, bytes).into_response();

    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&rec.content_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    resp.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", safe_name))
            .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
    );

    Ok(resp)
}
