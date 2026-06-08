use axum::{http::Request, middleware::Next, response::Response};
use rhelma_core::prelude::*;

/// Authentication layer for file-storage.
///
/// NOTE(v5.2): `file-storage` is intended to be deployed behind `api-gateway`, where
/// rhelma-auth verification is enforced (edge → gateway → service). This middleware
/// intentionally remains a **pass-through** to avoid duplicating auth logic and to
/// keep the service usable for internal jobs/tests.
///
/// If you need direct token verification (e.g. exposing file-storage publicly), implement
/// it behind a feature flag and enable it only in the appropriate tier.
pub fn auth_layer<S>() -> axum::middleware::from_fn::FromFnLayer<fn(Request<S>, Next<S>) -> _> {
    axum::middleware::from_fn(auth_middleware)
}

async fn auth_middleware<B>(req: Request<B>, next: Next<B>) -> Result<Response, RhelmaError> {
    Ok(next.run(req).await)
}
