#![forbid(unsafe_code)]

use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderValue, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use redis::Script;
use rhelma_core::{RateLimitKeyBuilder, RequestContext};
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{error::X_MACH_ERROR_TYPE, state::AppState};

/// Redis-backed *sliding-window* rate limiting (default: 60s).
///
/// Keying strategy (fail-closed):
/// 1) If `RequestContext` has tenant/user/region -> use canonical `RateLimitKeyBuilder`
/// 2) Else if Authorization bearer exists -> token fingerprint
/// 3) Else if client IP exists -> ip
/// 4) Else -> shared "anon" bucket (still rate limited)
///
/// Algorithm:
/// - ZADD key now_ms member
/// - ZREMRANGEBYSCORE key 0 (now_ms - window_ms)
/// - ZCARD key
/// - EXPIRE key ttl_sec
/// - allow if count <= limit
pub async fn rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let cfg = &state.config;

    // Disable if configured to 0 (explicit opt-out).
    if cfg.rate_limit_requests_per_minute == 0 {
        return next.run(req).await;
    }

    let limit =
        (cfg.rate_limit_requests_per_minute as u64).saturating_add(cfg.rate_limit_burst as u64);

    // Sliding window: 60 seconds
    let window_ms: i64 = 60_000;
    let now_ms = now_ms();

    let (key, ttl_secs) = build_key(cfg.service_name.as_str(), &req);
    let member = build_member(&req, now_ms);

    let mut con = state.redis.clone();

    match allow_sliding_window(&mut con, &key, now_ms, window_ms, &member, limit, ttl_secs).await {
        Ok(true) => next.run(req).await,
        Ok(false) => tagged(
            (StatusCode::TOO_MANY_REQUESTS, "rate limit exceeded").into_response(),
            "rate_limited",
        ),
        Err(_) => tagged(
            (StatusCode::SERVICE_UNAVAILABLE, "rate limiter unavailable").into_response(),
            "dependency",
        ),
    }
}

fn tagged(mut resp: Response, label: &str) -> Response {
    if let Ok(hv) = HeaderValue::from_str(label) {
        resp.headers_mut().insert(X_MACH_ERROR_TYPE, hv);
    }
    resp
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Member must be unique per request *within the actor key*.
/// Prefer RequestContext.request_id, otherwise use (now_ms + best-effort fingerprint).
fn build_member(req: &Request<Body>, now_ms: i64) -> String {
    if let Some(ctx) = req.extensions().get::<RequestContext>() {
        return format!("rid={}", ctx.request_id());
    }
    if let Some(fp) = bearer_fingerprint(req) {
        return format!("t={fp}:{now_ms}");
    }
    if let Some(ip) = best_effort_ip(req) {
        return format!("ip={ip}:{now_ms}");
    }
    format!("anon:{now_ms}")
}

fn build_key(service_name: &str, req: &Request<Body>) -> (String, u64) {
    let mut b = RateLimitKeyBuilder::new(service_name);

    let mut actor_suffix: Option<String> = None;

    if let Some(ctx) = req.extensions().get::<RequestContext>() {
        if let Some(t) = ctx.tenant_id() {
            b = b.with_tenant(t.clone());
        }
        if let Some(u) = ctx.user_id() {
            b = b.with_user(*u);
        }
        if let Some(r) = ctx.region() {
            b = b.with_region(r.clone());
        }
    }

    // If we don't have a user key, fall back to token or IP.
    if req
        .extensions()
        .get::<RequestContext>()
        .and_then(|c| c.user_id())
        .is_none()
    {
        if let Some(fp) = bearer_fingerprint(req) {
            actor_suffix = Some(format!("token={fp}"));
        } else if let Some(ip) = best_effort_ip(req) {
            actor_suffix = Some(format!("ip={ip}"));
        } else {
            actor_suffix = Some("anon".to_string());
        }
    }

    // Operation label (stable; no buckets in sliding window)
    let op = match actor_suffix {
        Some(sfx) => format!("gateway:sliding60s:{sfx}"),
        None => "gateway:sliding60s".to_string(),
    };

    // TTL slightly > window to allow cleanup even under low traffic.
    (b.build(&op), 62)
}

fn bearer_fingerprint(req: &Request<Body>) -> Option<String> {
    let hv = req.headers().get(header::AUTHORIZATION)?.to_str().ok()?;
    let token = hv
        .strip_prefix("Bearer ")
        .or_else(|| hv.strip_prefix("bearer "))?
        .trim();
    if token.is_empty() {
        return None;
    }
    let hash = blake3::hash(token.as_bytes());
    let hex = hex::encode(hash.as_bytes());
    // Keep keys shorter while remaining collision-resistant enough for rate limiting.
    Some(hex.chars().take(32).collect())
}

fn best_effort_ip(req: &Request<Body>) -> Option<String> {
    // 1) x-forwarded-for (first)
    if let Some(ip) = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
    {
        let first = ip.split(',').next().unwrap_or("").trim();
        if !first.is_empty() {
            return Some(first.to_string());
        }
    }

    // 2) x-real-ip
    if let Some(ip) = req.headers().get("x-real-ip").and_then(|h| h.to_str().ok()) {
        let ip = ip.trim();
        if !ip.is_empty() {
            return Some(ip.to_string());
        }
    }

    None
}

static RL_SLIDING_LUA: &str = r#"
local key = KEYS[1]
local now_ms = tonumber(ARGV[1])
local window_ms = tonumber(ARGV[2])
local member = ARGV[3]
local limit = tonumber(ARGV[4])
local ttl_sec = tonumber(ARGV[5])

redis.call('ZADD', key, now_ms, member)
redis.call('ZREMRANGEBYSCORE', key, 0, now_ms - window_ms)
local count = redis.call('ZCARD', key)
redis.call('EXPIRE', key, ttl_sec)

if count > limit then
  return 0
end
return 1
"#;

async fn allow_sliding_window(
    con: &mut redis::aio::ConnectionManager,
    key: &str,
    now_ms: i64,
    window_ms: i64,
    member: &str,
    limit: u64,
    ttl_secs: u64,
) -> Result<bool, redis::RedisError> {
    let script = Script::new(RL_SLIDING_LUA);

    let ok: i32 = script
        .key(key)
        .arg(now_ms)
        .arg(window_ms)
        .arg(member)
        .arg(limit)
        .arg(ttl_secs)
        .invoke_async(con)
        .await?;

    Ok(ok == 1)
}
