#![forbid(unsafe_code)]

use axum::{
    body::Body,
    extract::Query,
    http::Request,
    middleware::Next,
    response::{IntoResponse, Response},
    Extension, Json,
};
use rhelma_auth::UserPrincipal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    error::{ApiResult, GatewayError},
    middleware::AuthUserExtractor,
    state::AppState,
};

#[derive(Debug, Serialize)]
pub struct SystemMetrics {
    /// Field `active_connections`.
    pub active_connections: u64,
    /// Field `memory_usage`.
    pub memory_usage: MemoryUsage,
    /// Field `database_metrics`.
    pub database_metrics: DatabaseMetrics,
    /// Field `cache_metrics`.
    pub cache_metrics: CacheMetrics,
    /// Field `request_metrics`.
    pub request_metrics: RequestMetrics,
}

#[derive(Debug, Serialize)]
pub struct MemoryUsage {
    /// Field `used`.
    pub used: u64,
    /// Field `total`.
    pub total: u64,
    /// Field `percentage`.
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct DatabaseMetrics {
    /// Field `active_connections`.
    pub active_connections: u32,
    /// Field `idle_connections`.
    pub idle_connections: u32,
    /// Field `query_count`.
    pub query_count: u64,
}

#[derive(Debug, Serialize)]
pub struct CacheMetrics {
    /// Field `hit_rate`.
    pub hit_rate: f64,
    /// Field `total_operations`.
    pub total_operations: u64,
    /// Field `memory_used`.
    pub memory_used: u64,
}

#[derive(Debug, Serialize)]
pub struct RequestMetrics {
    /// Field `total_requests`.
    pub total_requests: u64,
    /// Field `requests_per_second`.
    pub requests_per_second: f64,
    /// Field `error_rate`.
    pub error_rate: f64,
}

#[derive(Debug, Deserialize)]
pub struct MetricsQuery {
    /// Field `timeframe`.
    pub timeframe: Option<String>,
}

/// Centralized admin permission check
fn ensure_admin(principal: &UserPrincipal) -> Result<(), GatewayError> {
    let is_admin = principal
        .permissions
        .iter()
        .any(|p| p.0 == "admin" || p.0.starts_with("admin:"));

    if !is_admin {
        return Err(GatewayError::from(rhelma_core::RhelmaError::Authz(
            "Admin privileges required".to_string(),
        )));
    }
    Ok(())
}

/// ✅ Admin-only middleware (early reject)
async fn require_admin_middleware(req: Request<Body>, next: Next) -> Response {
    let principal = match req.extensions().get::<UserPrincipal>() {
        Some(p) => p,
        None => {
            return GatewayError::unauthorized("authentication required").into_response();
        }
    };

    if let Err(err) = ensure_admin(principal) {
        return err.into_response();
    }

    next.run(req).await
}

pub fn router() -> axum::Router {
    use super::governance;
    use axum::{middleware, routing::get};

    axum::Router::new()
        .route("/dashboard", get(admin_dashboard_handler))
        .route("/metrics", get(system_metrics_handler))
        .route(
            "/region-routing/snapshot",
            get(region_routing_snapshot_handler),
        )
        .route(
            "/region-routing/override",
            get(region_routing_override_handler),
        )
        .route(
            "/region-routing/simulate-failover",
            axum::routing::post(region_routing_simulate_failover_handler),
        )
        .route("/users", get(user_management_handler))
        .nest("/governance", governance::router())
        // ✅ admin guard for all admin routes
        .layer(middleware::from_fn(require_admin_middleware))
}

pub async fn admin_dashboard_handler(
    Extension(_state): Extension<Arc<AppState>>,
    auth_user: AuthUserExtractor,
) -> ApiResult<Json<serde_json::Value>> {
    let principal: UserPrincipal = auth_user.0;

    tracing::info!(
        target: "ops.audit",
        user_id = %principal.user_id,
        operation = "admin_dashboard",
        "Admin dashboard access"
    );

    Ok(Json(serde_json::json!({
        "system_status": "healthy",
        "active_users": 0,
        "total_requests": 0,
        "services": {
            "api_gateway": "running",
            "auth_service": "running",
            "search_service": "running",
            "user_service": "running"
        }
    })))
}

pub async fn system_metrics_handler(
    Extension(_state): Extension<Arc<AppState>>,
    auth_user: AuthUserExtractor,
    Query(query): Query<MetricsQuery>,
) -> ApiResult<Json<SystemMetrics>> {
    let principal: UserPrincipal = auth_user.0;

    tracing::info!(
        target: "ops.audit",
        user_id = %principal.user_id,
        timeframe = ?query.timeframe,
        operation = "admin_metrics",
        "System metrics access"
    );

    Ok(Json(SystemMetrics {
        active_connections: 0,
        memory_usage: MemoryUsage {
            used: 0,
            total: 0,
            percentage: 0.0,
        },
        database_metrics: DatabaseMetrics {
            active_connections: 0,
            idle_connections: 0,
            query_count: 0,
        },
        cache_metrics: CacheMetrics {
            hit_rate: 0.0,
            total_operations: 0,
            memory_used: 0,
        },
        request_metrics: RequestMetrics {
            total_requests: 0,
            requests_per_second: 0.0,
            error_rate: 0.0,
        },
    }))
}

pub async fn user_management_handler(
    Extension(_state): Extension<Arc<AppState>>,
    auth_user: AuthUserExtractor,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<serde_json::Value>> {
    let principal: UserPrincipal = auth_user.0;

    let page = params.get("page").and_then(|p| p.parse().ok()).unwrap_or(1);
    let limit = params
        .get("limit")
        .and_then(|l| l.parse().ok())
        .unwrap_or(50);

    tracing::info!(
        target: "ops.audit",
        user_id = %principal.user_id,
        operation = "admin_user_management",
        page,
        limit,
        "User management access"
    );

    Ok(Json(serde_json::json!({
        "users": [],
        "pagination": {
            "page": page,
            "limit": limit,
            "total": 0
        }
    })))
}

// -----------------------------------------------------------------------------
// Region routing debug endpoints (admin-only)
// -----------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct RegionOverrideQuery {
    pub upstream: String,
}

#[derive(Debug, Deserialize)]
pub struct SimulateFailoverRequest {
    pub upstream_service: String,
    pub from_region: String,
    pub to_region: String,
    #[serde(default)]
    pub reason: Option<String>,
}

/// Returns the current router snapshot (regions + active overrides).
pub async fn region_routing_snapshot_handler(
    Extension(state): Extension<Arc<AppState>>,
    auth_user: AuthUserExtractor,
) -> ApiResult<Json<serde_json::Value>> {
    let principal: UserPrincipal = auth_user.0;

    tracing::info!(
        target: "ops.audit",
        user_id = %principal.user_id,
        operation = "admin_region_routing_snapshot",
        "Region routing snapshot requested"
    );

    let Some(handle) = state.region_router.as_ref() else {
        return Ok(Json(serde_json::json!({
            "enabled": false
        })));
    };

    let regions = handle
        .snapshot()
        .into_values()
        .map(|r| {
            serde_json::json!({
                "region_id": r.region_id,
                "priority": r.priority,
                "is_healthy": r.is_healthy,
                "latency_ms": r.latency_ms,
                "endpoints": r.endpoints,
            })
        })
        .collect::<Vec<_>>();

    let overrides = handle.overrides_snapshot();

    Ok(Json(serde_json::json!({
        "enabled": true,
        "regions": regions,
        "overrides": overrides,
    })))
}

/// Returns the active override (if any) for a single upstream.
pub async fn region_routing_override_handler(
    Extension(state): Extension<Arc<AppState>>,
    auth_user: AuthUserExtractor,
    Query(q): Query<RegionOverrideQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let principal: UserPrincipal = auth_user.0;

    tracing::info!(
        target: "ops.audit",
        user_id = %principal.user_id,
        operation = "admin_region_routing_override",
        upstream = %q.upstream,
        "Region routing override requested"
    );

    let Some(handle) = state.region_router.as_ref() else {
        return Ok(Json(serde_json::json!({
            "enabled": false,
            "override": null
        })));
    };

    let ov = handle.active_override(q.upstream.as_str());

    Ok(Json(serde_json::json!({
        "enabled": true,
        "override": ov
    })))
}

/// Simulate a failover override (does not require Kafka).
///
/// This is useful for validating end-to-end routing behavior in staging.
pub async fn region_routing_simulate_failover_handler(
    Extension(state): Extension<Arc<AppState>>,
    auth_user: AuthUserExtractor,
    Json(req): Json<SimulateFailoverRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let principal: UserPrincipal = auth_user.0;

    tracing::warn!(
        target: "ops.audit",
        user_id = %principal.user_id,
        operation = "admin_region_routing_simulate_failover",
        upstream = %req.upstream_service,
        from = %req.from_region,
        to = %req.to_region,
        "Simulating region failover override (admin)"
    );

    let Some(handle) = state.region_router.as_ref() else {
        return Ok(Json(serde_json::json!({
            "enabled": false,
            "applied": false
        })));
    };

    handle.note_failover(
        req.upstream_service.as_str(),
        req.from_region.as_str(),
        req.to_region.as_str(),
        req.reason.as_deref().unwrap_or("admin_simulation"),
    );

    let ov = handle.active_override(req.upstream_service.as_str());

    Ok(Json(serde_json::json!({
        "enabled": true,
        "applied": ov.is_some(),
        "override": ov
    })))
}
