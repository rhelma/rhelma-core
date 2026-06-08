#![forbid(unsafe_code)]

use axum::{routing::post, Extension, Json, Router};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    error::{ApiError, ApiResult},
    middleware::OptionalAuthUserExtractor,
    state::AppState,
};
use rhelma_core::RequestContext;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct SearchRequest {
    #[validate(length(min = 1, max = 512))]
    /// Field `query`.
    pub query: String,

    #[validate(range(min = 1, max = 100))]
    /// Field `limit`.
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResponse {
    /// Field `query`.
    pub query: String,
    /// Field `limit`.
    pub limit: u32,
    /// Field `hits`.
    pub hits: Vec<serde_json::Value>,
}

pub fn router() -> Router {
    Router::new().route("/", post(search))
}

pub async fn search(
    Extension(state): Extension<std::sync::Arc<AppState>>,
    Extension(ctx): Extension<RequestContext>,
    OptionalAuthUserExtractor(_principal): OptionalAuthUserExtractor,
    Json(req): Json<SearchRequest>,
) -> ApiResult<Json<SearchResponse>> {
    req.validate()
        .map_err(|e| ApiError::bad_request(format!("invalid request: {e}")))?;

    let limit = req.limit.unwrap_or(20).min(100);

    // Forward to upstream search-service via SearchService wrapper.
    let hits = state.search_service.search(&ctx, &req.query, limit).await?;

    Ok(Json(SearchResponse {
        query: req.query,
        limit,
        hits,
    }))
}
