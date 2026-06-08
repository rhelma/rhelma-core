#![forbid(unsafe_code)]

use std::sync::Arc;

use axum::{extract::Path, routing::get, Extension, Json, Router};
use serde::Serialize;

use rhelma_core::RhelmaError;

use crate::{
    error::{ApiResult, GatewayError},
    state::AppState,
};

/// Liveness response payload.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// Field `status`.
    pub status: String,
    /// Field `timestamp`.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Field `version`.
    pub version: String,
    /// Field `environment`.
    pub environment: String,
    /// Field `service`.
    pub service: String,
}

/// Readiness response payload.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Serialize)]
pub struct ReadyResponse {
    /// Field `status`.
    pub status: String,
    /// Field `database`.
    pub database: bool,
    /// Field `redis`.
    pub redis: bool,
    /// Field `uptime_seconds`.
    pub uptime_seconds: u64,
}

/// Region routing health snapshot payload.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Serialize)]
pub struct RegionHealthResponse {
    /// Region id.
    pub region_id: String,
    /// Whether the region is currently considered healthy.
    pub is_healthy: bool,
    /// Last observed latency in milliseconds.
    pub latency_ms: u32,
    /// Routing priority (lower is better).
    pub priority: u8,
    /// Configured endpoints for this region.
    pub endpoints: Vec<String>,
}

static STARTED_AT: once_cell::sync::Lazy<std::time::Instant> =
    once_cell::sync::Lazy::new(std::time::Instant::now);

fn uptime() -> u64 {
    STARTED_AT.elapsed().as_secs()
}

pub fn router() -> Router {
    Router::new()
        .route("/", get(live))
        .route("/ready", get(ready))
        .route("/region/{region_id}", get(region))
}

/// Liveness probe (does not touch dependencies).
pub async fn live(Extension(state): Extension<Arc<AppState>>) -> ApiResult<Json<HealthResponse>> {
    Ok(Json(HealthResponse {
        status: "ok".into(),
        timestamp: chrono::Utc::now(),
        version: env!("CARGO_PKG_VERSION").into(),
        environment: state.config.central.environment.clone(),
        service: state.config.service_name.clone(),
    }))
}

/// Readiness probe (checks critical dependencies).
pub async fn ready(Extension(state): Extension<Arc<AppState>>) -> ApiResult<Json<ReadyResponse>> {
    let db_ok = state.database.acquire().await.is_ok();
    let redis_ok = state.redis_ready().await;

    let status = if db_ok && redis_ok { "ok" } else { "degraded" };

    Ok(Json(ReadyResponse {
        status: status.to_string(),
        database: db_ok,
        redis: redis_ok,
        uptime_seconds: uptime(),
    }))
}

/// Region routing health snapshot for a single region.
///
/// This is only available when multi-region routing is enabled.
pub async fn region(
    Path(region_id): Path<String>,
    Extension(state): Extension<Arc<AppState>>,
) -> ApiResult<Json<RegionHealthResponse>> {
    let Some(router) = state.region_router.as_ref() else {
        return Err(GatewayError::from(RhelmaError::Dependency(
            "region routing not enabled".into(),
        )));
    };

    let snap = router.snapshot();
    let Some(r) = snap.get(region_id.trim()) else {
        return Err(GatewayError::from(RhelmaError::NotFound(format!(
            "unknown region: {}",
            region_id
        ))));
    };

    Ok(Json(RegionHealthResponse {
        region_id: r.region_id.clone(),
        is_healthy: r.is_healthy,
        latency_ms: r.latency_ms,
        priority: r.priority,
        endpoints: r.endpoints.clone(),
    }))
}
