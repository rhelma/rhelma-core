#![forbid(unsafe_code)]

//! Trace context helpers (Contract v6.0).
//!
//! This module provides *best-effort* "local context" storage for systems that
//! don't explicitly pass `rhelma_core::RequestContext` through the call stack.
//!
//! **Important:** In async runtimes, tasks may move between threads. For strict
//! correctness, use `scope(...)` / `scope_with_headers(...)` to bind context to a
//! Tokio task. Outside of a Tokio scope, a thread-local fallback is used for
//! compatibility with sync code and tests.

use std::cell::RefCell;
use std::collections::HashMap;

use rhelma_core::TraceContext;

// Optional OTEL bridge: when services install `tracing-opentelemetry` (feature `otel`),
// we can mirror the active span's W3C trace/span ids into Rhelma's lightweight
// local context store. This ensures Kafka/HTTP propagation matches the OTEL
// exporter and improves end-to-end trace correlation.
#[cfg(feature = "otel")]
use opentelemetry::trace::TraceContextExt;
#[cfg(feature = "otel")]
use tracing_opentelemetry::OpenTelemetrySpanExt;

const HEADER_TRACEPARENT: &str = "traceparent";
const HEADER_TRACESTATE: &str = "tracestate";
const HEADER_BAGGAGE: &str = "baggage";
const HEADER_X_TRACE_ID: &str = "x-trace-id";
const HEADER_X_SPAN_ID: &str = "x-span-id";
const HEADER_X_MACH_TRACE_ID: &str = "x-rhelma-trace-id";
const HEADER_X_MACH_SPAN_ID: &str = "x-rhelma-span-id";
const HEADER_X_CORRELATION_ID: &str = "x-correlation-id";
const HEADER_X_MACH_CORRELATION_ID: &str = "x-rhelma-correlation-id";
const HEADER_X_REQUEST_ID: &str = "x-request-id";
const HEADER_X_MACH_REQUEST_ID: &str = "x-rhelma-request-id";
const HEADER_X_RESIDENCY: &str = "x-residency";
const HEADER_X_MACH_RESIDENCY: &str = "x-rhelma-residency";
const HEADER_X_TENANT_ID: &str = "x-tenant-id";
const HEADER_X_REGION: &str = "x-region";

// --- Security hardening (DoS guards) ---
// W3C traceparent is ~55 chars in normal form; we allow a small safety margin.
const MAX_TRACEPARENT_LEN: usize = 256;
// tracestate can be larger; keep a firm cap to avoid pathological inputs.
const MAX_TRACESTATE_LEN: usize = 1024;
const MAX_BAGGAGE_LEN: usize = 2048;
const MAX_BAGGAGE_ITEMS: usize = 16;
const MAX_BAGGAGE_KEY_LEN: usize = 64;
const MAX_BAGGAGE_VALUE_LEN: usize = 128;
// Legacy IDs should be tiny (trace-id 32 hex, span-id 16 hex). Correlation IDs
// may vary, but still must be bounded.
const MAX_LEGACY_ID_LEN: usize = 128;
const MAX_TENANT_ID_LEN: usize = 256;
const MAX_REGION_LEN: usize = 128;

#[derive(Debug, Clone)]
struct LocalContext {
    trace: TraceContext,
    tracestate: Option<String>,
    baggage: Option<String>,
    request_id: Option<String>,
    correlation_id: Option<String>,
    residency: Option<String>,
    tenant_id: Option<String>,
    region: Option<String>,
    sampled: Option<bool>,
}

impl Default for LocalContext {
    fn default() -> Self {
        Self {
            trace: TraceContext::new(None, None),
            tracestate: None,
            baggage: None,
            request_id: None,
            correlation_id: None,
            residency: None,
            tenant_id: None,
            region: None,
            sampled: None,
        }
    }
}

tokio::task_local! {
    static TASK_CTX: RefCell<LocalContext>;
}

thread_local! {
    static THREAD_CTX: RefCell<LocalContext> = RefCell::new(LocalContext::default());
}

/// Run a future with a fresh, task-local tracing context.
///
/// Prefer this in async request handlers.
pub async fn scope<Fut, R>(fut: Fut) -> R
where
    Fut: std::future::Future<Output = R>,
{
    TASK_CTX
        .scope(RefCell::new(LocalContext::default()), fut)
        .await
}

/// Run a future with context extracted from incoming headers.
pub async fn scope_with_headers<Fut, R>(headers: &HashMap<String, String>, fut: Fut) -> R
where
    Fut: std::future::Future<Output = R>,
{
    let mut ctx = LocalContext::default();
    apply_extract_into(&mut ctx, headers, /*only_traceparent=*/ false);
    TASK_CTX.scope(RefCell::new(ctx), fut).await
}

fn with_ctx_mut<R>(f: impl FnOnce(&mut LocalContext) -> R) -> R {
    // Prefer task-local if available (Tokio scope). Fall back to thread-local.
    //
    // IMPORTANT: We must avoid holding a RefCell borrow across the user closure.
    // Otherwise, if the closure calls back into this module (e.g. inject/extract),
    // we'll panic with "RefCell already borrowed".
    //
    // To make this re-entrant, we temporarily *move* the context out using
    // `RefCell::replace`, run the closure, then put it back.
    let mut f = Some(f);

    if let Ok(r) = TASK_CTX.try_with(|cell| {
        let mut local = cell.replace(LocalContext::default());
        let r = (f.take().expect("with_ctx_mut called twice"))(&mut local);
        cell.replace(local);
        r
    }) {
        return r;
    }

    THREAD_CTX.with(|cell| {
        let mut local = cell.replace(LocalContext::default());
        let r = (f.take().expect("with_ctx_mut missing fallback"))(&mut local);
        cell.replace(local);
        r
    })
}

fn with_ctx<R>(f: impl FnOnce(&LocalContext) -> R) -> R {
    // Same `FnOnce`-in-two-branches issue as `with_ctx_mut`.
    //
    // We also avoid holding an active RefCell borrow across the user closure by
    // cloning a snapshot. This keeps the API re-entrant (closure may call
    // `with_ctx_mut`).
    let mut f = Some(f);

    if let Ok(r) = TASK_CTX.try_with(|cell| {
        let snapshot = cell.borrow().clone();
        (f.take().expect("with_ctx called twice"))(&snapshot)
    }) {
        return r;
    }

    THREAD_CTX.with(|cell| {
        let snapshot = cell.borrow().clone();
        (f.take().expect("with_ctx missing fallback"))(&snapshot)
    })
}

/// Clear all in-memory context (task-local if present; otherwise thread-local).
pub fn clear_current_ids() {
    with_ctx_mut(|ctx| {
        *ctx = LocalContext::default();
    });
}

/// Returns the current trace-id (W3C 32-hex), generating if absent.
pub fn current_trace_id() -> Option<String> {
    ensure_trace_context();
    with_ctx(|ctx| ctx.trace.trace_id.clone())
}

/// Returns the current span-id (W3C 16-hex), generating if absent.
pub fn current_span_id() -> Option<String> {
    ensure_trace_context();
    with_ctx(|ctx| ctx.trace.span_id.clone())
}

/// Returns current tracestate, if any.
pub fn current_tracestate() -> Option<String> {
    with_ctx(|ctx| ctx.tracestate.clone())
}

/// Returns current W3C baggage header value (sanitized/allowlisted).
pub fn current_baggage() -> Option<String> {
    with_ctx(|ctx| ctx.baggage.clone())
}

/// Sanitize a raw W3C `baggage` header value using Rhelma bounds + allowlist.
///
/// This is useful for transports (e.g. Kafka producers) that want to read baggage
/// from an OpenTelemetry context but still enforce Rhelma's low-cardinality rules.
pub fn sanitize_baggage_header_value(raw: impl AsRef<str>) -> Option<String> {
    sanitize_baggage(raw.as_ref())
}

/// Set baggage from a raw header value. Value is sanitized and bounded.
pub fn set_baggage_header(raw: impl AsRef<str>) {
    let raw = raw.as_ref();
    let sanitized = sanitize_baggage(raw);
    with_ctx_mut(|ctx| ctx.baggage = sanitized);
}

/// Add/replace a single baggage key/value (bounded allowlist).
pub fn set_baggage_item(key: &str, value: &str) {
    let key = key.trim();
    let value = value.trim();
    if key.is_empty() || value.is_empty() {
        return;
    }

    with_ctx_mut(|ctx| {
        let mut items = parse_baggage_items(ctx.baggage.as_deref().unwrap_or(""));
        upsert_baggage_item(&mut items, key, value);
        ctx.baggage = build_baggage_header(&items);
    });
}

/// Returns current correlation id, if any.
pub fn current_correlation_id() -> Option<String> {
    with_ctx(|ctx| ctx.correlation_id.clone())
}

/// Returns current request id, if any.
pub fn current_request_id() -> Option<String> {
    with_ctx(|ctx| ctx.request_id.clone())
}

/// Set request id (for cross-service request correlation).
pub fn set_request_id(id: impl Into<String>) {
    with_ctx_mut(|ctx| ctx.request_id = Some(id.into()));
}

/// Returns current residency policy string, if any (e.g. GLOBAL).
pub fn current_residency() -> Option<String> {
    with_ctx(|ctx| ctx.residency.clone())
}

/// Set residency policy string (e.g. GLOBAL / REGIONAL_PREFERRED / REGIONAL_STRICT).
pub fn set_residency(res: impl Into<String>) {
    with_ctx_mut(|ctx| ctx.residency = Some(res.into()));
}
/// Returns current tenant id, if any.
pub fn current_tenant_id() -> Option<String> {
    with_ctx(|ctx| ctx.tenant_id.clone())
}

/// Set tenant id.
pub fn set_tenant_id(id: impl Into<String>) {
    with_ctx_mut(|ctx| ctx.tenant_id = Some(id.into()));
}

/// Returns current region, if any.
pub fn current_region() -> Option<String> {
    with_ctx(|ctx| ctx.region.clone())
}

/// Set region.
pub fn set_region(region: impl Into<String>) {
    with_ctx_mut(|ctx| ctx.region = Some(region.into()));
}

/// Set correlation id (for cross-service log correlation).
pub fn set_correlation_id(id: impl Into<String>) {
    with_ctx_mut(|ctx| ctx.correlation_id = Some(id.into()));
}

/// Set W3C sampled flag (affects injected traceparent flags).
pub fn set_sampled(sampled: bool) {
    with_ctx_mut(|ctx| ctx.sampled = Some(sampled));
}

/// Returns sampling decision, if known.
pub fn is_sampled() -> Option<bool> {
    with_ctx(|ctx| ctx.sampled)
}

/// Get current traceparent header value.
pub fn current_traceparent() -> Option<String> {
    ensure_trace_context();
    with_ctx(|ctx| {
        let flags = if ctx.sampled.unwrap_or(true) {
            "01"
        } else {
            "00"
        };
        // Build directly to avoid split/allocations.
        Some(format!(
            "00-{}-{}-{}",
            ctx.trace.trace_id.as_deref()?,
            ctx.trace.span_id.as_deref()?,
            flags
        ))
    })
}

/// Extract ONLY traceparent (+ optional tracestate) from headers.
/// Invalid traceparent → generate a new pair (zero-trust).
pub fn extract_traceparent(headers: &HashMap<String, String>) {
    with_ctx_mut(|ctx| apply_extract_into(ctx, headers, /*only_traceparent=*/ true));
}

/// Inject ONLY traceparent (+ optional tracestate) into outgoing headers.
pub fn inject_traceparent(headers: &mut HashMap<String, String>) {
    ensure_trace_context();
    with_ctx(|ctx| {
        if let Some(tp) = current_traceparent() {
            headers.insert(HEADER_TRACEPARENT.to_string(), tp);
        }
        if let Some(ts) = ctx.tracestate.clone() {
            headers.insert(HEADER_TRACESTATE.to_string(), ts);
        }
        if let Some(bg) = ctx.baggage.clone() {
            headers.insert(HEADER_BAGGAGE.to_string(), bg);
        }
    });
}

/// Extract full context from incoming headers (traceparent + legacy + correlation).
pub fn extract_current_context(headers: &HashMap<String, String>) {
    with_ctx_mut(|ctx| apply_extract_into(ctx, headers, /*only_traceparent=*/ false));
}

/// Inject full context into outgoing headers (traceparent + legacy + correlation).
pub fn inject_current_context(headers: &mut HashMap<String, String>) {
    ensure_trace_context();
    with_ctx(|ctx| {
        // W3C is canonical.
        if let Some(tp) = current_traceparent() {
            headers.insert(HEADER_TRACEPARENT.to_string(), tp);
        }
        if let Some(ts) = ctx.tracestate.clone() {
            headers.insert(HEADER_TRACESTATE.to_string(), ts);
        }
        if let Some(bg) = ctx.baggage.clone() {
            headers.insert(HEADER_BAGGAGE.to_string(), bg);
        }

        // Legacy headers for compatibility with older internal services.
        if let Some(tid) = ctx.trace.trace_id.clone() {
            headers.insert(HEADER_X_TRACE_ID.to_string(), tid.clone());
            headers.insert(HEADER_X_MACH_TRACE_ID.to_string(), tid);
        }
        if let Some(sid) = ctx.trace.span_id.clone() {
            headers.insert(HEADER_X_SPAN_ID.to_string(), sid.clone());
            headers.insert(HEADER_X_MACH_SPAN_ID.to_string(), sid);
        }

        if let Some(rid) = ctx.request_id.clone() {
            headers.insert(HEADER_X_MACH_REQUEST_ID.to_string(), rid.clone());
            headers.insert(HEADER_X_REQUEST_ID.to_string(), rid);
        }

        if let Some(cid) = ctx.correlation_id.clone() {
            headers.insert(HEADER_X_MACH_CORRELATION_ID.to_string(), cid.clone());
            headers.insert(HEADER_X_CORRELATION_ID.to_string(), cid);
        }

        if let Some(res) = ctx.residency.clone() {
            headers.insert(HEADER_X_RESIDENCY.to_string(), res.clone());
            headers.insert(HEADER_X_MACH_RESIDENCY.to_string(), res);
        }
        if let Some(tid) = ctx.tenant_id.clone() {
            headers.insert(HEADER_X_TENANT_ID.to_string(), tid);
        }
        if let Some(region) = ctx.region.clone() {
            headers.insert(HEADER_X_REGION.to_string(), region);
        }
    });
}

/// Back-compat alias (older name).
pub fn extract_or_generate(headers: &HashMap<String, String>) {
    extract_current_context(headers);
}

#[cfg(feature = "otel")]
fn try_sync_from_otel(ctx: &mut LocalContext) {
    // When `tracing-opentelemetry` is installed, the current tracing span carries
    // an OTEL span context that already includes the canonical W3C trace/span IDs.
    // We mirror those ids into our local store so transports (Kafka/HTTP) emit
    // trace headers that line up with the OTEL exporter.
    let span = tracing::Span::current();
    let otel_ctx = span.context();
    let sc = otel_ctx.span().span_context();
    if !sc.is_valid() {
        return;
    }

    if ctx.trace.trace_id.is_none() {
        ctx.trace.trace_id = Some(sc.trace_id().to_string());
    }
    if ctx.trace.span_id.is_none() {
        ctx.trace.span_id = Some(sc.span_id().to_string());
    }
    if ctx.sampled.is_none() {
        ctx.sampled = Some(sc.trace_flags().is_sampled());
    }

    if ctx.tracestate.is_none() {
        let ts = sc.trace_state().header();
        if !ts.trim().is_empty() {
            ctx.tracestate = Some(ts);
        }
    }
}

fn ensure_trace_context() {
    with_ctx_mut(|ctx| {
        #[cfg(feature = "otel")]
        {
            if ctx.trace.trace_id.is_none()
                || ctx.trace.span_id.is_none()
                || ctx.sampled.is_none()
                || ctx.tracestate.is_none()
            {
                try_sync_from_otel(ctx);
            }
        }

        if ctx.trace.trace_id.is_none() || ctx.trace.span_id.is_none() {
            let gen = TraceContext::generate();
            if ctx.trace.trace_id.is_none() {
                ctx.trace.trace_id = gen.trace_id;
            }
            if ctx.trace.span_id.is_none() {
                ctx.trace.span_id = gen.span_id;
            }
        }

        // Default to sampled=true to keep behavior consistent with earlier Rhelma
        // releases (fail-open observability).
        if ctx.sampled.is_none() {
            ctx.sampled = Some(true);
        }
    });
}

fn apply_extract_into(
    ctx: &mut LocalContext,
    headers: &HashMap<String, String>,
    only_traceparent: bool,
) {
    // Extract only relevant trace headers into a small fixed set.
    // This reduces work and avoids passing arbitrary large headers downstream.
    let mut traceparent: Option<&str> = None;
    let mut x_trace_id: Option<&str> = None;
    let mut x_rhelma_trace_id: Option<&str> = None;
    let mut x_span_id: Option<&str> = None;
    let mut x_rhelma_span_id: Option<&str> = None;
    let mut x_request_id: Option<&str> = None;
    let mut x_rhelma_request_id: Option<&str> = None;
    let mut x_correlation_id: Option<&str> = None;
    let mut x_rhelma_correlation_id: Option<&str> = None;
    let mut x_residency: Option<&str> = None;
    let mut x_rhelma_residency: Option<&str> = None;

    for (k, v) in headers.iter() {
        let key = k.trim();
        let value = v.trim();

        if key.eq_ignore_ascii_case(HEADER_TRACEPARENT) {
            if value.len() <= MAX_TRACEPARENT_LEN {
                traceparent = Some(value);
            }
            continue;
        }
        if key.eq_ignore_ascii_case(HEADER_X_TRACE_ID) {
            if value.len() <= MAX_LEGACY_ID_LEN {
                x_trace_id = Some(value);
            }
            continue;
        }
        if key.eq_ignore_ascii_case(HEADER_X_MACH_TRACE_ID) {
            if value.len() <= MAX_LEGACY_ID_LEN {
                x_rhelma_trace_id = Some(value);
            }
            continue;
        }
        if key.eq_ignore_ascii_case(HEADER_X_SPAN_ID) {
            if value.len() <= MAX_LEGACY_ID_LEN {
                x_span_id = Some(value);
            }
            continue;
        }
        if key.eq_ignore_ascii_case(HEADER_X_MACH_SPAN_ID) {
            if value.len() <= MAX_LEGACY_ID_LEN {
                x_rhelma_span_id = Some(value);
            }
            continue;
        }

        if key.eq_ignore_ascii_case(HEADER_X_REQUEST_ID) {
            if value.len() <= MAX_LEGACY_ID_LEN {
                x_request_id = Some(value);
            }
            continue;
        }
        if key.eq_ignore_ascii_case(HEADER_X_MACH_REQUEST_ID) {
            if value.len() <= MAX_LEGACY_ID_LEN {
                x_rhelma_request_id = Some(value);
            }
            continue;
        }
        if key.eq_ignore_ascii_case(HEADER_X_CORRELATION_ID) {
            if value.len() <= MAX_LEGACY_ID_LEN {
                x_correlation_id = Some(value);
            }
            continue;
        }
        if key.eq_ignore_ascii_case(HEADER_X_MACH_CORRELATION_ID) {
            if value.len() <= MAX_LEGACY_ID_LEN {
                x_rhelma_correlation_id = Some(value);
            }
            continue;
        }
        if key.eq_ignore_ascii_case(HEADER_X_RESIDENCY) {
            if value.len() <= MAX_LEGACY_ID_LEN {
                x_residency = Some(value);
            }
            continue;
        }
        if key.eq_ignore_ascii_case(HEADER_X_MACH_RESIDENCY) {
            if value.len() <= MAX_LEGACY_ID_LEN {
                x_rhelma_residency = Some(value);
            }
            continue;
        }
    }

    // Build a bounded header iterator using canonical keys.
    // NOTE: Avoid a closure here. A closure that accepts `&str` cannot store that reference
    // into an outer array safely in Rust's borrow checker model (it would allow the reference
    // to outlive the call). Manual pushes are allocation-free and lifetime-safe.
    let mut pairs: [(&str, &str); 5] = [("", ""); 5];
    let mut n = 0usize;

    if let Some(v) = traceparent {
        if n < pairs.len() {
            pairs[n] = (HEADER_TRACEPARENT, v);
            n += 1;
        }
    }
    if let Some(v) = x_trace_id {
        if n < pairs.len() {
            pairs[n] = (HEADER_X_TRACE_ID, v);
            n += 1;
        }
    }
    if let Some(v) = x_rhelma_trace_id {
        if n < pairs.len() {
            pairs[n] = (HEADER_X_MACH_TRACE_ID, v);
            n += 1;
        }
    }
    if let Some(v) = x_span_id {
        if n < pairs.len() {
            pairs[n] = (HEADER_X_SPAN_ID, v);
            n += 1;
        }
    }
    if let Some(v) = x_rhelma_span_id {
        if n < pairs.len() {
            pairs[n] = (HEADER_X_MACH_SPAN_ID, v);
            n += 1;
        }
    }

    // rhelma-core performs strict W3C/legacy extraction.

    ctx.trace = TraceContext::extract_from_headers(pairs[..n].iter().copied());

    // tracestate is pass-through if present
    ctx.tracestate = headers
        .get(HEADER_TRACESTATE)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .filter(|s| s.len() <= MAX_TRACESTATE_LEN)
        .map(|s| s.to_string());

    // baggage is pass-through but sanitized/allowlisted
    ctx.baggage = headers
        .get(HEADER_BAGGAGE)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .filter(|s| s.len() <= MAX_BAGGAGE_LEN)
        .and_then(sanitize_baggage);

    // sampled flag from traceparent (if present & parseable), otherwise keep existing/default
    if let Some(tp) = headers
        .get(HEADER_TRACEPARENT)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .filter(|s| s.len() <= MAX_TRACEPARENT_LEN)
    {
        if let Some(sampled) = parse_sampled_flag(tp) {
            ctx.sampled = Some(sampled);
        }
    } else if ctx.sampled.is_none() {
        ctx.sampled = Some(true);
    }

    // request/correlation/residency are only extracted in "full" mode
    if !only_traceparent {
        // request id: prefer canonical
        ctx.request_id = x_rhelma_request_id
            .or(x_request_id)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        // correlation id: prefer canonical
        ctx.correlation_id = x_rhelma_correlation_id
            .or(x_correlation_id)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        // residency: prefer canonical
        ctx.residency = x_rhelma_residency
            .or(x_residency)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        // tenant + region (v6.0)
        ctx.tenant_id = headers
            .get(HEADER_X_TENANT_ID)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .filter(|s| s.len() <= MAX_TENANT_ID_LEN)
            .map(|s| s.to_string());

        ctx.region = headers
            .get(HEADER_X_REGION)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .filter(|s| s.len() <= MAX_REGION_LEN)
            .map(|s| s.to_string());
    }
}

// --- Baggage (W3C) ---
//
// We only propagate a small allowlist of keys to avoid accidental PII leakage and
// keep header sizes bounded. If you need additional keys, add them here and keep
// them low-cardinality.
const ALLOWED_BAGGAGE_KEYS: [&str; 4] = [
    "rhelma.operation",
    "rhelma.tenant",
    "rhelma.subject",
    "rhelma.value.amount",
];

fn is_allowed_baggage_key(key: &str) -> bool {
    let key = key.trim();
    if key.is_empty() {
        return false;
    }
    ALLOWED_BAGGAGE_KEYS
        .iter()
        .any(|k| k.eq_ignore_ascii_case(key))
}

fn sanitize_baggage(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    if raw.len() > MAX_BAGGAGE_LEN {
        return None;
    }
    let items = parse_baggage_items(raw);
    build_baggage_header(&items)
}

fn parse_baggage_items(raw: &str) -> Vec<(String, String)> {
    let mut items: Vec<(String, String)> = Vec::new();

    for part in raw.split(',') {
        if items.len() >= MAX_BAGGAGE_ITEMS {
            break;
        }

        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        // Ignore any per-item properties after ';' (we don't propagate them).
        let main = part.split(';').next().unwrap_or(part).trim();
        let Some((k, v)) = main.split_once('=') else {
            continue;
        };

        upsert_baggage_item(&mut items, k, v);
    }

    items
}

fn upsert_baggage_item(items: &mut Vec<(String, String)>, key: &str, value: &str) {
    let key = key.trim();
    let value = value.trim();

    if key.is_empty() || value.is_empty() {
        return;
    }
    if key.len() > MAX_BAGGAGE_KEY_LEN || value.len() > MAX_BAGGAGE_VALUE_LEN {
        return;
    }
    if !key.is_ascii() || !value.is_ascii() {
        return;
    }
    if !is_allowed_baggage_key(key) {
        return;
    }

    let key_lc = key.to_ascii_lowercase();

    if let Some(pos) = items.iter().position(|(k, _)| k == &key_lc) {
        items[pos].1 = value.to_string();
        return;
    }

    if items.len() >= MAX_BAGGAGE_ITEMS {
        return;
    }

    items.push((key_lc, value.to_string()));
}

fn build_baggage_header(items: &Vec<(String, String)>) -> Option<String> {
    if items.is_empty() {
        return None;
    }

    let mut out = String::new();

    for (k, v) in items {
        if out.is_empty() {
            if k.len() + 1 + v.len() > MAX_BAGGAGE_LEN {
                break;
            }
            out.push_str(k);
            out.push('=');
            out.push_str(v);
        } else {
            if out.len() + 1 + k.len() + 1 + v.len() > MAX_BAGGAGE_LEN {
                break;
            }
            out.push(',');
            out.push_str(k);
            out.push('=');
            out.push_str(v);
        }
    }

    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn parse_sampled_flag(tp: &str) -> Option<bool> {
    if tp.len() > MAX_TRACEPARENT_LEN {
        return None;
    }
    // Parse "00-<traceid>-<spanid>-<flags>" without allocating.
    let mut it = tp.splitn(4, '-');
    let _version = it.next()?;
    let _trace_id = it.next()?;
    let _span_id = it.next()?;
    let flags = it.next()?.trim();
    if flags.len() != 2 {
        return None;
    }
    let b = u8::from_str_radix(flags, 16).ok()?;
    Some((b & 0x01) == 0x01)
}
