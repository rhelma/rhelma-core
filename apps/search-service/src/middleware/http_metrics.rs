#![forbid(unsafe_code)]

//! Standard HTTP request metrics.

use std::time::Instant;

use axum::{body::Body, extract::MatchedPath, http::Request, middleware::Next, response::Response};

use rhelma_metrics::global as global_metrics;

fn should_record(path: &str) -> bool {
    match path {
        "/metrics" => false,
        // Admin endpoints
        "/admin/health" | "/admin/info" => true,
        // Search endpoints (note: Axum nested routers often yield a trailing "/").
        "/search" | "/search/" | "/search/enhanced" | "/search/enhanced/" => true,
        _ => false,
    }
}

pub async fn http_metrics_middleware(req: Request<Body>, next: Next) -> Response {
    let start = Instant::now();

    let matched = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| req.uri().path().to_string());

    let method = req.method().to_string();

    let resp = next.run(req).await;

    let status = resp.status().as_u16();
    let ms = start.elapsed().as_millis() as u64;

    if should_record(&matched) {
        if let Some(m) = global_metrics() {
            m.record_http_request(&method, &matched, status, ms as f64);
        }
    }

    resp
}
