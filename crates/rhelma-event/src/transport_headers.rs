#![forbid(unsafe_code)]
//! Transport-level observability header helpers (Contract v5.2).
//!
//! This module provides a **transport-agnostic** way to carry Rhelma context across
//! systems that support simple string headers (e.g. NATS headers, HTTP headers,
//! etc.). Kafka has its own adapter in `rhelma-event-kafka`, but uses the same
//! canonical keys.
//!
//! Goals:
//! - Prefer W3C `traceparent`.
//! - Carry `x-rhelma-request-id`, `x-rhelma-correlation-id`, and `x-residency`.
//! - Tolerate common legacy keys (`x-request-id`, `x-correlation-id`).
//!
//! The helpers are **best-effort**: invalid values are ignored and replaced with
//! sensible defaults.

use std::collections::HashMap;

use rhelma_core::constants::{
    HEADER_BAGGAGE, HEADER_MACH_CORRELATION_ID, HEADER_MACH_REQUEST_ID, HEADER_RESIDENCY,
    HEADER_TRACEPARENT, HEADER_TRACESTATE,
};

use crate::{EventRequestContext, EventRequestFlags, EventTraceContext, Residency};

const LEGACY_REQUEST_ID: &str = "x-request-id";
const LEGACY_CORRELATION_ID: &str = "x-correlation-id";
const LEGACY_TRACE_ID: &str = "x-trace-id";
const LEGACY_SPAN_ID: &str = "x-span-id";

fn is_lower_hex_len(s: &str, n: usize) -> bool {
    s.len() == n && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn parse_residency(s: &str) -> Residency {
    match s.trim().to_ascii_uppercase().as_str() {
        "GLOBAL" => Residency::Global,
        "REGIONAL_ONLY" | "REGIONALONLY" | "REGIONAL-ONLY" => Residency::RegionalOnly,
        "REGION_STRICT" | "REGIONSTRICT" | "REGION-STRICT" => Residency::RegionStrict,
        _ => Residency::Global,
    }
}

fn residency_to_str(r: Residency) -> &'static str {
    match r {
        Residency::Global => "GLOBAL",
        Residency::RegionalOnly => "REGIONAL_ONLY",
        Residency::RegionStrict => "REGION_STRICT",
    }
}

/// Build transport headers from (request, trace, residency).
///
/// This is suitable for NATS headers (after conversion) or any string-key/value carrier.
pub fn headers_from_context(
    request: &EventRequestContext,
    trace: &EventTraceContext,
    residency: Residency,
) -> HashMap<String, String> {
    let mut h = HashMap::new();

    if let Some(rid) = request.request_id.as_deref() {
        h.insert(HEADER_MACH_REQUEST_ID.to_string(), rid.to_string());
        // legacy mirror
        h.insert(LEGACY_REQUEST_ID.to_string(), rid.to_string());
    }
    if let Some(cid) = request.correlation_id.as_deref() {
        h.insert(HEADER_MACH_CORRELATION_ID.to_string(), cid.to_string());
        h.insert(LEGACY_CORRELATION_ID.to_string(), cid.to_string());
    }

    h.insert(
        HEADER_RESIDENCY.to_string(),
        residency_to_str(residency).to_string(),
    );

    // Prefer W3C traceparent if we have both trace_id + span_id.
    if let (Some(tid), Some(sid)) = (trace.trace_id.as_deref(), trace.span_id.as_deref()) {
        if is_lower_hex_len(tid, 32) && is_lower_hex_len(sid, 16) {
            let tp = format!("00-{tid}-{sid}-01");
            h.insert(HEADER_TRACEPARENT.to_string(), tp);
        }
    }

    // tracestate (optional; W3C).
    if let Some(ts) = trace.tracestate.as_deref() {
        if !ts.trim().is_empty() {
            h.insert(HEADER_TRACESTATE.to_string(), ts.trim().to_string());
        }
    }

    // baggage (optional; W3C). Keep bounded to avoid large headers.
    if let Some(bg) = trace.baggage.as_deref() {
        let bg = bg.trim();
        if !bg.is_empty() && bg.len() <= 2048 {
            h.insert(HEADER_BAGGAGE.to_string(), bg.to_string());
        }
    }

    // Legacy trace mirrors (best-effort).
    if let Some(tid) = trace.trace_id.as_deref() {
        h.insert(LEGACY_TRACE_ID.to_string(), tid.to_string());
    }
    if let Some(sid) = trace.span_id.as_deref() {
        h.insert(LEGACY_SPAN_ID.to_string(), sid.to_string());
    }

    h
}

/// Extract (request, trace, residency) from transport headers.
///
/// Accepts canonical keys and common legacy keys.
pub fn extract_context_from_headers(
    headers: &HashMap<String, String>,
) -> (EventRequestContext, EventTraceContext, Residency) {
    let mut req = EventRequestContext {
        request_id: None,
        correlation_id: None,
        tenant_id: None,
        user_id: None,
        flags: EventRequestFlags::default(),
    };

    let mut trace = EventTraceContext {
        trace_id: None,
        span_id: None,
        tracestate: None,
        baggage: None,
        parent_span_id: None,
    };

    let mut residency = Residency::Global;

    // request/correlation/residency
    for (k, v) in headers.iter() {
        let key = k.trim();
        let val = v.trim();

        if key.eq_ignore_ascii_case(HEADER_MACH_REQUEST_ID)
            || key.eq_ignore_ascii_case(LEGACY_REQUEST_ID)
        {
            if req.request_id.is_none() {
                req.request_id = Some(val.to_string());
            }
            continue;
        }
        if key.eq_ignore_ascii_case(HEADER_MACH_CORRELATION_ID)
            || key.eq_ignore_ascii_case(LEGACY_CORRELATION_ID)
        {
            if req.correlation_id.is_none() {
                req.correlation_id = Some(val.to_string());
            }
            continue;
        }
        if key.eq_ignore_ascii_case(HEADER_RESIDENCY) {
            residency = parse_residency(val);
            continue;
        }
    }

    // trace (prefer traceparent)
    let traceparent = headers.iter().find_map(|(k, v)| {
        if k.trim().eq_ignore_ascii_case(HEADER_TRACEPARENT) {
            Some(v.trim())
        } else {
            None
        }
    });

    if let Some(tp) = traceparent {
        let parts: Vec<&str> = tp.split('-').collect();
        if parts.len() == 4 {
            let tid = parts[1];
            let sid = parts[2];
            if is_lower_hex_len(tid, 32) && is_lower_hex_len(sid, 16) {
                trace.trace_id = Some(tid.to_string());
                trace.span_id = Some(sid.to_string());
            }
        }
    }

    // legacy fallback if trace still missing
    if trace.trace_id.is_none() {
        if let Some(tid) = headers.iter().find_map(|(k, v)| {
            if k.trim().eq_ignore_ascii_case(LEGACY_TRACE_ID) {
                Some(v.trim())
            } else {
                None
            }
        }) {
            if is_lower_hex_len(tid, 32) {
                trace.trace_id = Some(tid.to_string());
            }
        }
    }
    if trace.span_id.is_none() {
        if let Some(sid) = headers.iter().find_map(|(k, v)| {
            if k.trim().eq_ignore_ascii_case(LEGACY_SPAN_ID) {
                Some(v.trim())
            } else {
                None
            }
        }) {
            if is_lower_hex_len(sid, 16) {
                trace.span_id = Some(sid.to_string());
            }
        }
    }

    // W3C tracestate (optional).
    if let Some(ts) = headers.iter().find_map(|(k, v)| {
        if k.trim().eq_ignore_ascii_case(HEADER_TRACESTATE) {
            Some(v.trim())
        } else {
            None
        }
    }) {
        if !ts.is_empty() && ts.len() <= 1024 {
            trace.tracestate = Some(ts.to_string());
        }
    }

    // W3C baggage (optional). Keep bounded.
    if let Some(bg) = headers.iter().find_map(|(k, v)| {
        if k.trim().eq_ignore_ascii_case(HEADER_BAGGAGE) {
            Some(v.trim())
        } else {
            None
        }
    }) {
        if !bg.is_empty() && bg.len() <= 2048 {
            trace.baggage = Some(bg.to_string());
        }
    }

    (req, trace, residency)
}
