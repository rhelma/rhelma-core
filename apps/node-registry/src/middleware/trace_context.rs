#![forbid(unsafe_code)]

use axum::{body::Body, http::Request, middleware::Next, response::Response};

use rhelma_http_observability::extract_minimal_headers;
use rhelma_tracing::context::scope_with_headers;

pub async fn trace_context_middleware(req: Request<Body>, next: Next) -> Response {
    let h = extract_minimal_headers(req.headers());
    scope_with_headers(&h, async move { next.run(req).await }).await
}
