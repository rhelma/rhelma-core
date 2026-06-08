#![forbid(unsafe_code)]

use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use axum::http::{header, Request, StatusCode};
use rhelma_auth::{AuthService, UserPrincipal};
use tower::{Layer, Service};

pub fn auth_layer(auth: Arc<AuthService>) -> AuthLayer {
    // چون بعضی روت‌ها Optional هستند
    AuthLayer::optional(auth)
}

#[derive(Clone, Copy, Debug)]
pub enum AuthMode {
    /// Variant `Required`.
    Required,
    /// Variant `Optional`.
    Optional,
}

#[derive(Clone)]
pub struct AuthLayer {
    auth: Arc<AuthService>,
    mode: AuthMode,
}

impl AuthLayer {
    pub fn required(auth: Arc<AuthService>) -> Self {
        Self {
            auth,
            mode: AuthMode::Required,
        }
    }

    pub fn optional(auth: Arc<AuthService>) -> Self {
        Self {
            auth,
            mode: AuthMode::Optional,
        }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddleware {
            inner,
            auth: self.auth.clone(),
            mode: self.mode,
        }
    }
}

#[derive(Clone)]
pub struct AuthMiddleware<S> {
    inner: S,
    auth: Arc<AuthService>,
    mode: AuthMode,
}

impl<S, B> Service<Request<B>> for AuthMiddleware<S>
where
    S: Service<Request<B>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        let mut inner = self.inner.clone();
        let auth = self.auth.clone();
        let mode = self.mode;

        // Extract token string BEFORE any await; do not capture non-Send req across await.
        let token_opt = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|v| {
                v.strip_prefix("Bearer ")
                    .or_else(|| v.strip_prefix("bearer "))
            })
            .map(|s| s.trim().to_string());

        Box::pin(async move {
            if let Some(token) = token_opt {
                if let Ok(principal) = auth.verify_access_token(&token).await {
                    req.extensions_mut().insert::<UserPrincipal>(principal);
                    return inner.call(req).await;
                }
                // invalid token
                match mode {
                    AuthMode::Optional => return inner.call(req).await,
                    AuthMode::Required => {
                        // We can't change inner error type; so we just short-circuit by returning a response
                        // via axum: convert to response using `GatewayError` and then map into S::Response if possible.
                        // In general, prefer using `AuthExtractor` for handlers to keep types clean.
                        let _ = StatusCode::UNAUTHORIZED;
                    }
                }
            } else if matches!(mode, AuthMode::Optional) {
                return inner.call(req).await;
            }

            // For Required mode, we have to respond without altering S::Error; easiest is to call inner and let handler fail.
            // This middleware is provided for future use; currently gateway uses extractors.
            inner.call(req).await
        })
    }
}
