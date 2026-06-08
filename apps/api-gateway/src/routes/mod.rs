#![forbid(unsafe_code)]

use axum::{Extension, Router};
use std::sync::Arc;
use tower::ServiceBuilder;

use crate::middleware;
use crate::state::AppState;
use rhelma_http_observability::axum::{trace_layer_v60, ContractV60Layer, ScopeHeadersLayer};

pub mod admin;
pub mod auth;
pub mod docs;
pub mod governance;
pub mod health;
pub mod search;
pub mod social;
pub mod users;

pub fn build_router(state: Arc<AppState>) -> Router {
    let cors = middleware::create_cors_layer(state.config.env_name(), &state.config.cors);

    // Global layers (apply to all routes)
    //
    // Execution order (outer -> inner):
    // - error_envelope (wraps 4xx/5xx into v5.2 JSON envelope)
    // - cors
    let global_stack = ServiceBuilder::new()
        .layer(cors)
        .layer(axum::middleware::from_fn(
            middleware::error_envelope_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::timeout_middleware,
        ));

    // API-only layers
    //
    // Execution order (outer -> inner):
    // - request_guard (canonicalize headers + RequestContext)
    // - observability (logs with canonical request-id)
    // - rate_limit
    let api_stack = ServiceBuilder::new()
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::rate_limit_middleware,
        ))
        .layer(axum::middleware::from_fn(
            middleware::observability_middleware,
        ))
        .layer(axum::middleware::from_fn(
            middleware::request_guard_middleware,
        ));

    // Make AuthService available to extractors via Extension<Arc<AuthService>>.
    let auth = Arc::new(state.auth_service.clone());

    let health_router = {
        let r = Router::new()
            .nest("/health", health::router())
            .route("/docs", axum::routing::get(docs::docs_landing))
            .route(
                "/api-docs/openapi.json",
                axum::routing::get(docs::openapi_json),
            )
            .route_layer(axum::middleware::from_fn(
                middleware::http_metrics_middleware,
            ))
            .layer(Extension(state.clone()));

        #[cfg(feature = "openapi")]
        let r = r.merge(crate::openapi::swagger_router());

        r
    };

    let api_router = Router::new()
        .nest("/users", users::router())
        .nest("/search", search::router())
        .nest("/social", social::router())
        .nest("/auth", auth::router())
        .nest("/admin", admin::router())
        .route_layer(axum::middleware::from_fn(
            middleware::http_metrics_middleware,
        ))
        .layer(api_stack)
        .layer(Extension(auth))
        .layer(Extension(state));

    Router::new()
        .merge(health_router)
        .merge(api_router)
        .layer(global_stack)
        .layer(trace_layer_v60())
        .layer(ScopeHeadersLayer)
        .layer(ContractV60Layer)
}
