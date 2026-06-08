use crate::engines::hybrid_enhanced;
use crate::models::query::{EnhancedSearchRequest, SearchResponse};
use crate::state::AppState;
use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use rhelma_core::{HttpErrorMapping, RequestContext, RhelmaError};
use tracing::instrument;

/// Routes for enhanced search endpoints.
pub fn router() -> Router<AppState> {
    Router::<AppState>::new().route("/", post(handle_enhanced_search))
}

#[instrument(skip(state, ctx, req))]
async fn handle_enhanced_search(
    State(state): State<AppState>,
    Extension(ctx): Extension<RequestContext>,
    Json(req): Json<EnhancedSearchRequest>,
) -> Response {
    let limit = req.limit.min(state.config.max_page_size);
    let query = req.query.clone();
    let mut req = req;
    req.limit = limit;

    let hits = match hybrid_enhanced::enhanced_search(&state, req).await {
        Ok(h) => h,
        Err(e) => {
            tracing::error!(error = %e, "enhanced search backend failure");
            let (status, body) =
                RhelmaError::Dependency("search backend failure".to_string()).into_http(&ctx);
            return (status, Json(body)).into_response();
        }
    };

    let total = hits.len() as u64;

    // Fire-and-forget analytics (best effort).
    // We reuse the same event contract as basic search, with a small payload marker.
    let analytics = state.analytics.clone();
    let tenant_id = ctx.tenant_id().map(|t| t.as_str().to_string());
    let user_id = ctx.user_id().map(|u| u.as_uuid().to_string());

    let request_id = ctx.request_id().to_string();
    let correlation_id = ctx
        .correlation_id()
        .map(|s| s.to_string())
        .unwrap_or_else(|| request_id.clone());
    let trace_id = ctx.trace().trace_id.clone();
    let span_id = ctx.trace().span_id.clone();

    tokio::spawn(async move {
        if let Err(e) = analytics
            .record_search_query(
                tenant_id,
                user_id,
                request_id,
                correlation_id,
                trace_id,
                span_id,
                query,
                serde_json::json!({"variant":"enhanced"}),
                total,
            )
            .await
        {
            tracing::warn!(error = %e, "failed to record enhanced search analytics");
        }
    });

    (StatusCode::OK, Json(SearchResponse { total, hits })).into_response()
}
