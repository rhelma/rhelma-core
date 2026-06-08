#![forbid(unsafe_code)]

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    Json,
};
use rhelma_auth::UserPrincipal;
use serde::Serialize;

/// Marker extractor: ensures the current authenticated principal has `required` permission.
///
/// Usage pattern (per-route):
/// - attach required permission into extensions via `Extension(RequirePermission("perm".into()))`
/// - then use `axum::middleware::from_extractor::<RequirePermission>()`
///
/// This keeps authorization policy close to routing.
#[derive(Debug, Clone)]
pub struct RequirePermission(pub String);

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Field `code`.
    pub code: String,
    /// Field `message`.
    pub message: String,
}

impl<S> FromRequestParts<S> for RequirePermission
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<ErrorResponse>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Required permission must be set by a route layer.
        let required = parts
            .extensions
            .get::<RequirePermission>()
            .ok_or_else(|| unauthorized("missing required permission"))?
            .0
            .clone();

        // AuthExtractor inserts the principal into extensions.
        let principal = parts
            .extensions
            .get::<UserPrincipal>()
            .ok_or_else(|| unauthorized("not authenticated"))?;

        let ok = principal.permissions.iter().any(|p| p.0 == required);
        if ok {
            Ok(Self(required))
        } else {
            Err(forbidden("insufficient permissions"))
        }
    }
}

fn unauthorized(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            code: "unauthorized".into(),
            message: msg.into(),
        }),
    )
}

fn forbidden(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse {
            code: "forbidden".into(),
            message: msg.into(),
        }),
    )
}
