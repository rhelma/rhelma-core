#![cfg(feature = "openapi")]
#![forbid(unsafe_code)]

//! Optional OpenAPI + Swagger UI integration for `api-gateway`.
//!
//! This module is feature-gated behind `api-gateway/openapi` so the default build
//! remains lightweight and does not require OpenAPI generation dependencies.
//!
//! When enabled, the API Gateway exposes:
//! - OpenAPI JSON: `GET /api-docs/openapi.json`
//! - Swagger UI: `GET /swagger-ui`

use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::routes::{auth, health};

/// OpenAPI doc definition for the API Gateway.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Rhelma API Gateway",
        version = env!("CARGO_PKG_VERSION"),
        description = "Enterprise-grade API Gateway for Rhelma Platform (Rhelma6)."
    ),
    paths(
        health_live,
        health_ready,
        auth_login,
        auth_register,
        auth_refresh,
        auth_logout,
    ),
    components(
        schemas(
            health::HealthResponse,
            health::ReadyResponse,
            auth::LoginRequest,
            auth::RegisterRequest,
            auth::RefreshRequest,
            auth::AuthResponse,
        )
    ),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "auth", description = "Authentication endpoints"),
    )
)]
pub struct ApiDoc;

/// Router that mounts Swagger UI under `/swagger-ui`.
#[must_use]
pub fn swagger_router() -> Router {
    // `SwaggerUi` integrates with Axum via the `axum` feature.
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
}

// -----------------------------------------------------------------------------
// Documentation-only stubs.
//
// We intentionally keep OpenAPI descriptions decoupled from the concrete handler
// signatures (which use `Extension<...>` extractors). This avoids doc generation
// friction while still producing an accurate contract.
// -----------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/health/",
    tag = "health",
    responses(
        (status = 200, description = "Liveness probe (does not touch dependencies)", body = health::HealthResponse)
    )
)]
async fn health_live() {}

#[utoipa::path(
    get,
    path = "/health/ready",
    tag = "health",
    responses(
        (status = 200, description = "Readiness probe (checks dependencies)", body = health::ReadyResponse)
    )
)]
async fn health_ready() {}

#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "auth",
    request_body = auth::LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = auth::AuthResponse),
        (status = 401, description = "Invalid credentials")
    )
)]
async fn auth_login() {}

#[utoipa::path(
    post,
    path = "/auth/register",
    tag = "auth",
    request_body = auth::RegisterRequest,
    responses(
        (status = 200, description = "Registration successful", body = auth::AuthResponse),
        (status = 409, description = "Email already registered")
    )
)]
async fn auth_register() {}

#[utoipa::path(
    post,
    path = "/auth/refresh",
    tag = "auth",
    request_body = auth::RefreshRequest,
    responses(
        (status = 200, description = "Token refreshed", body = auth::AuthResponse),
        (status = 401, description = "Invalid refresh token")
    )
)]
async fn auth_refresh() {}

#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = "auth",
    responses(
        (status = 200, description = "Session revoked")
    )
)]
async fn auth_logout() {}
