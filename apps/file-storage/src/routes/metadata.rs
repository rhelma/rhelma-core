#![forbid(unsafe_code)]

use axum::{
    extract::{Extension, Path},
    Json,
};
use std::sync::Arc;

use rhelma_core::{RequestContext, RhelmaError};

use crate::{
    domain::FileRecord,
    error::{ApiError, ApiResult},
    routes::helpers::tenant_and_file_id,
    state::AppState,
};

pub async fn get_file_metadata(
    Path(file_id): Path<String>,
    Extension(state): Extension<Arc<AppState>>,
    Extension(ctx): Extension<RequestContext>,
) -> ApiResult<Json<FileRecord>> {
    let (tenant_id, id) = tenant_and_file_id(&ctx, &file_id)?;

    let rec = state
        .file_repo
        .find_by_id(&tenant_id, id)
        .await
        .map_err(|e| ApiError::with_ctx(RhelmaError::from(e), &ctx))?
        .ok_or_else(|| ApiError::with_ctx(RhelmaError::NotFound("file not found".into()), &ctx))?;

    Ok(Json(rec))
}
