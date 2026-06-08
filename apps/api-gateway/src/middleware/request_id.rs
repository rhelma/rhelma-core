#![forbid(unsafe_code)]

use axum::{
    http::HeaderValue,
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

pub const REQUEST_ID_HEADER: &str = "x-rhelma-request-id";

pub async fn request_id_middleware(
    mut req: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Response {
    let request_id = req
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::now_v7().to_string());

    // Make available to handlers/extractors
    req.extensions_mut().insert(request_id.clone());

    // Put request_id into the current tracing context for correlation.
    // Note: `tracing::Span` does not expose extensions; we rely on span fields + response header.
    let span = tracing::info_span!("request", request_id = %request_id);
    let _enter = span.enter();
    let mut resp = next.run(req).await;

    resp.headers_mut().insert(
        REQUEST_ID_HEADER,
        HeaderValue::from_str(&request_id).unwrap(),
    );

    resp
}
