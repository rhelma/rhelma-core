#![forbid(unsafe_code)]

use axum::http::HeaderValue;
use axum::{
    middleware::from_fn,
    routing::{get, post},
    Extension, Router,
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;

use rhelma_config::CentralEnv;
use rhelma_http_observability::axum::{trace_layer_v60, ContractV60Layer, ScopeHeadersLayer};
use rhelma_observability_core::ObservabilityCore;

mod config;
mod domain;
mod error;
mod health;
mod metrics_endpoint;
mod middleware;
mod repository;
mod routes;
mod services;
mod state;

use crate::config::FileStorageConfig;
use crate::middleware::{
    error_envelope_middleware, rate_limit::RateLimiter, rate_limit_middleware,
    request_guard_middleware,
};
use crate::routes::{download::download_file, metadata::get_file_metadata, upload::upload_file};
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Arc::new(FileStorageConfig::from_env_strict()?);
    // Touch policy-related config fields so they remain validated/visible even if
    // not actively enforced by routes yet.
    let _policy_cfg = (&cfg.region, cfg.encryption_at_rest, cfg.retention_days);

    // Init logging / tracing / metrics (unified, contract-aligned)
    let central = CentralEnv::from_env_strict()?;
    let unified =
        rhelma_config::UnifiedObservabilityConfig::from_central_env(&central, &cfg.service_name);
    rhelma_config::validation::validate_all(&unified, &central)?;
    let _obs = ObservabilityCore::init_from_unified(unified).await?;

    // Best-effort Prometheus recorder for `/metrics`.
    crate::metrics_endpoint::init_prometheus_recorder();

    let state = Arc::new(AppState::new(cfg.clone()).await?);
    let rate_limiter = Arc::new(RateLimiter::new());

    // CORS
    // - In production: strict allow-list. Any invalid origin fails startup.
    // - In non-prod: permissive for local development.
    let cors = if cfg.is_prod() {
        if cfg.cors_allowed_origins.is_empty() {
            CorsLayer::new()
        } else {
            let mut origins: Vec<HeaderValue> = Vec::with_capacity(cfg.cors_allowed_origins.len());
            for o in &cfg.cors_allowed_origins {
                let hv = o
                    .parse::<HeaderValue>()
                    .map_err(|_| format!("invalid CORS origin value: {o}"))?;
                origins.push(hv);
            }
            CorsLayer::new().allow_origin(AllowOrigin::list(origins))
        }
    } else {
        CorsLayer::new().allow_origin(Any)
    };

    // Per-route body limit (upload).
    let upload_router = Router::new()
        .route("/files", post(upload_file))
        .layer(RequestBodyLimitLayer::new(cfg.max_file_size_bytes as usize));

    let app = Router::new()
        .route("/metrics", get(metrics_endpoint::metrics_handler))
        .route("/health", get(health::health_check))
        .route("/health/deps", get(health::health_deps))
        .route("/files/{id}", get(download_file))
        .route("/files/{id}/metadata", get(get_file_metadata))
        .merge(upload_router)
        .layer(Extension(state))
        .layer(Extension(rate_limiter))
        .layer(Extension(cfg.clone()))
        .layer(
            ServiceBuilder::new()
                // Order matters: last layer is outermost.
                .layer(cors)
                .layer(from_fn(rate_limit_middleware))
                // Enforce tenant isolation for all API routes.
                // This runs after `request_guard_middleware` which populates `RequestContext`.
                .layer(from_fn(middleware::tenant::tenant_middleware))
                .layer(from_fn(error_envelope_middleware)),
        )
        // Standard v6.0 observability stack.
        .layer(trace_layer_v60())
        .layer(ScopeHeadersLayer)
        // RequestContext + region defaulting (service-specific), after contract.
        .layer(from_fn(request_guard_middleware))
        // Must be outermost: ensures request has canonical ids + traceparent.
        .layer(ContractV60Layer);

    let listener = tokio::net::TcpListener::bind(cfg.bind_addr()).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod metrics_endpoint_test;
