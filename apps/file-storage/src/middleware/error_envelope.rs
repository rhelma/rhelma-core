#![forbid(unsafe_code)]

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};

use rhelma_core::{error_v52, RequestContext};

pub const X_MACH_ERROR_ENVELOPE: &str = "x-rhelma-error-envelope";

/// Wrap all non-JSON error responses (status >= 400) into a Rhelma v5.2 error envelope.
///
/// Notes:
/// - If a handler already produced a Rhelma envelope (JSON) or already tagged the response,
///   this middleware will not wrap again.
/// - Uses `RequestContext` if the request guard inserted it; otherwise generates without ctx.
pub async fn error_envelope_middleware(req: Request<Body>, next: Next) -> Response {
    let ctx = req.extensions().get::<RequestContext>().cloned();

    let resp = next.run(req).await;
    let status = resp.status();

    if !(status.is_client_error() || status.is_server_error()) {
        return resp;
    }

    // Avoid double-wrapping.
    if resp.headers().contains_key(X_MACH_ERROR_ENVELOPE) {
        return resp;
    }

    // If upstream already returned JSON, keep it (assume it's a Rhelma envelope).
    if let Some(ct) = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
    {
        if ct.starts_with("application/json") {
            return resp;
        }
    }

    let message = default_message_for_status(status);
    let envelope = error_v52::envelope_from_status(status.as_u16(), message, ctx.as_ref());

    let mut new_resp = (status, Json(envelope)).into_response();

    // Preserve non-body headers (CORS, request ids, traceparent, ...)
    for (k, v) in resp.headers().iter() {
        if k == header::CONTENT_TYPE || k == header::CONTENT_LENGTH {
            continue;
        }
        if new_resp.headers().get(k).is_none() {
            new_resp.headers_mut().insert(k, v.clone());
        }
    }

    new_resp.headers_mut().insert(
        X_MACH_ERROR_ENVELOPE,
        header::HeaderValue::from_static("v5.2"),
    );

    new_resp
}

fn default_message_for_status(status: StatusCode) -> &'static str {
    match status {
        StatusCode::BAD_REQUEST => "bad_request",
        StatusCode::UNAUTHORIZED => "auth",
        StatusCode::FORBIDDEN => "authz",
        StatusCode::NOT_FOUND => "not_found",
        StatusCode::CONFLICT => "conflict",
        StatusCode::TOO_MANY_REQUESTS => "rate_limited",
        StatusCode::BAD_GATEWAY | StatusCode::SERVICE_UNAVAILABLE | StatusCode::GATEWAY_TIMEOUT => {
            "dependency"
        }
        _ => "internal",
    }
}
