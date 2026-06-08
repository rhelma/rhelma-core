#![forbid(unsafe_code)]

use axum::{
    body::Body,
    http::{header, HeaderMap, HeaderValue, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use uuid::Uuid;

use crate::error::X_MACH_ERROR_TYPE;

/// Wrap all error responses (status >= 400) into Rhelma v5.2 error envelope.
///
/// Notes:
/// - Runs outside `request_guard_middleware` (outer layer), so it can rely on echoed headers.
/// - Still fails closed if those headers are missing by generating fresh IDs (never "unknown").
/// - Uses `rhelma_core::error_v52::envelope_from_status(...)`.
pub async fn error_envelope_middleware(req: Request<Body>, next: Next) -> Response {
    let resp = next.run(req).await;

    let status = resp.status();
    if !(status.is_client_error() || status.is_server_error()) {
        return resp;
    }

    let orig_headers = resp.headers().clone();

    let request_id = parse_uuid_any(&orig_headers, &["x-rhelma-request-id"])
        .unwrap_or_else(Uuid::now_v7)
        .to_string();

    let correlation_id = parse_uuid_any(&orig_headers, &["x-rhelma-correlation-id"])
        .unwrap_or_else(|| Uuid::parse_str(&request_id).unwrap_or_else(|_| Uuid::now_v7()))
        .to_string();

    let traceparent = orig_headers
        .get("traceparent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Prefer typed mapping from ApiError / middleware that tags the response.
    let type_label = orig_headers
        .get(X_MACH_ERROR_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| infer_type_label_from_status(status).to_string());

    // Optionally map status when middleware tagged the response.
    // (keeps envelope consistent for rate-limit/timeout/residency signals)
    let http_status_u16 = map_tag_to_status(&type_label).unwrap_or(status.as_u16());
    let mapped_status =
        StatusCode::from_u16(http_status_u16).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    // Minimal, stable message; avoid parsing original body here.
    let message = mapped_status
        .canonical_reason()
        .unwrap_or("HTTP_ERROR")
        .to_string();

    // Build a minimal RequestContext so envelope IDs match the echoed headers.
    let mut hdrs: Vec<(String, String)> = vec![
        ("x-rhelma-request-id".into(), request_id.clone()),
        ("x-rhelma-correlation-id".into(), correlation_id.clone()),
    ];

    if let Some(tp) = traceparent.as_deref() {
        hdrs.push(("traceparent".into(), tp.to_string()));
    }

    if let Some(r) = orig_headers
        .get("x-residency")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
    {
        hdrs.push(("x-residency".into(), r));
    }

    let ctx = rhelma_core::RequestContext::from_headers(
        hdrs.iter().map(|(k, v)| (k.as_str(), v.as_str())),
    )
    .ok();

    let envelope =
        rhelma_core::error_v52::envelope_from_status(http_status_u16, message, ctx.as_ref());

    let mut new_resp = (mapped_status, Json(envelope)).into_response();

    // Preserve headers (CORS, tracing, request ids, etc.).
    // Skip content headers because we replace the body.
    // Skip internal error tag headers.
    copy_headers(&orig_headers, new_resp.headers_mut());

    // Ensure request/correlation IDs are present even if upstream didn't set them.
    if new_resp.headers().get("x-rhelma-request-id").is_none() {
        if let Ok(v) = HeaderValue::from_str(&request_id) {
            new_resp.headers_mut().insert("x-rhelma-request-id", v);
        }
    }
    if new_resp.headers().get("x-rhelma-correlation-id").is_none() {
        if let Ok(v) = HeaderValue::from_str(&correlation_id) {
            new_resp.headers_mut().insert("x-rhelma-correlation-id", v);
        }
    }

    new_resp
}

fn infer_type_label_from_status(status: StatusCode) -> &'static str {
    match status {
        StatusCode::BAD_REQUEST => "bad_request",
        StatusCode::UNAUTHORIZED => "auth",
        StatusCode::FORBIDDEN => "authz",
        StatusCode::NOT_FOUND => "not_found",
        StatusCode::CONFLICT => "conflict",
        StatusCode::TOO_MANY_REQUESTS => "rate_limited",
        StatusCode::BAD_GATEWAY => "dependency",
        StatusCode::SERVICE_UNAVAILABLE => "dependency",
        StatusCode::GATEWAY_TIMEOUT => "timeout",
        StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS => {
            rhelma_core::RhelmaError::RESIDENCY_VIOLATION_CODE
        }
        _ => "internal",
    }
}

fn map_tag_to_status(tag: &str) -> Option<u16> {
    match tag.trim().to_ascii_lowercase().as_str() {
        "rate_limited" | "too_many_requests" => Some(429),
        "timeout" => Some(504),
        "service_unavailable" | "dependency" => Some(503),
        x if x == rhelma_core::RhelmaError::RESIDENCY_VIOLATION_CODE => Some(451),
        _ => None,
    }
}

fn parse_uuid_any(headers: &HeaderMap, keys: &[&str]) -> Option<Uuid> {
    for k in keys {
        let raw = headers.get(*k).and_then(|v| v.to_str().ok())?.trim();
        if raw.is_empty() {
            continue;
        }
        if let Ok(u) = Uuid::parse_str(raw) {
            return Some(u);
        }
    }
    None
}

fn copy_headers(from: &HeaderMap, to: &mut HeaderMap) {
    for (k, v) in from.iter() {
        // Body-related headers become invalid once we replace the body.
        if k == header::CONTENT_LENGTH || k == header::CONTENT_TYPE {
            continue;
        }
        // Internal tagging header; not part of the public contract.
        if k == X_MACH_ERROR_TYPE {
            continue;
        }
        // Don't overwrite if already set by the new response.
        if to.get(k).is_none() {
            to.insert(k, v.clone());
        }
    }
}
