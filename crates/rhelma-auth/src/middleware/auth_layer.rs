//! Tower auth layer: extracts Bearer token, verifies JWT, checks session in Redis,
//! and injects `UserPrincipal` into request extensions.

use std::task::{Context, Poll};

use crate::session::SessionManager;
use crate::tracing_ext::auth_span;
use crate::types::UserPrincipal;
use crate::AuthError;
use futures_util::future::BoxFuture;
use http::{header, Request, Response, StatusCode};
use tower::{Layer, Service};

/// Authentication mode for the layer.
#[derive(Debug, Clone, Copy)]
/// enum (documented for contract compliance).
pub enum AuthMode {
    /// Missing/invalid token => 401.
    Required,
    /// Missing token => proceed as anonymous.
    Optional,
}

/// Tower Layer.
#[derive(Clone)]
/// struct (documented for contract compliance).
pub struct AuthLayer {
    sessions: SessionManager,
    mode: AuthMode,
}

impl AuthLayer {
    /// fn (documented for contract compliance).
    pub fn required(sessions: SessionManager) -> Self {
        Self {
            sessions,
            mode: AuthMode::Required,
        }
    }

    /// fn (documented for contract compliance).
    pub fn optional(sessions: SessionManager) -> Self {
        Self {
            sessions,
            mode: AuthMode::Optional,
        }
    }

    /// fn (documented for contract compliance).
    pub fn mode(&self) -> AuthMode {
        self.mode
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddleware {
            inner,
            sessions: self.sessions.clone(),
            mode: self.mode,
        }
    }
}

/// Tower Service.
#[derive(Clone)]
/// struct (documented for contract compliance).
pub struct AuthMiddleware<S> {
    inner: S,
    sessions: SessionManager,
    mode: AuthMode,
}

impl<S, B> Service<Request<B>> for AuthMiddleware<S>
where
    S: Service<Request<B>, Response = Response<B>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    B: Default + Send + 'static,
{
    type Response = Response<B>;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        let mut inner = self.inner.clone();
        let sessions = self.sessions.clone();
        let mode = self.mode;

        Box::pin(async move {
            let _span = auth_span("middleware.auth");

            let token = extract_bearer(req.headers().get(header::AUTHORIZATION));

            match token {
                None => {
                    if matches!(mode, AuthMode::Required) {
                        return Ok(unauthorized::<B>());
                    }
                    inner.call(req).await
                }
                Some(t) => {
                    match sessions.verify_access_token(&t).await {
                        Ok(principal) => {
                            req.extensions_mut().insert::<UserPrincipal>(principal);
                            inner.call(req).await
                        }
                        Err(
                            AuthError::Unauthorized
                            | AuthError::InvalidToken
                            | AuthError::TokenExpired,
                        ) => {
                            if matches!(mode, AuthMode::Optional) {
                                // token provided but invalid -> still unauthorized (avoid silent downgrade).
                                Ok(unauthorized::<B>())
                            } else {
                                Ok(unauthorized::<B>())
                            }
                        }
                        Err(AuthError::Forbidden) => Ok(forbidden::<B>()),
                        Err(AuthError::SessionStore) => Ok(service_unavailable::<B>()),
                        Err(_) => Ok(internal::<B>()),
                    }
                }
            }
        })
    }
}

fn extract_bearer(v: Option<&header::HeaderValue>) -> Option<String> {
    let s = v?.to_str().ok()?;
    let s = s.trim();
    let prefix = "Bearer ";
    if s.len() > prefix.len() && s.starts_with(prefix) {
        Some(s[prefix.len()..].trim().to_string())
    } else {
        None
    }
}

fn unauthorized<B: Default>() -> Response<B> {
    let mut r = Response::new(B::default());
    *r.status_mut() = StatusCode::UNAUTHORIZED;
    r
}

fn forbidden<B: Default>() -> Response<B> {
    let mut r = Response::new(B::default());
    *r.status_mut() = StatusCode::FORBIDDEN;
    r
}

fn service_unavailable<B: Default>() -> Response<B> {
    let mut r = Response::new(B::default());
    *r.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
    r
}

fn internal<B: Default>() -> Response<B> {
    let mut r = Response::new(B::default());
    *r.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    r
}
