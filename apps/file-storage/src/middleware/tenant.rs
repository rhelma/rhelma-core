#![forbid(unsafe_code)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use rhelma_core::RequestContext;

/// Tenant middleware.
///
/// Enforces that every request carries a valid tenant id in the `RequestContext`.
/// This makes tenant isolation a default property and prevents accidental cross-tenant access.
///
/// Notes:
/// - `request_guard` is responsible for building `RequestContext` from headers.
/// - This layer should run *after* `request_guard`.
pub async fn tenant_middleware(mut req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let tenant_id = {
        let ctx = req
            .extensions()
            .get::<RequestContext>()
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let tenant_id = ctx.tenant_id().ok_or(StatusCode::UNAUTHORIZED)?;
        if tenant_id.as_str().is_empty() {
            return Err(StatusCode::FORBIDDEN);
        }
        tenant_id.clone()
    };

    // Make tenant_id easy to access for downstream handlers without re-parsing context.
    req.extensions_mut().insert(tenant_id);

    Ok(next.run(req).await)
}
