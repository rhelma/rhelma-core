#![forbid(unsafe_code)]

use axum::{
    body::Body,
    http::{HeaderName, HeaderValue, Request},
    middleware::Next,
    response::Response,
};

use rhelma_core::{constants, RequestContext};
use rhelma_http_observability::axum::ensure_contract_v60_headers;

/// RequestContext injection middleware.
///
/// Responsibilities:
/// - Ensure Contract v6.0 headers exist (idempotent, fail-open).
/// - Build canonical [`RequestContext`] from headers and insert into request extensions.
/// - Echo canonical request/correlation IDs on the response.
///
/// Notes:
/// - Upstream gateways (e.g. `api-gateway`) already run a stricter request guard.
///   This middleware keeps `search-service` safe for direct/internal use and ensures
///   a consistent contract for analytics + error mapping.
pub async fn request_context_middleware(mut req: Request<Body>, next: Next) -> Response {
    // Ensure minimum Contract v6.0 headers exist. This is idempotent and fail-open.
    // (ContractV60Layer also does this globally in most services.)
    ensure_contract_v60_headers(req.headers_mut());

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

    // ---------------------------------------------------------------
    // 3) Build RequestContext from canonical headers
    // ---------------------------------------------------------------
    let pairs: Vec<(String, String)> = req
        .headers()
        .iter()
        .filter_map(|(k, v)| Some((k.as_str().to_string(), v.to_str().ok()?.to_string())))
        .collect();

    let ctx = RequestContext::from_headers(pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .unwrap_or_else(|_| RequestContext::empty());
    req.extensions_mut().insert(ctx);

    // ---------------------------------------------------------------
    // 4) Execute handler
    // ---------------------------------------------------------------
    let mut resp = next.run(req).await;

    // ---------------------------------------------------------------
    // 5) Echo canonical IDs
    // ---------------------------------------------------------------
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

    resp
}

// NOTE: we intentionally avoid helper setters here to keep the middleware trivial.

#[cfg(test)]
mod contract_v60_tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::{
        middleware::from_fn, response::IntoResponse, routing::get, Extension, Json, Router,
    };
    use rhelma_core::RequestContext;
    use serde_json::json;
    use tower::ServiceExt; // for `oneshot`
    use uuid::Uuid;

    fn test_app() -> Router {
        async fn handler(Extension(ctx): Extension<RequestContext>) -> impl IntoResponse {
            Json(json!({
                "has_trace": ctx.trace().current_trace_id().is_some(),
            }))
        }

        Router::new()
            .route("/t", get(handler))
            .layer(from_fn(request_context_middleware))
    }

    #[tokio::test]
    async fn missing_request_id_is_accepted_and_generated_and_echoed() {
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

        let rid_u = Uuid::parse_str(rid).expect("rid uuid");
        let cid_u = Uuid::parse_str(cid).expect("cid uuid");
        assert_eq!(rid_u, cid_u, "cid should default to rid when missing");
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
        let cid = resp
            .headers()
            .get(constants::HEADER_MACH_CORRELATION_ID)
            .and_then(|v| v.to_str().ok())
            .expect("x-rhelma-correlation-id");

        let rid_u = Uuid::parse_str(rid).expect("rid uuid");
        let cid_u = Uuid::parse_str(cid).expect("cid uuid");
        assert_eq!(rid_u, cid_u, "cid defaults to rid");
    }

    #[tokio::test]
    async fn traceparent_is_parsed_into_context_when_present() {
        let app = test_app();

        // valid W3C traceparent: version-traceid-spanid-flags
        let req = axum::http::Request::builder()
            .uri("/t")
            .header(
                constants::HEADER_TRACEPARENT,
                "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
            )
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.expect("resp");
        assert_eq!(resp.status(), StatusCode::OK);

        // Read body JSON and ensure has_trace=true
        let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(v["has_trace"], true);
    }
}
