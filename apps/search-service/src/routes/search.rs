use crate::models::query::{SearchRequest, SearchResponse};
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

/// Build routes for basic search endpoints.
pub fn router() -> Router<AppState> {
    Router::<AppState>::new().route("/", post(handle_search))
}

#[instrument(skip(state, ctx, req))]
async fn handle_search(
    State(state): State<AppState>,
    Extension(ctx): Extension<RequestContext>,
    Json(req): Json<SearchRequest>,
) -> Response {
    let limit = req.limit.min(state.config.max_page_size);

    let hits = match state.hybrid.search(&req.query, limit).await {
        Ok(h) => h,
        Err(e) => {
            tracing::error!(error = %e, "search backend failure");
            let (status, body) =
                RhelmaError::Dependency("search backend failure".to_string()).into_http(&ctx);
            return (status, Json(body)).into_response();
        }
    };

    let total = hits.len() as u64;

    // Fire-and-forget analytics (best effort; never block search).
    let analytics = state.analytics.clone();
    let tenant_id = ctx.tenant_id().map(|t| t.as_str().to_string());
    let user_id = ctx.user_id().map(|u| u.as_uuid().to_string());

    let query = req.query.clone();
    let filters = req.filters.clone();
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
                filters,
                total,
            )
            .await
        {
            tracing::warn!(error = %e, "failed to record search analytics");
        }
    });

    (StatusCode::OK, Json(SearchResponse { total, hits })).into_response()
}
