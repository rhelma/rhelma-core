#![forbid(unsafe_code)]

use axum::{
    body::Body,
    http::{header::HeaderName, HeaderValue, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

use rhelma_core::{constants, trace_context::TraceContext, RequestContext};
use rhelma_http_observability::axum::ContractV60Diagnostics;
use uuid::Uuid;

pub async fn request_guard_middleware(mut req: Request<Body>, next: Next) -> Response {
    // ContractV60Layer runs *outside* this middleware and guarantees canonical headers.
    // Here we optionally fail-closed based on diagnostics about what the client supplied.
    if let Some(diag) = req.extensions().get::<ContractV60Diagnostics>() {
        if diag.request_id_invalid {
            return (StatusCode::BAD_REQUEST, "invalid x-rhelma-request-id").into_response();
        }
        if diag.correlation_id_invalid {
            return (StatusCode::BAD_REQUEST, "invalid x-rhelma-correlation-id").into_response();
        }
        if diag.residency_invalid {
            return (StatusCode::BAD_REQUEST, "invalid x-residency").into_response();
        }
        if diag.traceparent_invalid {
            return (StatusCode::BAD_REQUEST, "invalid traceparent").into_response();
        }
    }

    // ---------------------------------------------------------------------
    // Canonicalize / synthesize minimum request headers.
    //
    // In production, `ContractV60Layer` runs outside this middleware and will
    // canonicalize + generate the required identifiers. However, tests and some
    // in-process uses may install `request_guard_middleware` without the contract
    // layer. In that case, we still want a stable minimum header set.
    // ---------------------------------------------------------------------
    // Request ID:
    // - If missing/blank: generate.
    // - If present but malformed: hard fail (v5.2 contract tests expect 400).
    let rid = match req
        .headers()
        .get(constants::HEADER_MACH_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(raw) => match Uuid::parse_str(raw) {
            Ok(u) => u,
            Err(_) => {
                return (StatusCode::BAD_REQUEST, "invalid x-rhelma-request-id").into_response()
            }
        },
        None => Uuid::now_v7(),
    };

    // Correlation ID defaults to request ID when missing/blank.
    // If present but malformed: hard fail (ingress should never propagate garbage IDs).
    let cid = match req
        .headers()
        .get(constants::HEADER_MACH_CORRELATION_ID)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(raw) => match Uuid::parse_str(raw) {
            Ok(u) => u,
            Err(_) => {
                return (StatusCode::BAD_REQUEST, "invalid x-rhelma-correlation-id").into_response()
            }
        },
        None => rid,
    };

    // Ensure canonical headers exist on the request before building RequestContext.
    if let Ok(v) = HeaderValue::from_str(&rid.to_string()) {
        req.headers_mut().insert(
            HeaderName::from_static(constants::HEADER_MACH_REQUEST_ID),
            v,
        );
    }
    if let Ok(v) = HeaderValue::from_str(&cid.to_string()) {
        req.headers_mut().insert(
            HeaderName::from_static(constants::HEADER_MACH_CORRELATION_ID),
            v,
        );
    }

    let residency_norm = req
        .headers()
        .get(constants::HEADER_RESIDENCY)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "GLOBAL".to_string());

    // Ensure a residency is present for in-process (no contract layer) usage.
    if req.headers().get(constants::HEADER_RESIDENCY).is_none() {
        if let Ok(v) = HeaderValue::from_str(&residency_norm) {
            req.headers_mut()
                .insert(HeaderName::from_static(constants::HEADER_RESIDENCY), v);
        }
    }

    let traceparent = req
        .headers()
        .get(constants::HEADER_TRACEPARENT)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    // Canonicalize / generate a valid W3C traceparent.
    let traceparent = match traceparent {
        Some(tp) if TraceContext::from_traceparent(&tp).is_some() => Some(tp),
        _ => TraceContext::generate().to_traceparent(),
    };

    if let Some(tp) = traceparent.as_ref() {
        if req.headers().get(constants::HEADER_TRACEPARENT).is_none() {
            if let Ok(v) = HeaderValue::from_str(tp) {
                req.headers_mut()
                    .insert(HeaderName::from_static(constants::HEADER_TRACEPARENT), v);
            }
        }
    }

    // ---------------------------------------------------------------------
    // 5) build RequestContext + insert
    // ---------------------------------------------------------------------
    let pairs: Vec<(String, String)> = req
        .headers()
        .iter()
        .filter_map(|(k, v)| Some((k.as_str().to_string(), v.to_str().ok()?.to_string())))
        .collect();

    let ctx =
        match RequestContext::from_headers(pairs.iter().map(|(k, v)| (k.as_str(), v.as_str()))) {
            Ok(c) => c,
            Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
        };
    req.extensions_mut().insert(ctx);

    let mut resp = next.run(req).await;

    // ---------------------------------------------------------------------
    // 6) echo canonical headers
    // ---------------------------------------------------------------------
    // Don't unwrap on header value creation in gateway code paths.
    if let Ok(v) = HeaderValue::from_str(&rid.to_string()) {
        resp.headers_mut().insert(
            HeaderName::from_static(constants::HEADER_MACH_REQUEST_ID),
            v,
        );
    }
    if let Ok(v) = HeaderValue::from_str(&cid.to_string()) {
        resp.headers_mut().insert(
            HeaderName::from_static(constants::HEADER_MACH_CORRELATION_ID),
            v,
        );
    }

    resp.headers_mut().insert(
        HeaderName::from_static(constants::HEADER_RESIDENCY),
        HeaderValue::from_str(&residency_norm)
            .unwrap_or_else(|_| HeaderValue::from_static("GLOBAL")),
    );

    // Echo W3C traceparent as well so downstream callers can correlate across hops.
    if let Some(tp) = traceparent {
        if let Ok(v) = HeaderValue::from_str(&tp) {
            resp.headers_mut()
                .insert(HeaderName::from_static(constants::HEADER_TRACEPARENT), v);
        }
    }

    resp
}

#[cfg(test)]
mod contract_v60_tests {
    use super::*;
    use axum::{middleware::from_fn, routing::get, Json, Router};
    use serde_json::json;
    use tower::ServiceExt;
    use uuid::Uuid;

    fn test_app() -> Router {
        async fn handler(headers: axum::http::HeaderMap) -> Json<serde_json::Value> {
            let tp = headers
                .get(constants::HEADER_TRACEPARENT)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            Json(json!({ "traceparent": tp }))
        }

        Router::new()
            .route("/t", get(handler))
            .layer(from_fn(request_guard_middleware))
            // Contract layer is outermost and provides canonicalization + diagnostics.
            .layer(rhelma_http_observability::axum::ContractV60Layer)
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
        let residency = resp
            .headers()
            .get(constants::HEADER_RESIDENCY)
            .and_then(|v| v.to_str().ok())
            .expect("x-residency");
        let tp = resp
            .headers()
            .get(constants::HEADER_TRACEPARENT)
            .and_then(|v| v.to_str().ok())
            .expect("traceparent");

        let rid_u = Uuid::parse_str(rid).expect("rid uuid");
        let cid_u = Uuid::parse_str(cid).expect("cid uuid");
        assert_eq!(rid_u, cid_u, "cid should default to rid when missing");
        assert_eq!(residency, "GLOBAL");
        assert!(
            tp.starts_with("00-") && tp.len() >= 55,
            "traceparent looks valid"
        );
    }

    #[tokio::test]
    async fn invalid_request_id_is_rejected() {
        let app = test_app();

        let req = axum::http::Request::builder()
            .uri("/t")
            .header(constants::HEADER_MACH_REQUEST_ID, "not-a-uuid")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.expect("resp");
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
