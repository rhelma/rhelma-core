#![forbid(unsafe_code)]
#![allow(dead_code)]

use axum::Router;
use rhelma_http_observability::axum::{trace_layer_v60, ContractV60Layer};
use std::net::SocketAddr;
use tracing::info;

mod analytics;
mod config;
mod engines;
mod features;
mod metrics_endpoint;
mod middleware;
mod models;
mod routes;
mod state;

// Query-understanding is experimental and optional. Keep it behind a feature so
// it never blocks the default build / CI.
#[cfg(feature = "query-understanding")]
mod query_understanding;

use crate::config::SearchConfig;
use crate::state::AppState;

/// Entrypoint for the search-service binary.
///
/// This binary:
/// - loads configuration
/// - initializes observability (tracing, metrics)
/// - builds shared AppState
/// - starts the HTTP server with all routes wired.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = SearchConfig::from_env()?;
    cfg.validate()?;

    init_observability(&cfg.service_name).await?;

    let app_state = AppState::initialize(cfg.clone()).await?;
    let app = build_router(app_state);

    let addr: SocketAddr = cfg
        .listen_addr
        .parse()
        .expect("Invalid listen_addr in SearchConfig");

    info!(%addr, "starting search-service");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn init_observability(service_name: &str) -> anyhow::Result<()> {
    let central = rhelma_config::CentralEnv::from_env_strict()?;
    let unified =
        rhelma_config::UnifiedObservabilityConfig::from_central_env(&central, service_name);
    rhelma_config::validation::validate_all(&unified, &central)?;
    rhelma_observability_core::ObservabilityCore::init_from_unified(unified).await?;

    // Best-effort: expose `/metrics` scrape endpoint.
    // If another recorder is already installed by observability-core, this simply
    // keeps `/metrics` returning 200 with a minimal body.
    crate::metrics_endpoint::init_prometheus_recorder();
    Ok(())
}

async fn shutdown_signal() {
    // Simple Ctrl+C handler; can be extended with Kubernetes SIGTERM, etc.
    let _ = tokio::signal::ctrl_c().await;
}

fn build_router(state: AppState) -> Router {
    use axum::middleware::from_fn;

    use crate::routes::{admin, search, search_enhanced};

    Router::new()
        .route(
            "/metrics",
            axum::routing::get(crate::metrics_endpoint::metrics_handler),
        )
        .nest("/search", search::router())
        .nest("/search/enhanced", search_enhanced::router())
        .nest("/admin", admin::router())
        .with_state(state)
        // Route-level HTTP metrics (requires MatchedPath).
        .route_layer(from_fn(middleware::http_metrics::http_metrics_middleware))
        // Ensure RequestContext exists for handlers + analytics even when
        // search-service is accessed directly (without api-gateway).
        .layer(from_fn(
            middleware::request_context::request_context_middleware,
        ))
        .layer(trace_layer_v60())
        .layer(rhelma_http_observability::axum::ScopeHeadersLayer)
        .layer(ContractV60Layer)
}

#[cfg(test)]
mod metrics_endpoint_test;
