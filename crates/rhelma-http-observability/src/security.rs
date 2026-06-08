#![forbid(unsafe_code)]

//! Security-focused middleware helpers for Rhelma services.
//!
//! This module provides small, dependency-light tower layers:
//!
//! - **Audit logging** for sensitive routes (admin/internal).
//! - **Rate limiting** (best-effort) for sensitive routes.
//! - **Idempotency / replay protection** for mutating routes (opt-in via `Idempotency-Key`).
//! - **Concurrency limiting** for backpressure (best-effort).
//!
//! Design constraints:
//! - Avoid extra dependencies so every service can adopt it.
//! - Keep logs low-cardinality (normalize obvious IDs in paths).
//! - Never log secrets (tokens are not emitted).

use axum::{
    body::{Body, Bytes},
    http::{header, Request, Response, StatusCode},
};
use ipnet::IpNet;

use std::{
    borrow::Cow,
    collections::{hash_map::DefaultHasher, HashMap, VecDeque},
    future::Future,
    hash::{Hash, Hasher},
    net::IpAddr,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
    time::{Duration, Instant},
};
use tokio::sync::{oneshot, Mutex as AsyncMutex, Semaphore};
use tower::{Layer, Service};

/// Default prefix list used by [`audit_layer_sensitive`] and [`rate_limit_layer_sensitive`].
///
/// These are intended to cover the internal/admin endpoints present across Rhelma apps.
pub const DEFAULT_SENSITIVE_PREFIXES: [&str; 3] = ["/v1/internal", "/v1/admin", "/v1/gov/"];

/// Build an audit logging layer for sensitive routes.
///
/// The layer logs a single structured event per request for any path that matches
/// [`DEFAULT_SENSITIVE_PREFIXES`] **and** uses a mutating HTTP method
/// (`POST`, `PUT`, `PATCH`, `DELETE`).
///
/// Audit logs are emitted at `INFO` for successful responses (`< 400`) and `WARN`
/// otherwise.
#[must_use]
pub fn audit_layer_sensitive() -> AuditLayer {
    AuditLayer::new(&DEFAULT_SENSITIVE_PREFIXES)
}

/// Build a rate limiting layer for sensitive routes.
///
/// By default this is **enabled** and configured via environment variables:
/// - `RHELMA_RATE_LIMIT__ENABLED` (default: `1`)
/// - `RHELMA_RATE_LIMIT__SENSITIVE_RPM` (default: `240`)
/// - `RHELMA_RATE_LIMIT__WINDOW_SECS` (default: `60`)
///
/// The limiter is best-effort and intentionally simple (sliding window).
/// Requests are keyed by a best-effort client identifier:
/// - IP from `x-forwarded-for` / `x-real-ip` (first value only)
/// - Plus an in-memory, non-cryptographic hash of any admin token header, if present
///   (to provide fair sharing when many callers share the same NAT).
#[must_use]
pub fn rate_limit_layer_sensitive() -> RateLimitLayer {
    let enabled = env_bool("RHELMA_RATE_LIMIT__ENABLED", true);
    let rpm = env_usize("RHELMA_RATE_LIMIT__SENSITIVE_RPM", 240);
    let window_secs = env_u64("RHELMA_RATE_LIMIT__WINDOW_SECS", 60);

    RateLimitLayer::new(
        enabled,
        Duration::from_secs(window_secs),
        rpm,
        &DEFAULT_SENSITIVE_PREFIXES,
    )
}

/// Build an idempotency / replay-protection layer.
///
/// This layer is **opt-in** by default: it only activates when the request
/// includes an `Idempotency-Key` header.
///
/// Environment variables:
/// - `RHELMA_IDEMPOTENCY__ENABLED` (default: `1`)
/// - `RHELMA_IDEMPOTENCY__TTL_SECS` (default: `900`)
/// - `RHELMA_IDEMPOTENCY__MAX_ENTRIES` (default: `8192`)
/// - `RHELMA_IDEMPOTENCY__MAX_BODY_BYTES` (default: `1048576`)
#[must_use]
pub fn idempotency_layer() -> IdempotencyLayer {
    let enabled = env_bool("RHELMA_IDEMPOTENCY__ENABLED", true);
    let ttl = Duration::from_secs(env_u64("RHELMA_IDEMPOTENCY__TTL_SECS", 900));
    let max_entries = env_usize("RHELMA_IDEMPOTENCY__MAX_ENTRIES", 8192);
    let max_body = env_usize("RHELMA_IDEMPOTENCY__MAX_BODY_BYTES", 1024 * 1024);
    IdempotencyLayer::new(enabled, ttl, max_entries, max_body)
}

/// Build a concurrency limiting layer for backpressure.
///
/// Environment variables:
/// - `RHELMA_CONCURRENCY__ENABLED` (default: `1`)
/// - `RHELMA_CONCURRENCY__LIMIT` (default: `256`)
#[must_use]
pub fn concurrency_limit_layer() -> BackpressureLayer {
    let enabled = env_bool("RHELMA_CONCURRENCY__ENABLED", true);
    if !enabled {
        return BackpressureLayer::disabled();
    }
    let limit = env_usize("RHELMA_CONCURRENCY__LIMIT", 256).max(1);
    let retry_after_secs = env_u64("RHELMA_CONCURRENCY__RETRY_AFTER_SECS", 1).max(1);
    BackpressureLayer::new(limit, retry_after_secs)
}

/// Build an IP allow-list layer for privileged routes exposed on a public listener.
///
/// The allow-list is configured via a comma-separated env var that may contain
/// IPs (e.g. `10.0.0.5`) and/or CIDRs (e.g. `10.0.0.0/8,192.168.0.0/16`).
///
/// If the env var is missing/empty, the layer is disabled (pass-through).
///
/// Note: this is best-effort and relies on either:
/// - `ConnectInfo<SocketAddr>` (when the Axum server is built with
///   `into_make_service_with_connect_info::<SocketAddr>()`), or
/// - proxy headers (`x-forwarded-for` / `x-real-ip`) as a fallback.
#[must_use]
pub fn ip_allowlist_layer_from_env(
    env_key: &'static str,
    prefixes: &'static [&'static str],
) -> IpAllowlistLayer {
    IpAllowlistLayer::from_env(env_key, prefixes)
}

fn env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .ok()
        .map(|v| {
            let v = v.trim();
            !(v.is_empty() || v == "0" || v.eq_ignore_ascii_case("false"))
        })
        .unwrap_or(default)
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(default)
}

/// Simple in-memory sliding-window rate limiter.
///
/// This limiter is **best-effort** and intended for low-cardinality sensitive endpoints.
/// It is not distributed and resets on process restart.
#[derive(Debug)]
struct SlidingWindowLimiter {
    window: Duration,
    max_requests: usize,
    state: Mutex<HashMap<String, VecDeque<Instant>>>,
    max_keys: usize,
}

impl SlidingWindowLimiter {
    #[must_use]
    fn new(window: Duration, max_requests: usize) -> Self {
        Self {
            window,
            max_requests: max_requests.max(1),
            state: Mutex::new(HashMap::new()),
            max_keys: 8192,
        }
    }

    /// Returns `(allowed, retry_after)`.
    fn allow(&self, key: &str) -> (bool, Option<Duration>) {
        let now = Instant::now();
        let mut state = match self.state.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };

        // Opportunistic bound on cardinality.
        if state.len() > self.max_keys.saturating_mul(2) {
            state.clear();
        }

        let dq = state.entry(key.to_string()).or_insert_with(VecDeque::new);

        // Prune timestamps outside the window.
        while let Some(front) = dq.front().copied() {
            if now.duration_since(front) >= self.window {
                dq.pop_front();
            } else {
                break;
            }
        }

        if dq.len() < self.max_requests {
            dq.push_back(now);
            return (true, None);
        }

        let retry_after = dq.front().copied().map(|oldest| {
            let elapsed = now.duration_since(oldest);
            if elapsed >= self.window {
                Duration::ZERO
            } else {
                self.window - elapsed
            }
        });

        (false, retry_after)
    }
}

/// Tower layer providing best-effort concurrency limiting (backpressure).
#[derive(Clone)]
pub struct BackpressureLayer {
    enabled: bool,
    semaphore: Arc<Semaphore>,
    retry_after: Duration,
}

impl BackpressureLayer {
    /// Create a new backpressure layer.
    ///
    /// `limit` is the maximum number of in-flight requests allowed. When the
    /// limit is exceeded, the service returns `429 Too Many Requests` and adds
    /// a best-effort `Retry-After` header.
    #[must_use]
    pub fn new(limit: usize, retry_after_secs: u64) -> Self {
        Self {
            enabled: true,
            semaphore: Arc::new(Semaphore::new(limit.max(1))),
            retry_after: Duration::from_secs(retry_after_secs.max(1)),
        }
    }

    /// Return a disabled backpressure layer (no concurrency limiting).
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            semaphore: Arc::new(Semaphore::new(1)),
            retry_after: Duration::from_secs(1),
        }
    }
}

impl<S> Layer<S> for BackpressureLayer {
    type Service = BackpressureService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        BackpressureService {
            inner,
            enabled: self.enabled,
            semaphore: self.semaphore.clone(),
            retry_after: self.retry_after,
        }
    }
}

/// Tower service implementing [`BackpressureLayer`].
#[derive(Clone)]
pub struct BackpressureService<S> {
    inner: S,
    enabled: bool,
    semaphore: Arc<Semaphore>,
    retry_after: Duration,
}

impl<S> Service<Request<Body>> for BackpressureService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();
        let enabled = self.enabled;
        let sem = self.semaphore.clone();
        let retry_after = self.retry_after;

        Box::pin(async move {
            if !enabled {
                return inner.call(req).await;
            }

            let permit = match sem.clone().try_acquire_owned() {
                Ok(p) => p,
                Err(_) => {
                    let mut resp = Response::new(Body::from("busy"));
                    *resp.status_mut() = StatusCode::TOO_MANY_REQUESTS;

                    let secs = retry_after.as_secs().max(1);
                    let _ = resp.headers_mut().insert(
                        header::RETRY_AFTER,
                        http::HeaderValue::from_str(&secs.to_string())
                            .unwrap_or_else(|_| http::HeaderValue::from_static("1")),
                    );
                    return Ok(resp);
                }
            };

            let res = inner.call(req).await;
            drop(permit);
            res
        })
    }
}

/// Normalize a URL path into a lower-cardinality form for logs.
///
/// Heuristics:
/// - Replace UUID-like segments with `:uuid`.
/// - Replace pure numeric segments with `:n`.
/// - Replace long hex-ish segments with `:id`.
#[must_use]
pub fn normalize_path(path: &str) -> Cow<'_, str> {
    // Fast path: if nothing looks like an ID, return borrowed.
    let looks_dynamic = path
        .split('/')
        .any(|seg| is_uuid_like(seg) || is_numeric(seg) || is_long_hex_like(seg));
    if !looks_dynamic {
        return Cow::Borrowed(path);
    }

    let mut out = String::with_capacity(path.len());
    for (i, seg) in path.split('/').enumerate() {
        if i > 0 {
            out.push('/');
        }
        if seg.is_empty() {
            continue;
        }
        if is_uuid_like(seg) {
            out.push_str(":uuid");
        } else if is_numeric(seg) {
            out.push_str(":n");
        } else if is_long_hex_like(seg) {
            out.push_str(":id");
        } else {
            out.push_str(seg);
        }
    }
    Cow::Owned(out)
}

fn is_numeric(seg: &str) -> bool {
    !seg.is_empty() && seg.bytes().all(|b| b.is_ascii_digit())
}

fn is_long_hex_like(seg: &str) -> bool {
    let len = seg.len();
    len >= 16
        && seg
            .bytes()
            .all(|b| b.is_ascii_hexdigit() || b == b'-' || b == b'_')
}

fn is_uuid_like(seg: &str) -> bool {
    // UUID v4/v7 textual length.
    if seg.len() != 36 {
        return false;
    }
    // Very small, allocation-free heuristic.
    // 8-4-4-4-12 with dashes.
    let bytes = seg.as_bytes();
    matches!(
        (bytes.get(8), bytes.get(13), bytes.get(18), bytes.get(23)),
        (Some(b'-'), Some(b'-'), Some(b'-'), Some(b'-'))
    ) && seg
        .bytes()
        .enumerate()
        .all(|(i, b)| matches!(i, 8 | 13 | 18 | 23) || b.is_ascii_hexdigit())
}

fn matches_sensitive(path: &str, prefixes: &[&'static str]) -> bool {
    prefixes.iter().any(|p| path.starts_with(p))
}

fn is_mutating_method(method: &http::Method) -> bool {
    matches!(
        *method,
        http::Method::POST | http::Method::PUT | http::Method::PATCH | http::Method::DELETE
    )
}

fn auth_header_present(headers: &http::HeaderMap) -> bool {
    headers.contains_key(header::AUTHORIZATION)
        || headers.contains_key("x-admin-token")
        || headers.contains_key("x-registry-admin-token")
        || headers.contains_key("x-judge-token")
        || headers.contains_key("x-police-token")
}

fn idempotency_key(headers: &http::HeaderMap) -> Option<String> {
    let v = headers
        .get("idempotency-key")
        .or_else(|| headers.get("Idempotency-Key"))?;
    let s = v.to_str().ok()?.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn method_path_fingerprint(
    method: &http::Method,
    path: &str,
    body_hash: u64,
    tok_hash: u64,
) -> u64 {
    let mut h = DefaultHasher::new();
    method.as_str().hash(&mut h);
    path.hash(&mut h);
    body_hash.hash(&mut h);
    tok_hash.hash(&mut h);
    h.finish()
}

fn hash_bytes(b: &[u8]) -> u64 {
    let mut h = DefaultHasher::new();
    b.hash(&mut h);
    h.finish()
}

fn strip_uncacheable_headers(hm: &mut http::HeaderMap) {
    // Avoid replaying request/trace identifiers which should be unique per response.
    for k in [
        "x-rhelma-request-id",
        "x-rhelma-correlation-id",
        "traceparent",
        "tracestate",
    ] {
        hm.remove(k);
    }
}

/// Cached response for idempotency.
#[derive(Clone)]
struct CachedResponse {
    status: StatusCode,
    headers: http::HeaderMap,
    body: Bytes,
}

enum EntryState {
    InFlight(Vec<oneshot::Sender<CachedResponse>>),
    Done(CachedResponse),
}

struct Entry {
    fingerprint: u64,
    created_at: Instant,
    state: EntryState,
}

#[derive(Clone)]
struct IdempotencyStore {
    ttl: Duration,
    max_entries: usize,
    // Async mutex because we need to await when registering waiters.
    map: Arc<AsyncMutex<HashMap<String, Entry>>>,
}

impl IdempotencyStore {
    fn new(ttl: Duration, max_entries: usize) -> Self {
        Self {
            ttl,
            max_entries: max_entries.max(64),
            map: Arc::new(AsyncMutex::new(HashMap::new())),
        }
    }

    async fn prune_locked(map: &mut HashMap<String, Entry>, ttl: Duration, max_entries: usize) {
        let now = Instant::now();
        map.retain(|_, e| now.duration_since(e.created_at) <= ttl);

        // Best-effort cap: drop arbitrary oldest-ish entries if still too big.
        if map.len() > max_entries {
            let mut items = map
                .iter()
                .map(|(k, v)| (k.clone(), v.created_at))
                .collect::<Vec<_>>();
            items.sort_by_key(|(_, t)| *t);
            let drop_n = map.len() - max_entries;
            for (k, _) in items.into_iter().take(drop_n) {
                map.remove(&k);
            }
        }
    }
}

/// Tower layer providing idempotency / replay protection.
#[derive(Clone)]
pub struct IdempotencyLayer {
    enabled: bool,
    store: IdempotencyStore,
    max_body_bytes: usize,
}

impl IdempotencyLayer {
    /// Create a new idempotency layer.
    ///
    /// This layer provides best-effort request replay protection using an
    /// in-memory store keyed by an idempotency key header.
    #[must_use]
    pub fn new(enabled: bool, ttl: Duration, max_entries: usize, max_body_bytes: usize) -> Self {
        Self {
            enabled,
            store: IdempotencyStore::new(ttl, max_entries),
            max_body_bytes: max_body_bytes.max(1024),
        }
    }
}

impl<S> Layer<S> for IdempotencyLayer {
    type Service = IdempotencyService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        IdempotencyService {
            inner,
            enabled: self.enabled,
            store: self.store.clone(),
            max_body_bytes: self.max_body_bytes,
        }
    }
}

/// Tower service implementing [`IdempotencyLayer`].
#[derive(Clone)]
pub struct IdempotencyService<S> {
    inner: S,
    enabled: bool,
    store: IdempotencyStore,
    max_body_bytes: usize,
}

impl<S> Service<Request<Body>> for IdempotencyService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();
        let enabled = self.enabled;
        let store = self.store.clone();
        let max_body_bytes = self.max_body_bytes;

        Box::pin(async move {
            if !enabled || !is_mutating_method(req.method()) {
                return inner.call(req).await;
            }

            let key = match idempotency_key(req.headers()) {
                Some(k) => k,
                None => return inner.call(req).await,
            };

            // Best-effort caller identity to prevent cross-tenant replay.
            let tok_hash = token_hash_hint(req.headers()).unwrap_or(0);
            let map_key = format!("tok={tok_hash};key={key}");

            let path = req.uri().path().to_string();
            let method = req.method().clone();

            // Buffer the request body to derive a fingerprint (replay protection) and to
            // allow re-sending the request downstream.
            let (parts, body) = req.into_parts();
            let body_bytes = axum::body::to_bytes(body, max_body_bytes)
                .await
                .unwrap_or_else(|_| Bytes::new());
            let body_hash = hash_bytes(&body_bytes);
            let fingerprint = method_path_fingerprint(&method, &path, body_hash, tok_hash);

            // Rebuild request.
            let req = Request::from_parts(parts, Body::from(body_bytes.clone()));

            // Fast path: check cache / register waiter.
            {
                let mut map = store.map.lock().await;
                IdempotencyStore::prune_locked(&mut map, store.ttl, store.max_entries).await;

                if let Some(entry) = map.get_mut(&map_key) {
                    if entry.fingerprint != fingerprint {
                        let mut resp = Response::new(Body::from(
                            "idempotency key reuse with different request",
                        ));
                        *resp.status_mut() = StatusCode::CONFLICT;
                        return Ok(resp);
                    }

                    match &mut entry.state {
                        EntryState::Done(cached) => {
                            let mut resp = Response::new(Body::from(cached.body.clone()));
                            *resp.status_mut() = cached.status;
                            *resp.headers_mut() = cached.headers.clone();
                            resp.headers_mut().insert(
                                "x-idempotency-status",
                                http::HeaderValue::from_static("hit"),
                            );
                            return Ok(resp);
                        }
                        EntryState::InFlight(waiters) => {
                            let (tx, rx) = oneshot::channel();
                            waiters.push(tx);
                            drop(map);
                            let cached = rx.await.unwrap_or(CachedResponse {
                                status: StatusCode::SERVICE_UNAVAILABLE,
                                headers: http::HeaderMap::new(),
                                body: Bytes::from_static(b"idempotency in-flight"),
                            });
                            let mut resp = Response::new(Body::from(cached.body.clone()));
                            *resp.status_mut() = cached.status;
                            *resp.headers_mut() = cached.headers.clone();
                            resp.headers_mut().insert(
                                "x-idempotency-status",
                                http::HeaderValue::from_static("wait"),
                            );
                            return Ok(resp);
                        }
                    }
                }

                map.insert(
                    map_key.clone(),
                    Entry {
                        fingerprint,
                        created_at: Instant::now(),
                        state: EntryState::InFlight(Vec::new()),
                    },
                );
            }

            // First writer: execute handler.
            let res = inner.call(req).await;

            // Convert to cached response if possible.
            match res {
                Ok(resp) => {
                    let status = resp.status();
                    let (parts, body) = resp.into_parts();
                    let body_bytes = axum::body::to_bytes(body, max_body_bytes)
                        .await
                        .unwrap_or_else(|_| Bytes::new());
                    let mut headers = parts.headers.clone();
                    strip_uncacheable_headers(&mut headers);

                    let cached = CachedResponse {
                        status,
                        headers,
                        body: body_bytes.clone(),
                    };

                    // Only cache < 500 (allow retries on server errors).
                    if status.as_u16() < 500 {
                        let mut map = store.map.lock().await;
                        if let Some(entry) = map.get_mut(&map_key) {
                            let waiters = match std::mem::replace(
                                &mut entry.state,
                                EntryState::Done(cached.clone()),
                            ) {
                                EntryState::InFlight(w) => w,
                                EntryState::Done(_) => Vec::new(),
                            };
                            for tx in waiters {
                                let _ = tx.send(cached.clone());
                            }
                        }
                    } else {
                        let mut map = store.map.lock().await;
                        map.remove(&map_key);
                    }

                    let mut out = Response::new(Body::from(body_bytes));
                    *out.status_mut() = status;
                    *out.headers_mut() = cached.headers.clone();
                    out.headers_mut().insert(
                        "x-idempotency-status",
                        http::HeaderValue::from_static("miss"),
                    );
                    Ok(out)
                }
                Err(e) => {
                    let mut map = store.map.lock().await;
                    map.remove(&map_key);
                    Err(e)
                }
            }
        })
    }
}

fn client_ip_hint(headers: &http::HeaderMap) -> Option<String> {
    let h = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))?
        .to_str()
        .ok()?;
    let first = h.split(',').next().unwrap_or("").trim();
    if first.is_empty() {
        None
    } else {
        Some(first.to_string())
    }
}

/// Return a client IP hint suitable for **logs**.
///
/// By default, this returns a hashed (non-reversible) representation to reduce
/// PII exposure in audit logs. Set `RHELMA_AUDIT__HASH_IP=0` to log raw IPs.
fn client_ip_log_hint(headers: &http::HeaderMap) -> String {
    let Some(ip) = client_ip_hint(headers) else {
        return "-".to_string();
    };

    if !env_bool("RHELMA_AUDIT__HASH_IP", true) {
        return ip;
    }

    let mut hasher = DefaultHasher::new();
    ip.hash(&mut hasher);
    format!("h:{:x}", hasher.finish())
}

fn token_hash_hint(headers: &http::HeaderMap) -> Option<u64> {
    let token = headers
        .get("x-admin-token")
        .or_else(|| headers.get("x-registry-admin-token"))
        .or_else(|| headers.get("x-judge-token"))
        .or_else(|| headers.get("x-police-token"))
        .or_else(|| headers.get(header::AUTHORIZATION))?
        .to_str()
        .ok()?;
    let token = token.trim();
    if token.is_empty() {
        return None;
    }
    let mut hasher = DefaultHasher::new();
    token.hash(&mut hasher);
    Some(hasher.finish())
}

/// Tower layer that enforces an IP allow-list for selected path prefixes.
///
/// This is intended as an extra guard when internal/admin routes must remain
/// reachable on a public listener (compat / rollout scenarios).
#[derive(Clone)]
pub struct IpAllowlistLayer {
    enabled: bool,
    prefixes: Arc<Vec<&'static str>>,
    allow: Arc<Vec<IpNet>>,
}

impl IpAllowlistLayer {
    /// Build from an env var; if unset/empty, this returns a disabled layer.
    #[must_use]
    pub fn from_env(env_key: &'static str, prefixes: &'static [&'static str]) -> Self {
        let raw = std::env::var(env_key).ok().unwrap_or_default();
        let raw = raw.trim();
        if raw.is_empty() {
            return Self {
                enabled: false,
                prefixes: Arc::new(prefixes.to_vec()),
                allow: Arc::new(Vec::new()),
            };
        }

        let mut nets = Vec::new();
        for part in raw.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            // Support both CIDR and bare IPs.
            let net = if part.contains('/') {
                part.parse::<IpNet>().ok()
            } else {
                let ip = part.parse::<IpAddr>().ok();
                ip.and_then(|ip| {
                    let prefix = match ip {
                        IpAddr::V4(_) => 32,
                        IpAddr::V6(_) => 128,
                    };
                    IpNet::new(ip, prefix).ok()
                })
            };

            if let Some(net) = net {
                nets.push(net);
            }
        }

        Self {
            enabled: !nets.is_empty(),
            prefixes: Arc::new(prefixes.to_vec()),
            allow: Arc::new(nets),
        }
    }

    /// Create a disabled allowlist layer.
    ///
    /// Requests are always allowed, but the configured `prefixes` are retained
    /// for consistent header behavior.
    #[must_use]
    pub fn disabled(prefixes: &'static [&'static str]) -> Self {
        Self {
            enabled: false,
            prefixes: Arc::new(prefixes.to_vec()),
            allow: Arc::new(Vec::new()),
        }
    }
}

impl<S> Layer<S> for IpAllowlistLayer {
    type Service = IpAllowlistService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        IpAllowlistService {
            inner,
            enabled: self.enabled,
            prefixes: self.prefixes.clone(),
            allow: self.allow.clone(),
        }
    }
}

/// Tower service implementing [`IpAllowlistLayer`].
#[derive(Clone)]
pub struct IpAllowlistService<S> {
    inner: S,
    enabled: bool,
    prefixes: Arc<Vec<&'static str>>,
    allow: Arc<Vec<IpNet>>,
}

impl<S> Service<Request<Body>> for IpAllowlistService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let enabled = self.enabled;
        let allow = self.allow.clone();
        let prefixes = self.prefixes.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            if enabled {
                let path = req.uri().path();
                if prefixes.iter().any(|p| path.starts_with(p)) {
                    let ip = client_ip_from_extensions_or_headers(&req);
                    let permitted = ip
                        .map(|ip| allow.iter().any(|net| net.contains(&ip)))
                        .unwrap_or(false);
                    if !permitted {
                        let mut resp = Response::new(Body::from("forbidden"));
                        *resp.status_mut() = StatusCode::FORBIDDEN;
                        return Ok(resp);
                    }
                }
            }

            inner.call(req).await
        })
    }
}

fn client_ip_from_extensions_or_headers(req: &Request<Body>) -> Option<IpAddr> {
    // Prefer connect info when available (true remote address).
    if let Some(ci) = req
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
    {
        return Some(ci.0.ip());
    }
    if let Some(sa) = req.extensions().get::<std::net::SocketAddr>() {
        return Some(sa.ip());
    }

    client_ip_hint(req.headers()).and_then(|s| s.parse::<IpAddr>().ok())
}

/// Tower layer that emits audit logs for sensitive routes.
#[derive(Clone)]
pub struct AuditLayer {
    prefixes: Arc<Vec<&'static str>>,
}

impl AuditLayer {
    /// Create a new audit layer from a list of sensitive path prefixes.
    #[must_use]
    pub fn new(prefixes: &[&'static str]) -> Self {
        Self {
            prefixes: Arc::new(prefixes.to_vec()),
        }
    }
}

impl<S> Layer<S> for AuditLayer {
    type Service = AuditService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuditService {
            inner,
            prefixes: self.prefixes.clone(),
        }
    }
}

/// Tower service implementing [`AuditLayer`].
#[derive(Clone)]
pub struct AuditService<S> {
    inner: S,
    prefixes: Arc<Vec<&'static str>>,
}

impl<S> Service<Request<Body>> for AuditService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();
        let prefixes = self.prefixes.clone();

        let method = req.method().clone();
        let path = req.uri().path().to_string();
        let normalized = normalize_path(&path).into_owned();
        let auth_present = auth_header_present(req.headers());
        let ip = client_ip_log_hint(req.headers());
        let start = Instant::now();

        Box::pin(async move {
            let should_audit = matches_sensitive(&path, &prefixes) && is_mutating_method(&method);
            let res = inner.call(req).await;
            if should_audit {
                match &res {
                    Ok(resp) => {
                        let status = resp.status().as_u16();
                        let latency_ms = start.elapsed().as_millis() as u64;
                        if status < 400 {
                            tracing::info!(
                                target: "rhelma.audit",
                                method = %method,
                                path = %normalized,
                                status = status,
                                latency_ms = latency_ms,
                                auth_present = auth_present,
                                client_ip = %ip,
                                "audit"
                            );
                        } else {
                            tracing::warn!(
                                target: "rhelma.audit",
                                method = %method,
                                path = %normalized,
                                status = status,
                                latency_ms = latency_ms,
                                auth_present = auth_present,
                                client_ip = %ip,
                                "audit"
                            );
                        }
                    }
                    Err(_) => {
                        let latency_ms = start.elapsed().as_millis() as u64;
                        tracing::warn!(
                            target: "rhelma.audit",
                            method = %method,
                            path = %normalized,
                            status = 0u16,
                            latency_ms = latency_ms,
                            auth_present = auth_present,
                            client_ip = %ip,
                            "audit_error"
                        );
                    }
                }
            }
            res
        })
    }
}

/// Tower layer providing best-effort rate limiting for sensitive routes.
#[derive(Clone)]
pub struct RateLimitLayer {
    enabled: bool,
    prefixes: Arc<Vec<&'static str>>,
    limiter: Arc<SlidingWindowLimiter>,
}

impl RateLimitLayer {
    /// Create a new rate limiting layer.
    #[must_use]
    pub fn new(
        enabled: bool,
        window: Duration,
        max_requests: usize,
        prefixes: &[&'static str],
    ) -> Self {
        Self {
            enabled,
            prefixes: Arc::new(prefixes.to_vec()),
            limiter: Arc::new(SlidingWindowLimiter::new(window, max_requests)),
        }
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            enabled: self.enabled,
            prefixes: self.prefixes.clone(),
            limiter: self.limiter.clone(),
        }
    }
}

/// Tower service implementing [`RateLimitLayer`].
#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    enabled: bool,
    prefixes: Arc<Vec<&'static str>>,
    limiter: Arc<SlidingWindowLimiter>,
}

impl<S> Service<Request<Body>> for RateLimitService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();
        let enabled = self.enabled;
        let prefixes = self.prefixes.clone();
        let limiter = self.limiter.clone();

        let path = req.uri().path().to_string();
        let headers = req.headers().clone();

        Box::pin(async move {
            if enabled && matches_sensitive(&path, &prefixes) {
                let ip = client_ip_hint(&headers).unwrap_or_else(|| "-".to_string());
                let tok = token_hash_hint(&headers);
                let key = if let Some(h) = tok {
                    format!("ip={ip};tok={h}")
                } else {
                    format!("ip={ip};tok=none")
                };

                let (allowed, retry_after) = limiter.allow(&key);
                if !allowed {
                    let mut resp = Response::new(Body::from("rate limited"));
                    *resp.status_mut() = StatusCode::TOO_MANY_REQUESTS;
                    if let Some(ra) = retry_after {
                        let secs = ra.as_secs().max(1);
                        let _ = resp.headers_mut().insert(
                            header::RETRY_AFTER,
                            http::HeaderValue::from_str(&secs.to_string())
                                .unwrap_or_else(|_| http::HeaderValue::from_static("1")),
                        );
                    }
                    return Ok(resp);
                }
            }

            inner.call(req).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tower::ServiceExt;

    #[test]
    fn normalize_path_reduces_cardinality() {
        assert_eq!(
            normalize_path("/v1/internal/disputes/00000000-0000-0000-0000-000000000000/resolve"),
            "/v1/internal/disputes/:uuid/resolve"
        );
        assert_eq!(normalize_path("/v1/admin/tx/12345"), "/v1/admin/tx/:n");
        assert_eq!(
            normalize_path("/v1/admin/blob/abcdef0123456789"),
            "/v1/admin/blob/:id"
        );
    }

    #[test]
    fn sliding_window_limits() {
        let limiter = SlidingWindowLimiter::new(Duration::from_secs(60), 2);
        assert!(limiter.allow("k").0);
        assert!(limiter.allow("k").0);
        assert!(!limiter.allow("k").0);
    }

    #[test]
    fn audit_ip_is_hashed_by_default() {
        std::env::remove_var("RHELMA_AUDIT__HASH_IP");

        let mut headers = http::HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.9".parse().unwrap());

        let out = client_ip_log_hint(&headers);
        assert!(out.starts_with("h:"));
        assert_ne!(out, "203.0.113.9");

        std::env::set_var("RHELMA_AUDIT__HASH_IP", "0");
        let raw = client_ip_log_hint(&headers);
        assert_eq!(raw, "203.0.113.9");
    }

    #[tokio::test]
    async fn idempotency_caches_first_response() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls2 = calls.clone();

        let inner = tower::service_fn(move |_req: Request<Body>| {
            let calls = calls2.clone();
            async move {
                let n = calls.fetch_add(1, Ordering::SeqCst) + 1;
                Ok::<_, std::convert::Infallible>(Response::new(Body::from(format!("n={n}"))))
            }
        });

        let svc =
            IdempotencyLayer::new(true, Duration::from_secs(60), 128, 1024 * 1024).layer(inner);

        let req1 = Request::builder()
            .method("POST")
            .uri("/v1/credits/earn")
            .header("Idempotency-Key", "k1")
            .body(Body::from("{}"))
            .unwrap();

        let resp1 = svc.clone().oneshot(req1).await.unwrap();
        let body1 = axum::body::to_bytes(resp1.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body1[..], b"n=1");

        let req2 = Request::builder()
            .method("POST")
            .uri("/v1/credits/earn")
            .header("Idempotency-Key", "k1")
            .body(Body::from("{}"))
            .unwrap();

        let resp2 = svc.oneshot(req2).await.unwrap();
        let body2 = axum::body::to_bytes(resp2.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body2[..], b"n=1");

        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn idempotency_conflict_on_mismatched_body() {
        let inner = tower::service_fn(|_req: Request<Body>| async {
            Ok::<_, std::convert::Infallible>(Response::new(Body::from("ok")))
        });

        let svc =
            IdempotencyLayer::new(true, Duration::from_secs(60), 128, 1024 * 1024).layer(inner);

        let req1 = Request::builder()
            .method("POST")
            .uri("/v1/receipts/issue")
            .header("Idempotency-Key", "k2")
            .body(Body::from("{\"a\":1}"))
            .unwrap();
        let _ = svc.clone().oneshot(req1).await.unwrap();

        let req2 = Request::builder()
            .method("POST")
            .uri("/v1/receipts/issue")
            .header("Idempotency-Key", "k2")
            .body(Body::from("{\"a\":2}"))
            .unwrap();
        let resp2 = svc.oneshot(req2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::CONFLICT);
    }

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[tokio::test]
    async fn ip_allowlist_allows_allowed_ip() {
        let (svc, req) = {
            let _g = ENV_LOCK.lock().expect("env lock");
            std::env::set_var("TEST_IP_ALLOWLIST", "10.0.0.0/8");

            let inner = tower::service_fn(|_req: Request<Body>| async {
                Ok::<_, std::convert::Infallible>(Response::new(Body::from("ok")))
            });

            let svc = IpAllowlistLayer::from_env("TEST_IP_ALLOWLIST", &["/v1/"]).layer(inner);

            let req = Request::builder()
                .method("GET")
                .uri("/v1/internal/test")
                .header("x-real-ip", "10.1.2.3")
                .body(Body::empty())
                .unwrap();

            (svc, req)
        };

        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn ip_allowlist_blocks_disallowed_ip() {
        let (svc, req) = {
            let _g = ENV_LOCK.lock().expect("env lock");
            std::env::set_var("TEST_IP_ALLOWLIST", "10.0.0.0/8");

            let inner = tower::service_fn(|_req: Request<Body>| async {
                Ok::<_, std::convert::Infallible>(Response::new(Body::from("ok")))
            });

            let svc = IpAllowlistLayer::from_env("TEST_IP_ALLOWLIST", &["/v1/"]).layer(inner);

            let req = Request::builder()
                .method("GET")
                .uri("/v1/internal/test")
                .header("x-real-ip", "192.168.1.10")
                .body(Body::empty())
                .unwrap();

            (svc, req)
        };

        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }
}
