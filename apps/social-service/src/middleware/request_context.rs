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
/// Ensures Contract v6.0 headers, builds [`RequestContext`] from headers, and
/// echoes canonical IDs back on the response.
pub async fn request_context_middleware(mut req: Request<Body>, next: Next) -> Response {
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

    let pairs: Vec<(String, String)> = req
        .headers()
        .iter()
        .filter_map(|(k, v)| Some((k.as_str().to_string(), v.to_str().ok()?.to_string())))
        .collect();

    let ctx = RequestContext::from_headers(pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .unwrap_or_else(|_| RequestContext::empty());
    req.extensions_mut().insert(ctx);

    let mut resp = next.run(req).await;

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
