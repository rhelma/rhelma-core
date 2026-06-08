//! Redis-backed rate limiting (fixed window).
//!
//! This is a minimal, safe baseline. For advanced token bucket, add Lua script.
//! Contract: no in-memory limiter for enterprise; Redis provides distributed consistency.

use std::task::{Context, Poll};
use std::time::Instant;

use crate::error::AuthError;
use crate::tracing_ext::auth_span;
use futures_util::future::BoxFuture;
use http::{Request, Response, StatusCode};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use tower::{Layer, Service};

/// Rate limit configuration.
#[derive(Clone)]
/// struct (documented for contract compliance).
pub struct RateLimitConfig {
    /// Max requests in window.
    pub max: u64,
    /// Window seconds.
    pub window_secs: u64,
    /// Redis key prefix (should match auth prefix policy).
    pub prefix: String,
}

#[derive(Clone)]
/// struct (documented for contract compliance).
pub struct RedisRateLimitLayer {
    conn: ConnectionManager,
    cfg: RateLimitConfig,
}

impl RedisRateLimitLayer {
    /// Create new rate limiter layer.
    pub fn new(conn: ConnectionManager, cfg: RateLimitConfig) -> Self {
        Self { conn, cfg }
    }
}

impl<S> Layer<S> for RedisRateLimitLayer {
    type Service = RedisRateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RedisRateLimitService {
            inner,
            conn: self.conn.clone(),
            cfg: self.cfg.clone(),
        }
    }
}

#[derive(Clone)]
/// struct (documented for contract compliance).
pub struct RedisRateLimitService<S> {
    inner: S,
    conn: ConnectionManager,
    cfg: RateLimitConfig,
}

impl<S, B> Service<Request<B>> for RedisRateLimitService<S>
where
    S: Service<Request<B>, Response = Response<B>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    B: Default + Send + 'static,
{
    type Response = Response<B>;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Response<B>, S::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), S::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let mut inner = self.inner.clone();
        let mut conn = self.conn.clone();
        let cfg = self.cfg.clone();

        Box::pin(async move {
            // IMPORTANT: don't hold an entered span across `.await` (would break `Send`).
            let _span = auth_span("rate_limit");

            let key = rate_key(&cfg, &req);

            let start = Instant::now();
            let allowed = check_fixed_window(&mut conn, &key, cfg.max, cfg.window_secs).await;
            let _lat = start.elapsed().as_secs_f64();

            match allowed {
                Ok(true) => inner.call(req).await,
                Ok(false) => Ok(too_many::<B>()),
                Err(_) => Ok(service_unavailable::<B>()),
            }
        })
    }
}

fn rate_key<B>(cfg: &RateLimitConfig, req: &Request<B>) -> String {
    // Default: IP-based if present, else fallback to "anonymous".
    let ip = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "anonymous".to_string());

    format!("{}:rl:{}:{}", cfg.prefix, ip, req.uri().path())
}

async fn check_fixed_window(
    conn: &mut ConnectionManager,
    key: &str,
    max: u64,
    window_secs: u64,
) -> Result<bool, AuthError> {
    // INCR + EXPIRE pattern
    let count: u64 = conn.incr(key, 1).await?;
    if count == 1 {
        let _: () = conn.expire(key, window_secs as i64).await?;
    }
    Ok(count <= max)
}

fn too_many<B: Default>() -> Response<B> {
    let mut r = Response::new(B::default());
    *r.status_mut() = StatusCode::TOO_MANY_REQUESTS;
    r
}

fn service_unavailable<B: Default>() -> Response<B> {
    let mut r = Response::new(B::default());
    *r.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
    r
}
