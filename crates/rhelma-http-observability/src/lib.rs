#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Shared HTTP observability helpers for Rhelma services (Contract v6.0).
//!
//! This crate exists to prevent "middleware drift" across services.
//!
//! Scope:
//! - **Inbound (Axum/Tower):** ensure Contract headers exist / are normalized.
//! - **Outbound (Reqwest):** propagate a small, safe header allow-list.
//! - **Tracing context:** extract a minimal header set used by `rhelma_tracing::context`.
//!
//! Principles:
//! - **Fail-open:** observability must never take a service down.
//! - **Low-cardinality:** we only propagate a stable allow-list of headers.
//! - **Zero-trust inputs:** invalid values are dropped or replaced with safe defaults.

use std::collections::HashMap;

use http::{HeaderMap, HeaderName, HeaderValue};
use rhelma_core::constants;

pub mod axum;

/// Lightweight security middleware (rate limiting + audit logging).
///
/// This module intentionally keeps its implementation dependency-light so it can be
/// used by every service without pulling in extra crates.
pub mod security;

#[cfg(feature = "reqwest")]
pub mod reqwest;

/// Extract a minimal allow-list of headers used by `rhelma_tracing` / event envelopes.
///
/// This prevents accidental high-cardinality or sensitive header propagation.
///
/// Returned keys are lowercased (matching how `rhelma_tracing::context` expects them).
#[must_use]
pub fn extract_minimal_headers(headers: &http::HeaderMap) -> HashMap<String, String> {
    // Keep this list short and stable.
    const EXTRA: [&str; 6] = [
        // Legacy compatibility
        "x-trace-id",
        "x-span-id",
        "x-request-id",
        "x-correlation-id",
        // Rhelma legacy aliases
        "x-rhelma-trace-id",
        "x-rhelma-span-id",
    ];

    let mut out = HashMap::new();

    for k in [
        constants::HEADER_TRACEPARENT,
        constants::HEADER_TRACESTATE,
        constants::HEADER_MACH_REQUEST_ID,
        constants::HEADER_MACH_CORRELATION_ID,
        constants::HEADER_TENANT_ID,
        constants::HEADER_REGION,
        constants::HEADER_RESIDENCY,
    ] {
        if let Some(v) = headers.get(k).and_then(|v| v.to_str().ok()) {
            let v = v.trim();
            if !v.is_empty() {
                out.insert(k.to_ascii_lowercase(), v.to_string());
            }
        }
    }

    for k in EXTRA {
        if let Some(v) = headers.get(k).and_then(|v| v.to_str().ok()) {
            let v = v.trim();
            if !v.is_empty() {
                out.insert(k.to_ascii_lowercase(), v.to_string());
            }
        }
    }

    out
}

/// Ensure Rhelma Contract v5.2 headers exist on the given header map.
///
/// This is a small wrapper over [`axum::ensure_contract_v52_headers`] so services and
/// HTTP clients can call a single stable helper.
///
/// Prefer [`ensure_contract_v60`] for new code.
pub fn ensure_contract_v52(headers: &mut http::HeaderMap) {
    axum::ensure_contract_v52_headers(headers);
}

/// Ensure Rhelma Contract v6.0 headers exist / are normalized on the given header map.
///
/// This is a small wrapper over [`axum::ensure_contract_v60_headers`] so services and
/// HTTP clients can call a single stable helper.
pub fn ensure_contract_v60(headers: &mut http::HeaderMap) {
    axum::ensure_contract_v60_headers(headers);
}

/// Insert canonical request/correlation IDs into headers from the current local context.
///
/// This is best-effort; if the IDs are missing, callers should follow with
/// [`ensure_contract_v60`] to generate safe defaults.
pub fn insert_request_correlation_from_context(headers: &mut HeaderMap) {
    if let Some(rid) = rhelma_tracing::context::current_request_id() {
        insert_str(headers, constants::HEADER_MACH_REQUEST_ID, &rid);
        // Legacy aliases (kept for internal compatibility).
        insert_str(headers, "x-request-id", &rid);
    }

    if let Some(cid) = rhelma_tracing::context::current_correlation_id() {
        insert_str(headers, constants::HEADER_MACH_CORRELATION_ID, &cid);
        insert_str(headers, "x-correlation-id", &cid);
    }
}

/// Insert tenant/region headers from the current local context (best-effort).
///
/// If these are missing, the caller should treat the request as "internal" or ensure
/// they are enforced at an ingress/gateway layer.
pub fn insert_tenant_region_from_context(headers: &mut HeaderMap) {
    if let Some(tid) = rhelma_tracing::context::current_tenant_id() {
        insert_str(headers, constants::HEADER_TENANT_ID, &tid);
    }

    if let Some(region) = rhelma_tracing::context::current_region() {
        insert_str(headers, constants::HEADER_REGION, &region);
    }
}

/// Insert residency policy header from the current local context (best-effort).
///
/// If residency is missing, callers should follow with [`ensure_contract_v60`] to set
/// the default `GLOBAL` policy.
pub fn insert_residency_from_context(headers: &mut HeaderMap) {
    if let Some(res) = rhelma_tracing::context::current_residency() {
        insert_str(headers, constants::HEADER_RESIDENCY, &res);
        // Legacy alias.
        insert_str(headers, "x-rhelma-residency", &res);
    }
}

fn insert_str(headers: &mut HeaderMap, name: &str, value: &str) {
    let Ok(hn) = HeaderName::from_bytes(name.as_bytes()) else {
        return;
    };
    let Ok(hv) = HeaderValue::from_str(value) else {
        return;
    };

    // Do not clobber explicit values set by the caller.
    if !headers.contains_key(&hn) {
        headers.insert(hn, hv);
    }
}

/// Internal helpers used when OTEL is not available.
mod fallback {
    use super::*;

    pub fn inject_trace_headers_from_context(headers: &mut HeaderMap) {
        if let Some(tp) = rhelma_tracing::context::current_traceparent() {
            insert_str(headers, constants::HEADER_TRACEPARENT, &tp);
        }
        if let Some(ts) = rhelma_tracing::context::current_tracestate() {
            insert_str(headers, constants::HEADER_TRACESTATE, &ts);
        }
    }
}

/// Optional OpenTelemetry propagation helpers.
#[cfg(feature = "otel")]
mod otel {
    use super::*;

    /// Inject OTEL headers using the current span context.
    ///
    /// Returns true if at least one header was injected.
    pub fn try_inject_otel_headers(headers: &mut HeaderMap) -> bool {
        let cx = tracing_opentelemetry::OpenTelemetrySpanExt::context(&tracing::Span::current());

        let mut injector = HeaderMapInjector(headers);

        opentelemetry::global::get_text_map_propagator(|prop| {
            prop.inject_context(&cx, &mut injector);
        });

        !headers.is_empty()
    }

    struct HeaderMapInjector<'a>(&'a mut HeaderMap);

    impl<'a> opentelemetry::propagation::Injector for HeaderMapInjector<'a> {
        fn set(&mut self, key: &str, value: String) {
            let Ok(hn) = HeaderName::from_bytes(key.as_bytes()) else {
                return;
            };
            let Ok(hv) = HeaderValue::from_str(&value) else {
                return;
            };
            if !self.0.contains_key(&hn) {
                self.0.insert(hn, hv);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_headers_is_allow_listed() {
        let mut h = http::HeaderMap::new();
        h.insert(
            constants::HEADER_TRACEPARENT,
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
                .parse()
                .unwrap(),
        );
        h.insert(
            constants::HEADER_MACH_REQUEST_ID,
            "00000000-0000-0000-0000-000000000000".parse().unwrap(),
        );
        h.insert(constants::HEADER_TENANT_ID, "tenant-a".parse().unwrap());
        h.insert(constants::HEADER_REGION, "eu-west".parse().unwrap());
        h.insert("authorization", "Bearer secret".parse().unwrap());

        let m = extract_minimal_headers(&h);
        assert!(m.contains_key(constants::HEADER_TRACEPARENT));
        assert!(m.contains_key(constants::HEADER_MACH_REQUEST_ID));
        assert!(m.contains_key(constants::HEADER_TENANT_ID));
        assert!(m.contains_key(constants::HEADER_REGION));
        assert!(!m.contains_key("authorization"));
    }
}
