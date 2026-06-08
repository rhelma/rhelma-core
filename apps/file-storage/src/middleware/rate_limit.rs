#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::time::{Duration, Instant};

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};

use rhelma_core::RequestContext;

use crate::config::FileStorageConfig;

/// Simple in-memory token bucket rate limiter.
///
/// Notes:
/// - This is a *defense-in-depth* limiter for a single instance.
/// - Global/enforced limits should still be applied at the edge/gateway.
/// - Keys are derived from `tenant_id` when available, otherwise client IP headers.
#[derive(Debug)]
pub struct RateLimiter {
    inner: tokio::sync::Mutex<HashMap<String, Bucket>>,
}

#[derive(Debug, Clone)]
struct Bucket {
    tokens: f64,
    last: Instant,
    last_seen: Instant,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            inner: tokio::sync::Mutex::new(HashMap::new()),
        }
    }

    pub async fn allow(&self, key: &str, capacity: u32, refill_per_sec: f64) -> bool {
        let now = Instant::now();
        let mut map = self.inner.lock().await;

        // Opportunistic cleanup to prevent unbounded growth.
        // Purge buckets not seen in the last 10 minutes when the map becomes large.
        if map.len() > 10_000 {
            let max_age = Duration::from_secs(600);
            map.retain(|_, b| {
                now.checked_duration_since(b.last_seen)
                    .map(|age| age <= max_age)
                    .unwrap_or(true)
            });
        }

        let b = map.entry(key.to_string()).or_insert(Bucket {
            tokens: capacity as f64,
            last: now,
            last_seen: now,
        });

        let elapsed = now.duration_since(b.last).as_secs_f64();
        b.last = now;
        b.last_seen = now;

        // Refill.
        b.tokens = (b.tokens + elapsed * refill_per_sec).min(capacity as f64);

        if b.tokens >= 1.0 {
            b.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

#[derive(serde::Serialize)]
struct RateLimitedBody {
    error: &'static str,
    retry_after_seconds: u64,
}

pub async fn rate_limit_middleware(req: Request<Body>, next: Next) -> Response {
    let cfg = match req.extensions().get::<std::sync::Arc<FileStorageConfig>>() {
        Some(c) => c.clone(),
        None => return next.run(req).await, // should not happen
    };

    let limiter = match req.extensions().get::<std::sync::Arc<RateLimiter>>() {
        Some(l) => l.clone(),
        None => return next.run(req).await,
    };

    let ctx = req.extensions().get::<RequestContext>();

    let method = req.method().clone();
    let path = req.uri().path().to_string();

    // Key preference: tenant_id -> user_id -> ip -> anonymous.
    let base_key = if let Some(t) = ctx.and_then(|c| c.tenant_id()) {
        format!("tenant:{}", t.as_str())
    } else if let Some(u) = ctx.and_then(|c| c.user_id()) {
        format!("user:{:?}", u)
    } else if let Some(ip) = client_ip(&req) {
        format!("ip:{ip}")
    } else {
        "anonymous".to_string()
    };

    // Apply a stricter bucket for uploads.
    let is_upload = method == axum::http::Method::POST && path.starts_with("/v1/files");

    let (rpm, burst) = if method == axum::http::Method::GET || method == axum::http::Method::HEAD {
        (cfg.rate_limit_read_rpm, cfg.rate_limit_burst.max(1))
    } else if is_upload {
        // Uploads are more expensive; cap at write rpm but clamp burst.
        (cfg.rate_limit_write_rpm, cfg.rate_limit_burst.clamp(1, 30))
    } else {
        (cfg.rate_limit_write_rpm, cfg.rate_limit_burst.max(1))
    };

    let refill_per_sec = (rpm as f64) / 60.0;

    let key = format!(
        "{}:{}:{}",
        base_key,
        method.as_str(),
        if is_upload { "upload" } else { "default" }
    );

    if !limiter.allow(&key, burst, refill_per_sec).await {
        let mut resp = (
            StatusCode::TOO_MANY_REQUESTS,
            Json(RateLimitedBody {
                error: "rate_limited",
                retry_after_seconds: 1,
            }),
        )
            .into_response();

        resp.headers_mut().insert(
            axum::http::header::RETRY_AFTER,
            axum::http::HeaderValue::from_static("1"),
        );

        return resp;
    }

    next.run(req).await
}

fn client_ip(req: &Request<Body>) -> Option<String> {
    // Prefer CDN/proxy headers.
    for k in [
        "cf-connecting-ip",
        "x-client-ip",
        "x-real-ip",
        "x-forwarded-for",
    ] {
        if let Some(v) = req.headers().get(k).and_then(|v| v.to_str().ok()) {
            let raw = v.trim();
            if raw.is_empty() {
                continue;
            }
            // x-forwarded-for may contain a list.
            let first = raw.split(',').next().unwrap_or(raw).trim();
            if !first.is_empty() {
                return Some(first.to_string());
            }
        }
    }
    None
}
