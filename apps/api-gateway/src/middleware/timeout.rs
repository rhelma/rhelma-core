use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use crate::state::AppState;

/// Enforce a global request timeout at the API Gateway layer.
///
/// We implement this as Axum middleware (instead of `tower_http::timeout::TimeoutLayer`) because
/// Axum's `Router` service uses `Infallible` as its error type, which does not satisfy the
/// timeout layer's error conversion bounds.
pub async fn timeout_middleware(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let timeout = state.config.timeouts.global;

    match tokio::time::timeout(timeout, next.run(req)).await {
        Ok(res) => res,
        Err(_) => StatusCode::GATEWAY_TIMEOUT.into_response(),
    }
}
