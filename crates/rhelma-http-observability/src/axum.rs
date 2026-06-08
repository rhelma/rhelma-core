#![forbid(unsafe_code)]

//! Axum/Tower helpers for Rhelma HTTP Contract (v5.2 + v6.0).

use axum::{body::Body, http::Request};
use rhelma_core::constants::{
    HEADER_MACH_CORRELATION_ID, HEADER_MACH_REQUEST_ID, HEADER_REGION, HEADER_RESIDENCY,
    HEADER_TENANT_ID, HEADER_TRACEPARENT, HEADER_TRACESTATE,
};
use rhelma_core::TraceContext;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{Layer, Service};
use tower_http::classify::{ServerErrorsAsFailures, SharedClassifier};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

// ================================================================================================
// Contract v5.2 (kept for back-compat)
// ================================================================================================

/// Concrete trace layer type used by [`trace_layer_v52`].
pub type TraceLayerV52 =
    TraceLayer<SharedClassifier<ServerErrorsAsFailures>, fn(&Request<Body>) -> tracing::Span>;

/// Contract bootstrap layer (v5.2).
///
/// Responsibilities:
/// - Ensure canonical IDs exist: `x-rhelma-request-id`, `x-rhelma-correlation-id` (UUIDv7 recommended)
/// - Ensure `x-residency` exists (default `GLOBAL`)
/// - Ensure W3C `traceparent` exists (generate if missing or invalid)
///
/// This layer is intentionally lightweight and does not allocate beyond UUID strings.
#[derive(Clone, Default)]
pub struct ContractV52Layer;

impl<S> Layer<S> for ContractV52Layer {
    type Service = ContractV52Middleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ContractV52Middleware { inner }
    }
}

#[derive(Clone)]
/// Tower middleware that applies the Rhelma Contract v5.2 request header normalization.
///
/// This is the concrete service wrapper produced by [`ContractV52Layer`].
pub struct ContractV52Middleware<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for ContractV52Middleware<S>
where
    S: Service<Request<Body>, Response = axum::response::Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        ensure_contract_v52_headers(req.headers_mut());
        self.inner.call(req)
    }
}

/// Ensure Rhelma Contract v5.2 headers exist on the request.
///
/// This is exposed for services that prefer function middleware (`middleware::from_fn`)
/// rather than a `tower::Layer`.
pub fn ensure_contract_v52_headers(headers: &mut http::HeaderMap) {
    // request id
    let rid = headers
        .get(HEADER_MACH_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or_else(Uuid::now_v7);

    headers.insert(HEADER_MACH_REQUEST_ID, rid.to_string().parse().unwrap());

    // correlation id
    // If missing or invalid, default to request-id to keep end-to-end grouping stable.
    let cid = headers
        .get(HEADER_MACH_CORRELATION_ID)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or(rid);

    headers.insert(HEADER_MACH_CORRELATION_ID, cid.to_string().parse().unwrap());

    // residency default
    if !headers.contains_key(HEADER_RESIDENCY) {
        headers.insert(HEADER_RESIDENCY, "GLOBAL".parse().unwrap());
    }

    // traceparent (W3C)
    let tp_ok = headers
        .get(HEADER_TRACEPARENT)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .and_then(TraceContext::from_traceparent)
        .is_some();

    if !tp_ok {
        let trace = TraceContext::generate();
        if let Some(tp) = trace.to_traceparent() {
            headers.insert(HEADER_TRACEPARENT, tp.parse().unwrap());
        }
    }
}

/// Standard TraceLayer for Rhelma services.
///
/// This span reads canonical IDs to keep log correlation consistent across services.
/// It is intentionally low-cardinality: `method` and `path` only (no user input).
pub fn trace_layer_v52() -> TraceLayerV52 {
    TraceLayer::new_for_http().make_span_with(make_http_span_v52)
}

/// Span factory used by [`trace_layer_v52`].
pub fn make_http_span_v52(req: &Request<Body>) -> tracing::Span {
    let method = req.method().as_str();
    let path = req.uri().path();

    let request_id = req
        .headers()
        .get(HEADER_MACH_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    let correlation_id = req
        .headers()
        .get(HEADER_MACH_CORRELATION_ID)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    let residency = req
        .headers()
        .get(HEADER_RESIDENCY)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    let mut trace_id: Option<String> = None;
    let mut span_id: Option<String> = None;
    if let Some(tp) = req
        .headers()
        .get(HEADER_TRACEPARENT)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(tc) = TraceContext::from_traceparent(tp) {
            trace_id = tc.trace_id;
            span_id = tc.span_id;
        }
    }

    let trace_id = trace_id.as_deref().unwrap_or("-");
    let span_id = span_id.as_deref().unwrap_or("-");

    tracing::info_span!(
        "http.request",
        %method,
        %path,
        request_id = %request_id,
        correlation_id = %correlation_id,
        residency = %residency,
        trace_id = %trace_id,
        span_id = %span_id,
    )
}

// ================================================================================================
// Contract v6.0 (recommended)
// ================================================================================================

/// Concrete trace layer type used by [`trace_layer_v60`].
pub type TraceLayerV60 =
    TraceLayer<SharedClassifier<ServerErrorsAsFailures>, fn(&Request<Body>) -> tracing::Span>;

/// Contract bootstrap layer (v6.0).
///
/// Responsibilities (fail-open):
/// - Ensure `x-rhelma-request-id` and `x-rhelma-correlation-id` exist (uuidv7, best-effort)
/// - Normalize `x-residency` into the v6.0 enum (default `GLOBAL`)
/// - Ensure W3C `traceparent` exists (generate if missing or invalid)
/// - Sanitize (trim / drop invalid) `tracestate`, `x-tenant-id`, `x-region` (no generation)
#[derive(Clone, Default)]
pub struct ContractV60Layer;

/// Diagnostics about the *incoming* request headers observed by [`ContractV60Layer`]
/// before normalization.
///
/// This is primarily useful for ingress services (e.g. `api-gateway`) that want to
/// **fail-closed** on malformed client-supplied values while still letting all
/// downstream middlewares see canonicalized headers.
///
/// Non-ingress services should generally ignore this and remain fail-open.
#[derive(Debug, Clone, Default)]
pub struct ContractV60Diagnostics {
    /// Client supplied an invalid `x-rhelma-request-id` (non-UUID).
    pub request_id_invalid: bool,
    /// Client supplied an invalid `x-rhelma-correlation-id` (non-UUID).
    pub correlation_id_invalid: bool,
    /// Client supplied an invalid `traceparent`.
    pub traceparent_invalid: bool,
    /// Client supplied an invalid `tracestate` (too large or contains control chars).
    pub tracestate_invalid: bool,
    /// Client supplied an invalid `x-residency` (not recognized).
    pub residency_invalid: bool,
    /// Client supplied an invalid `x-tenant-id` (too large or contains control chars).
    pub tenant_id_invalid: bool,
    /// Client supplied an invalid `x-region` (too large or contains control chars).
    pub region_invalid: bool,
}

impl<S> Layer<S> for ContractV60Layer {
    type Service = ContractV60Middleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ContractV60Middleware { inner }
    }
}

#[derive(Clone)]
/// Tower middleware that applies the Rhelma Contract v6.0 request header normalization.
///
/// This is the concrete service wrapper produced by [`ContractV60Layer`].
pub struct ContractV60Middleware<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for ContractV60Middleware<S>
where
    S: Service<Request<Body>, Response = axum::response::Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        // Capture diagnostics about the *incoming* header state before we normalize.
        // This enables ingress services to fail-closed without causing middleware drift.
        let diag = diagnose_contract_v60(req.headers());
        req.extensions_mut().insert(diag);

        ensure_contract_v60_headers(req.headers_mut());
        self.inner.call(req)
    }
}

fn diagnose_contract_v60(headers: &http::HeaderMap) -> ContractV60Diagnostics {
    let mut d = ContractV60Diagnostics::default();

    // request id
    if let Some(raw) = headers
        .get(HEADER_MACH_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if Uuid::parse_str(raw).is_err() {
            d.request_id_invalid = true;
        }
    }

    // correlation id
    if let Some(raw) = headers
        .get(HEADER_MACH_CORRELATION_ID)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if Uuid::parse_str(raw).is_err() {
            d.correlation_id_invalid = true;
        }
    }

    // residency
    if let Some(raw) = headers
        .get(HEADER_RESIDENCY)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_ascii_uppercase())
        .filter(|s| !s.is_empty())
    {
        let ok = matches!(
            raw.as_str(),
            "GLOBAL"
                | "REGIONAL_PREFERRED"
                | "REGIONALPREFERRED"
                | "PREFERRED"
                | "REGIONAL_ONLY"
                | "REGIONAL_STRICT"
                | "REGIONALSTRICT"
                | "STRICT"
                | "REGION_STRICT"
        );
        if !ok {
            d.residency_invalid = true;
        }
    }

    // traceparent
    if let Some(raw) = headers
        .get(HEADER_TRACEPARENT)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if TraceContext::from_traceparent(raw).is_none() {
            d.traceparent_invalid = true;
        }
    }

    // tracestate
    if let Some(raw) = headers
        .get(HEADER_TRACESTATE)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if raw.len() > 1024 || raw.contains('\n') || raw.contains('\r') {
            d.tracestate_invalid = true;
        }
    }

    // tenant / region
    if let Some(raw) = headers
        .get(HEADER_TENANT_ID)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if raw.len() > 256 || raw.contains('\n') || raw.contains('\r') {
            d.tenant_id_invalid = true;
        }
    }

    if let Some(raw) = headers
        .get(HEADER_REGION)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if raw.len() > 128 || raw.contains('\n') || raw.contains('\r') {
            d.region_invalid = true;
        }
    }

    d
}

// ================================================================================================
// Trace context scoping (recommended)
// ================================================================================================

/// Binds a minimal header allow-list into `rhelma_tracing` task-local context.
///
/// Use this layer so outbound HTTP calls (e.g. via `reqwest` helpers) automatically
/// propagate the correct request/correlation IDs, W3C trace headers, and
/// tenancy/location metadata without threading context explicitly.
///
/// Place this layer **inside** [`ContractV60Layer`] so headers are canonical before binding.
#[derive(Clone, Default)]
pub struct ScopeHeadersLayer;

impl<S> Layer<S> for ScopeHeadersLayer {
    type Service = ScopeHeadersMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ScopeHeadersMiddleware { inner }
    }
}

#[derive(Clone)]
/// Tower middleware produced by [`ScopeHeadersLayer`].
pub struct ScopeHeadersMiddleware<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for ScopeHeadersMiddleware<S>
where
    S: Service<Request<Body>, Response = axum::response::Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let headers = crate::extract_minimal_headers(req.headers());
        let mut inner = self.inner.clone();
        Box::pin(async move {
            rhelma_tracing::context::scope_with_headers(
                &headers,
                async move { inner.call(req).await },
            )
            .await
        })
    }
}

fn normalize_simple_header(headers: &mut http::HeaderMap, name: &'static str, max_len: usize) {
    let Some(raw) = headers.get(name).and_then(|v| v.to_str().ok()) else {
        return;
    };

    let v = raw.trim();
    if v.is_empty() || v.len() > max_len || v.contains('\n') || v.contains('\r') {
        headers.remove(name);
        return;
    }

    if v != raw {
        if let Ok(hv) = http::HeaderValue::from_str(v) {
            headers.insert(name, hv);
        }
    }
}

fn normalize_residency_v60(headers: &mut http::HeaderMap) {
    // `x-residency` is primarily a policy enum, but we allow unknown values to pass through
    // (bounded/sanitized) so gateways can propagate experimental/custom values without clobbering.
    //
    // We still canonicalize known synonyms into the v6.0 enum.
    let raw = headers
        .get(HEADER_RESIDENCY)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        // Safety: keep the value bounded and prevent header injection.
        .filter(|s| s.len() <= 128 && !s.contains('\n') && !s.contains('\r'));

    let norm: String = match raw {
        None => "GLOBAL".to_string(),
        Some(v) => {
            let upper = v.to_ascii_uppercase();
            match upper.as_str() {
                "GLOBAL" => "GLOBAL".to_string(),
                "REGIONAL_PREFERRED" | "REGIONALPREFERRED" | "PREFERRED" => {
                    "REGIONAL_PREFERRED".to_string()
                }
                // Back-compat (v5.x)
                "REGIONAL_ONLY" => "REGIONAL_PREFERRED".to_string(),
                "REGIONAL_STRICT" | "REGIONALSTRICT" | "STRICT" | "REGION_STRICT" => {
                    "REGIONAL_STRICT".to_string()
                }
                // Unknown value: preserve as-is.
                _ => v.to_string(),
            }
        }
    };

    // Always ensure a value exists, but avoid panicking on invalid header values.
    if let Ok(hv) = http::HeaderValue::from_str(&norm) {
        headers.insert(HEADER_RESIDENCY, hv);
    } else {
        headers.insert(HEADER_RESIDENCY, http::HeaderValue::from_static("GLOBAL"));
    }
}

/// Ensure Rhelma Contract v6.0 headers exist on the request.
///
/// This is exposed for services that prefer function middleware (`middleware::from_fn`)
/// rather than a `tower::Layer`.
pub fn ensure_contract_v60_headers(headers: &mut http::HeaderMap) {
    // request id
    let rid = headers
        .get(HEADER_MACH_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or_else(Uuid::now_v7);

    headers.insert(HEADER_MACH_REQUEST_ID, rid.to_string().parse().unwrap());

    // correlation id
    // If missing or invalid, default to request-id to keep end-to-end grouping stable.
    let cid = headers
        .get(HEADER_MACH_CORRELATION_ID)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or(rid);

    headers.insert(HEADER_MACH_CORRELATION_ID, cid.to_string().parse().unwrap());

    // Residency (v6.0) normalization.
    normalize_residency_v60(headers);

    // traceparent (W3C)
    let tp_ok = headers
        .get(HEADER_TRACEPARENT)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .and_then(TraceContext::from_traceparent)
        .is_some();

    if !tp_ok {
        let trace = TraceContext::generate();
        if let Some(tp) = trace.to_traceparent() {
            headers.insert(HEADER_TRACEPARENT, tp.parse().unwrap());
        }
    }

    // tracestate is pass-through, but sanitize aggressively.
    normalize_simple_header(headers, HEADER_TRACESTATE, 1024);

    // Tenant + region are required at ingress (edge/gateway) but are not generated here.
    // We only sanitize to prevent header injection and to keep spans safe.
    normalize_simple_header(headers, HEADER_TENANT_ID, 256);
    normalize_simple_header(headers, HEADER_REGION, 128);
}

/// Standard TraceLayer for Rhelma services (Contract v6.0).
///
/// This span reads canonical IDs + tenancy/location so log correlation is consistent across services.
/// It is intentionally low-cardinality in the core span name and keys.
pub fn trace_layer_v60() -> TraceLayerV60 {
    TraceLayer::new_for_http().make_span_with(make_http_span_v60)
}

/// Span factory used by [`trace_layer_v60`].
pub fn make_http_span_v60(req: &Request<Body>) -> tracing::Span {
    let method = req.method().as_str();
    let path = req.uri().path();

    let request_id = req
        .headers()
        .get(HEADER_MACH_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    let correlation_id = req
        .headers()
        .get(HEADER_MACH_CORRELATION_ID)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    let tenant_id = req
        .headers()
        .get(HEADER_TENANT_ID)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    let region = req
        .headers()
        .get(HEADER_REGION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    let residency = req
        .headers()
        .get(HEADER_RESIDENCY)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    let mut trace_id: Option<String> = None;
    let mut span_id: Option<String> = None;
    if let Some(tp) = req
        .headers()
        .get(HEADER_TRACEPARENT)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(tc) = TraceContext::from_traceparent(tp) {
            trace_id = tc.trace_id;
            span_id = tc.span_id;
        }
    }

    let trace_id = trace_id.as_deref().unwrap_or("-");
    let span_id = span_id.as_deref().unwrap_or("-");

    tracing::info_span!(
        "http.request",
        %method,
        %path,
        request_id = %request_id,
        correlation_id = %correlation_id,
        tenant_id = %tenant_id,
        region = %region,
        residency = %residency,
        trace_id = %trace_id,
        span_id = %span_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhelma_core::constants::{HEADER_MACH_CORRELATION_ID, HEADER_MACH_REQUEST_ID};

    #[test]
    fn correlation_defaults_to_request_id_when_missing_or_invalid() {
        let mut headers = http::HeaderMap::new();
        ensure_contract_v52_headers(&mut headers);

        let rid = headers
            .get(HEADER_MACH_REQUEST_ID)
            .and_then(|v| v.to_str().ok())
            .expect("rid");
        let cid = headers
            .get(HEADER_MACH_CORRELATION_ID)
            .and_then(|v| v.to_str().ok())
            .expect("cid");
        assert_eq!(rid, cid);

        // Invalid correlation id is replaced with request id.
        headers.insert(HEADER_MACH_CORRELATION_ID, "not-a-uuid".parse().unwrap());
        ensure_contract_v52_headers(&mut headers);
        let rid2 = headers
            .get(HEADER_MACH_REQUEST_ID)
            .and_then(|v| v.to_str().ok())
            .unwrap();
        let cid2 = headers
            .get(HEADER_MACH_CORRELATION_ID)
            .and_then(|v| v.to_str().ok())
            .unwrap();
        assert_eq!(rid2, cid2);
    }

    #[test]
    fn residency_is_normalized_to_v60_enum() {
        let mut headers = http::HeaderMap::new();
        headers.insert(HEADER_RESIDENCY, "REGIONAL_ONLY".parse().unwrap());
        ensure_contract_v60_headers(&mut headers);

        let res = headers
            .get(HEADER_RESIDENCY)
            .and_then(|v| v.to_str().ok())
            .unwrap();
        assert_eq!(res, "REGIONAL_PREFERRED");

        headers.insert(HEADER_RESIDENCY, "REGION_STRICT".parse().unwrap());
        ensure_contract_v60_headers(&mut headers);

        let res2 = headers
            .get(HEADER_RESIDENCY)
            .and_then(|v| v.to_str().ok())
            .unwrap();
        assert_eq!(res2, "REGIONAL_STRICT");
    }

    #[test]
    fn residency_preserves_forward_compatible_tokens() {
        let mut headers = http::HeaderMap::new();
        headers.insert(HEADER_RESIDENCY, "local".parse().unwrap());
        ensure_contract_v60_headers(&mut headers);

        let res = headers
            .get(HEADER_RESIDENCY)
            .and_then(|v| v.to_str().ok())
            .unwrap();
        assert_eq!(res, "local");

        // Trimming is applied but unknown values are preserved.
        headers.insert(HEADER_RESIDENCY, "  Local-1  ".parse().unwrap());
        ensure_contract_v60_headers(&mut headers);
        let res2 = headers
            .get(HEADER_RESIDENCY)
            .and_then(|v| v.to_str().ok())
            .unwrap();
        assert_eq!(res2, "Local-1");
    }
}
