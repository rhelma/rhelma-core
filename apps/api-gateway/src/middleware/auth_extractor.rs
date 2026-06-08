#![forbid(unsafe_code)]

use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
    Extension,
};
use redis::AsyncCommands;
use rhelma_auth::{AuthService, UserPrincipal};
use std::sync::Arc;

use crate::error::GatewayError;
use crate::state::AppState;

#[derive(Clone, Debug)]
pub struct AuthUserExtractor(pub UserPrincipal);

/// Authenticated user principal extracted from the Authorization header.
///
/// This is kept for backwards compatibility with older route signatures.
#[derive(Clone, Debug)]
pub struct AuthPrincipal(pub UserPrincipal);

#[derive(Clone, Debug)]
pub struct OptionalAuthUserExtractor(pub Option<UserPrincipal>);

impl<S> FromRequestParts<S> for AuthUserExtractor
where
    S: Send + Sync,
{
    type Rejection = GatewayError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Extension(auth) = Extension::<Arc<AuthService>>::from_request_parts(parts, state)
            .await
            .map_err(|_| GatewayError::internal("missing auth service"))?;

        let Extension(app_state) = Extension::<Arc<AppState>>::from_request_parts(parts, state)
            .await
            .map_err(|_| GatewayError::internal("missing app state"))?;

        let token = extract_bearer(parts.headers.get(header::AUTHORIZATION))
            .ok_or_else(|| unauthorized("missing bearer token"))?;

        let principal = auth
            .verify_access_token(&token)
            .await
            .map_err(|_| unauthorized("invalid or expired token"))?;

        // Token revocation checks (Redis-backed)
        if is_revoked(&app_state, &token, &principal).await {
            return Err(unauthorized("token revoked"));
        }

        parts.extensions.insert(principal.clone());
        Ok(Self(principal))
    }
}

impl<S> FromRequestParts<S> for AuthPrincipal
where
    S: Send + Sync,
{
    type Rejection = GatewayError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let AuthUserExtractor(p) = AuthUserExtractor::from_request_parts(parts, state).await?;
        Ok(Self(p))
    }
}

impl<S> FromRequestParts<S> for OptionalAuthUserExtractor
where
    S: Send + Sync,
{
    type Rejection = GatewayError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth = match Extension::<Arc<AuthService>>::from_request_parts(parts, state).await {
            Ok(Extension(auth)) => auth,
            Err(_) => return Ok(Self(None)),
        };

        let app_state = match Extension::<Arc<AppState>>::from_request_parts(parts, state).await {
            Ok(Extension(s)) => Some(s),
            Err(_) => None,
        };

        let token = match extract_bearer(parts.headers.get(header::AUTHORIZATION)) {
            Some(t) => t,
            None => return Ok(Self(None)),
        };

        match auth.verify_access_token(&token).await {
            Ok(principal) => {
                if let Some(app_state) = app_state {
                    if is_revoked(&app_state, &token, &principal).await {
                        return Ok(Self(None));
                    }
                }
                parts.extensions.insert(principal.clone());
                Ok(Self(Some(principal)))
            }
            Err(_) => Ok(Self(None)),
        }
    }
}

async fn is_revoked(app_state: &Arc<AppState>, token: &str, principal: &UserPrincipal) -> bool {
    let fp = token_fingerprint(token);
    let key_token = format!("revoke:token:{fp}");
    let key_user = format!("revoke:user:{}", principal.user_id);

    let mut con = app_state.redis.clone();

    let revoked_token: bool = con.exists(&key_token).await.unwrap_or(false);
    let revoked_user: bool = con.exists(&key_user).await.unwrap_or(false);

    revoked_token || revoked_user
}

fn token_fingerprint(token: &str) -> String {
    let hash = blake3::hash(token.as_bytes());
    hex::encode(hash.as_bytes())
}

fn extract_bearer(h: Option<&axum::http::HeaderValue>) -> Option<String> {
    let v = h?.to_str().ok()?.trim();
    let (scheme, rest) = v.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let token = rest.trim();
    if token.is_empty() {
        return None;
    }
    Some(token.to_string())
}

fn unauthorized(msg: &str) -> GatewayError {
    // NOTE: body will be replaced by the global v5.2 envelope middleware.
    // This error is only used for stable status + internal tagging.
    GatewayError::unauthorized(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn extract_bearer_parses_token() {
        let hv = axum::http::HeaderValue::from_static("Bearer abc.def.ghi");
        assert_eq!(extract_bearer(Some(&hv)).as_deref(), Some("abc.def.ghi"));
    }

    #[tokio::test]
    async fn extract_bearer_accepts_lowercase() {
        let hv = axum::http::HeaderValue::from_static("bearer token123");
        assert_eq!(extract_bearer(Some(&hv)).as_deref(), Some("token123"));
    }

    #[tokio::test]
    async fn extract_bearer_rejects_non_bearer() {
        let hv = axum::http::HeaderValue::from_static("Basic abc");
        assert!(extract_bearer(Some(&hv)).is_none());
    }
}
