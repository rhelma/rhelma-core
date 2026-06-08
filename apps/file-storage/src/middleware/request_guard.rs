#![forbid(unsafe_code)]

use axum::{
    body::Body,
    http::{header::HeaderName, HeaderValue, Request},
    middleware::Next,
    response::Response,
};

use rhelma_config::central_env::CentralEnv;
use rhelma_core::{constants, RequestContext};
use rhelma_http_observability::axum::ensure_contract_v60_headers;

/// RequestContext injection middleware for file-storage.
///
/// Responsibilities (fail-open):
/// - Ensure Contract v6.0 tracing headers exist (idempotent)
/// - Normalize service region (default = `RHELMA_REGION` / `global`) when missing
/// - Build and attach [`RequestContext`]
/// - Echo canonical headers on the response
pub async fn request_guard_middleware(mut req: Request<Body>, next: Next) -> Response {
    // Ensure minimum Contract v6.0 headers exist. This is idempotent and fail-open.
    ensure_contract_v60_headers(req.headers_mut());

    // Region (default = service region)
    let region = req
        .headers()
        .get(constants::HEADER_REGION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            CentralEnv::from_env_with_defaults("global", "development", "0.0.0-dev")
                .region
                .trim()
                .to_ascii_lowercase()
        });
    set_header(&mut req, constants::HEADER_REGION, &region);

    // Capture canonical values (for response echoing).
    let rid = req
        .headers()
        .get(constants::HEADER_MACH_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let cid = req
        .headers()
        .get(constants::HEADER_MACH_CORRELATION_ID)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let residency = req
        .headers()
        .get(constants::HEADER_RESIDENCY)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("GLOBAL")
        .to_string();

    let traceparent = req
        .headers()
        .get(constants::HEADER_TRACEPARENT)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    // Build RequestContext + insert (fail-open).
    let pairs: Vec<(String, String)> = req
        .headers()
        .iter()
        .filter_map(|(k, v)| Some((k.as_str().to_string(), v.to_str().ok()?.to_string())))
        .collect();

    let ctx = RequestContext::from_headers(pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .unwrap_or_else(|_| RequestContext::empty());
    req.extensions_mut().insert(ctx);

    let mut resp = next.run(req).await;

    // Echo canonical headers
    if let Some(rid) = rid {
        if let Ok(v) = HeaderValue::from_str(&rid) {
            resp.headers_mut().insert(
                HeaderName::from_static(constants::HEADER_MACH_REQUEST_ID),
                v,
            );
        }
    }
    if let Some(cid) = cid {
        if let Ok(v) = HeaderValue::from_str(&cid) {
            resp.headers_mut().insert(
                HeaderName::from_static(constants::HEADER_MACH_CORRELATION_ID),
                v,
            );
        }
    }

    if let Ok(v) = HeaderValue::from_str(&region) {
        resp.headers_mut()
            .insert(HeaderName::from_static(constants::HEADER_REGION), v);
    }

    if let Ok(v) = HeaderValue::from_str(&residency) {
        resp.headers_mut()
            .insert(HeaderName::from_static(constants::HEADER_RESIDENCY), v);
    }

    if let Some(tp) = traceparent {
        if let Ok(v) = HeaderValue::from_str(&tp) {
            resp.headers_mut()
                .insert(HeaderName::from_static(constants::HEADER_TRACEPARENT), v);
        }
    }

    resp
}

fn set_header(req: &mut Request<Body>, name: &'static str, value: impl AsRef<str>) {
    if let Ok(v) = HeaderValue::from_str(value.as_ref()) {
        req.headers_mut().insert(HeaderName::from_static(name), v);
    }
}

#[cfg(test)]
mod contract_v60_tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::{
        middleware::from_fn, response::IntoResponse, routing::get, Extension, Json, Router,
    };
    use rhelma_core::RequestContext;
    use serde_json::json;
    use tower::ServiceExt;
    use uuid::Uuid;

    fn test_app() -> Router {
        async fn handler(Extension(ctx): Extension<RequestContext>) -> impl IntoResponse {
            Json(json!({
                "has_trace": ctx.trace().current_trace_id().is_some(),
            }))
        }

        Router::new()
            .route("/t", get(handler))
            .layer(from_fn(request_guard_middleware))
    }

    #[tokio::test]
    async fn missing_headers_are_normalized_and_echoed() {
        let app = test_app();

        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/t")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("resp");

        assert_eq!(resp.status(), StatusCode::OK);

        let rid = resp
            .headers()
            .get(constants::HEADER_MACH_REQUEST_ID)
            .and_then(|v| v.to_str().ok())
            .expect("x-rhelma-request-id");
        let cid = resp
            .headers()
            .get(constants::HEADER_MACH_CORRELATION_ID)
            .and_then(|v| v.to_str().ok())
            .expect("x-rhelma-correlation-id");
        let tp = resp
            .headers()
            .get(constants::HEADER_TRACEPARENT)
            .and_then(|v| v.to_str().ok())
            .expect("traceparent");
        let region = resp
            .headers()
            .get(constants::HEADER_REGION)
            .and_then(|v| v.to_str().ok())
            .expect("x-rhelma-region");
        let residency = resp
            .headers()
            .get(constants::HEADER_RESIDENCY)
            .and_then(|v| v.to_str().ok())
            .expect("x-residency");

        let rid_u = Uuid::parse_str(rid).expect("rid uuid");
        let cid_u = Uuid::parse_str(cid).expect("cid uuid");
        assert_eq!(rid_u, cid_u, "cid should default to rid when missing");

        assert!(
            tp.starts_with("00-") && tp.len() >= 55,
            "traceparent should be valid-ish: {tp}"
        );
        assert!(!region.trim().is_empty());
        assert!(!residency.trim().is_empty());
    }

    #[tokio::test]
    async fn invalid_request_id_is_canonicalized() {
        let app = test_app();

        let req = axum::http::Request::builder()
            .uri("/t")
            .header(constants::HEADER_MACH_REQUEST_ID, "not-a-uuid")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.expect("resp");
        assert_eq!(resp.status(), StatusCode::OK);

        let rid = resp
            .headers()
            .get(constants::HEADER_MACH_REQUEST_ID)
            .and_then(|v| v.to_str().ok())
            .expect("x-rhelma-request-id");
        let _ = Uuid::parse_str(rid).expect("rid uuid");
    }

    #[tokio::test]
    async fn invalid_traceparent_is_replaced_with_valid() {
        let app = test_app();

        let req = axum::http::Request::builder()
            .uri("/t")
            .header(constants::HEADER_TRACEPARENT, "nope")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.expect("resp");
        assert_eq!(resp.status(), StatusCode::OK);

        let tp = resp
            .headers()
            .get(constants::HEADER_TRACEPARENT)
            .and_then(|v| v.to_str().ok())
            .expect("traceparent echoed/generated");

        assert!(
            tp.starts_with("00-") && tp.len() >= 55,
            "traceparent should be valid-ish: {tp}"
        );
    }
}
