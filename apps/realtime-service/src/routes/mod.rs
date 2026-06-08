#![forbid(unsafe_code)]

use axum::{middleware::from_fn, routing::get, Json, Router};
use chrono::Utc;
use serde_json::json;

use rhelma_http_observability::axum::{trace_layer_v60, ContractV60Layer, ScopeHeadersLayer};

use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/metrics", get(crate::metrics_endpoint::metrics_handler))
        // Liveness: process is up.
        .route("/healthz", get(healthz))
        // Readiness: dependencies are (best-effort) initialized.
        .route("/readyz", get(readyz))
        .merge(crate::ws::router())
        .with_state(state)
        // Route-level HTTP metrics (requires MatchedPath).
        .route_layer(from_fn(
            crate::middleware::http_metrics::http_metrics_middleware,
        ))
        .layer(trace_layer_v60())
        .layer(ScopeHeadersLayer)
        // Ensure v5.2 canonical headers exist for all requests (incl. WS upgrade).
        .layer(ContractV60Layer)
}

async fn healthz() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "ts": Utc::now().to_rfc3339(),
    }))
}

async fn readyz() -> Json<serde_json::Value> {
    // In v5.2, realtime-service has no hard external dependencies at boot
    // (auth + event bus are best-effort when allow_anonymous=true).
    Json(json!({ "status": "ready" }))
}
