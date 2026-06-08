#![forbid(unsafe_code)]
use axum::body::Body;

// Trace-context binding middleware.
//
// Binds a minimal header allow-list into `rhelma-tracing` task-local context so
// spans/logs downstream automatically include request/correlation/trace IDs.
use axum::{http::Request, middleware::Next, response::Response};

use rhelma_http_observability::extract_minimal_headers;
use rhelma_tracing::context::scope_with_headers;

pub async fn trace_context_middleware(req: Request<Body>, next: Next) -> Response {
    let h = extract_minimal_headers(req.headers());
    scope_with_headers(&h, async move { next.run(req).await }).await
}
