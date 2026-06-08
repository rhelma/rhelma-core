#![forbid(unsafe_code)]

use axum::{routing::get, Extension, Router};
use std::sync::Arc;

use rhelma_http_observability::axum::{trace_layer_v60, ContractV60Layer, ScopeHeadersLayer};

use crate::middleware::request_context::request_context_middleware;
use crate::state::AppState;

pub mod metrics;
pub mod social;

pub fn build_router(state: Arc<AppState>) -> Router {
    let auth = Arc::new(state.auth_service.clone());

    Router::new()
        .route("/health", get(social::health))
        .route("/metrics", get(metrics::metrics_handler))
        .merge(social::router())
        .layer(axum::middleware::from_fn(request_context_middleware))
        .layer(Extension(auth))
        .layer(Extension(state))
        .layer(ScopeHeadersLayer)
        .layer(ContractV60Layer)
        .layer(trace_layer_v60())
}
