//! HTTP request metrics middleware.
//!
//! Records a low-cardinality set of per-endpoint metrics aligned with Rhelma.
//!
//! Notes:
//! - Prefer Axum's `MatchedPath` (route pattern), which is stable.
//! - Normalize to a small allow-list to avoid metric cardinality blowups.

#![forbid(unsafe_code)]

use std::time::Instant;

use axum::{body::Body, extract::MatchedPath, http::Request, middleware::Next, response::Response};
use rhelma_http_observability::security::normalize_path;

/// Measures request duration and records a small, safe set of HTTP metrics.
pub async fn http_metrics_middleware(req: Request<Body>, next: Next) -> Response {
    let start = Instant::now();

    let method = normalize_method(req.method().as_str());

    // Prefer the route pattern when available; otherwise, normalize concrete path to reduce cardinality.
    let endpoint = req
        .extensions()
        .get::<MatchedPath>()
        .map(|m| normalize_endpoint(m.as_str()))
        .unwrap_or_else(|| {
            let concrete = req.uri().path();
            normalize_endpoint(normalize_path(concrete).as_ref())
        });

    let resp = next.run(req).await;

    if let Some(m) = rhelma_metrics::global() {
        let status = resp.status().as_u16();
        let duration = start.elapsed().as_secs_f64();
        m.record_http_request(method, endpoint, status, duration);
    }

    resp
}

fn normalize_method(m: &str) -> &'static str {
    match m {
        "GET" => "GET",
        "POST" => "POST",
        "PUT" => "PUT",
        "PATCH" => "PATCH",
        "DELETE" => "DELETE",
        "HEAD" => "HEAD",
        "OPTIONS" => "OPTIONS",
        _ => "OTHER",
    }
}

fn normalize_endpoint(p: &str) -> &'static str {
    match p {
        // Health
        "/health/" => "/health",
        "/health/ready" => "/health/ready",
        "/health/live" => "/health/live",
        "/healthz" => "/healthz",

        // API
        "/users/" => "/users",
        "/search/" => "/search",

        // Auth
        "/auth/health" => "/auth/health",
        "/auth/register" => "/auth/register",
        "/auth/login" => "/auth/login",
        "/auth/refresh" => "/auth/refresh",
        "/auth/logout" => "/auth/logout",

        // Admin
        "/admin/dashboard" => "/admin/dashboard",
        "/admin/metrics" => "/admin/metrics",
        "/admin/users" => "/admin/users",
        "/admin/governance/policy/runtime" => "/admin/governance/policy/runtime",
        "/admin/governance/policy/db_current" => "/admin/governance/policy/db_current",
        "/admin/governance/policy/ingest" => "/admin/governance/policy/ingest",

        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_normalization_is_low_cardinality() {
        // Concrete paths should collapse.
        assert_eq!(normalize_endpoint("/users/123"), "other");
        assert_eq!(
            normalize_endpoint("/admin/governance/policy/runtime?x=1"),
            "other"
        );

        // Route-pattern paths / stable endpoints are allowed.
        assert_eq!(normalize_endpoint("/users/"), "/users");
        assert_eq!(normalize_endpoint("/search/"), "/search");
        assert_eq!(normalize_endpoint("/auth/login"), "/auth/login");
        assert_eq!(normalize_endpoint("/admin/metrics"), "/admin/metrics");
    }
}
