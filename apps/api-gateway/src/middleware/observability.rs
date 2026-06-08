#![forbid(unsafe_code)]

use axum::{body::Body, extract::MatchedPath, http::Request, middleware::Next, response::Response};
use rhelma_core::RequestContext;
use rhelma_http_observability::security::normalize_path;
use std::time::Instant;
use tracing::{info, warn};

/// axum 0.7-compatible middleware.
///
/// IMPORTANT:
/// - We intentionally rely on the canonical `RequestContext` injected by `request_guard_middleware`.
/// - We do NOT fall back to legacy request-id header.
pub async fn observability_middleware(req: Request<Body>, next: Next) -> Response {
    let start = Instant::now();

    let method = req.method().clone();

    // Prefer the route pattern (stable / low-cardinality), otherwise normalize the concrete path.
    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| normalize_path(req.uri().path()).into_owned());

    let request_id = req
        .extensions()
        .get::<RequestContext>()
        .map(|ctx| ctx.request_id().to_string())
        .or_else(|| {
            req.headers()
                .get("x-rhelma-request-id")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.trim().to_string())
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "missing".to_string());

    let resp = next.run(req).await;

    let ms = start.elapsed().as_millis() as u64;
    let status = resp.status().as_u16();

    if status >= 500 {
        warn!(status, ms, %method, %path, %request_id, "gateway request failed");
    } else {
        info!(status, ms, %method, %path, %request_id, "gateway request");
    }

    resp
}
