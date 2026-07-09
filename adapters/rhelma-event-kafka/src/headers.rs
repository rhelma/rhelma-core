#![forbid(unsafe_code)]

use rdkafka::message::{Header, Headers, OwnedHeaders};

use std::collections::HashMap;

#[cfg(feature = "otel")]
use opentelemetry::propagation::{
    BaggagePropagator, Extractor, Injector, TextMapPropagator, TraceContextPropagator,
};
#[cfg(feature = "otel")]
use opentelemetry::Context;
#[cfg(feature = "otel")]
use tracing_opentelemetry::OpenTelemetrySpanExt;

use rhelma_event::{EventEnvelope, EventRequestContext, EventTraceContext, Residency};

/// Build Kafka headers from a v5.2 `EventEnvelope`.
///
/// Alignment goals (v5.2):
/// - Prefer W3C propagation (`traceparent`).
/// - Emit legacy trace headers for compatibility.
/// - Emit request/correlation/tenant/region/residency for debugging and routing.
pub fn kafka_headers_from_envelope(env: &EventEnvelope) -> OwnedHeaders {
    let mut h = OwnedHeaders::new();

    // W3C traceparent (canonical)
    if let (Some(trace_id), Some(span_id)) =
        (env.trace.trace_id.as_deref(), env.trace.span_id.as_deref())
    {
        // Match rhelma-tracing default: sampled=true => flags "01".
        // NOTE: sampled is not carried on the envelope; defaulting to "01" is consistent with rhelma-tracing.
        let tp = format!("00-{trace_id}-{span_id}-01");
        h = h.insert(Header {
            key: "traceparent",
            value: Some(tp.as_bytes()),
        });

        // Optional W3C tracestate
        if let Some(ts) = env.trace.tracestate.as_deref() {
            h = h.insert(Header {
                key: "tracestate",
                value: Some(ts.as_bytes()),
            });
        }

        // Optional W3C baggage (bounded)
        if let Some(bg) = env.trace.baggage.as_deref() {
            let bg = bg.trim();
            if !bg.is_empty() && bg.len() <= 2048 {
                h = h.insert(Header {
                    key: "baggage",
                    value: Some(bg.as_bytes()),
                });
            }
        }

        // Legacy compatibility headers
        h = h.insert(Header {
            key: "x-trace-id",
            value: Some(trace_id.as_bytes()),
        });
        h = h.insert(Header {
            key: "x-rhelma-trace-id",
            value: Some(trace_id.as_bytes()),
        });
        h = h.insert(Header {
            key: "x-span-id",
            value: Some(span_id.as_bytes()),
        });
        h = h.insert(Header {
            key: "x-rhelma-span-id",
            value: Some(span_id.as_bytes()),
        });
    }

    // Request + correlation (Contract v5.2 canonical keys)
    if let Some(rid) = env.request.request_id.as_deref() {
        // Canonical
        h = h.insert(Header {
            key: "x-rhelma-request-id",
            value: Some(rid.as_bytes()),
        });
        // Legacy compatibility (some older services and proxies)
        h = h.insert(Header {
            key: "x-request-id",
            value: Some(rid.as_bytes()),
        });
    }
    if let Some(cid) = env.request.correlation_id.as_deref() {
        // Canonical
        h = h.insert(Header {
            key: "x-rhelma-correlation-id",
            value: Some(cid.as_bytes()),
        });
        // Legacy compatibility
        h = h.insert(Header {
            key: "x-correlation-id",
            value: Some(cid.as_bytes()),
        });
    }

    // Residency (routing + audit)
    h = h.insert(Header {
        key: "x-residency",
        value: Some(env.residency.as_str().as_bytes()),
    });

    // Tenant + region are widely used across Rhelma layers.
    if let Some(tid) = env.request.tenant_id.as_deref() {
        h = h.insert(Header {
            key: "x-tenant-id",
            value: Some(tid.as_bytes()),
        });
    }
    if !env.source.region.trim().is_empty() {
        h = h.insert(Header {
            key: "x-region",
            value: Some(env.source.region.as_bytes()),
        });
    }

    // Optional: event identity hints (useful for DLQ/replay/debug tooling).
    if !env.event_id.trim().is_empty() {
        h = h.insert(Header {
            key: "x-rhelma-event-id",
            value: Some(env.event_id.as_bytes()),
        });
    }
    if !env.schema_ref.trim().is_empty() {
        h = h.insert(Header {
            key: "x-rhelma-schema-ref",
            value: Some(env.schema_ref.as_bytes()),
        });
    }

    h
}

/// Build Kafka headers from an envelope, but **prefer the current OpenTelemetry trace context**
/// (when the `otel` feature is enabled and the service installed `tracing-opentelemetry`).
///
/// Why this exists:
/// - Some services may publish events without fully mirroring OTEL context into the envelope.
/// - Even when they do, OTEL's propagator encodes flags consistently (sampling, etc.).
///
/// When no OTEL context is available, this falls back to `kafka_headers_from_envelope`.
pub fn kafka_headers_from_envelope_prefer_current_otel(env: &EventEnvelope) -> OwnedHeaders {
    #[cfg(feature = "otel")]
    {
        if let Some(h) = kafka_headers_from_current_otel(env) {
            return h;
        }
    }

    kafka_headers_from_envelope(env)
}

#[cfg(feature = "otel")]
pub fn otel_trace_headers_from_context(ctx: &Context) -> Option<(String, Option<String>)> {
    let mut injected: HashMap<String, String> = HashMap::new();

    struct MapInjector<'a>(&'a mut HashMap<String, String>);
    impl<'a> Injector for MapInjector<'a> {
        fn set(&mut self, key: &str, value: String) {
            self.0.insert(key.to_string(), value);
        }
    }

    TraceContextPropagator::new().inject_context(ctx, &mut MapInjector(&mut injected));

    let tp = injected.get("traceparent")?.trim().to_string();
    if tp.is_empty() || tp.len() > 256 {
        return None;
    }

    // tracestate is optional
    let ts = injected
        .get("tracestate")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s.len() <= 1024);

    Some((tp, ts))
}

#[cfg(feature = "otel")]
pub fn otel_baggage_header_from_context(ctx: &Context) -> Option<String> {
    let mut injected: HashMap<String, String> = HashMap::new();

    struct MapInjector<'a>(&'a mut HashMap<String, String>);
    impl<'a> Injector for MapInjector<'a> {
        fn set(&mut self, key: &str, value: String) {
            self.0.insert(key.to_string(), value);
        }
    }

    // Only inject baggage (not trace) so we can prefer trace data from TraceContextPropagator.
    BaggagePropagator::new().inject_context(ctx, &mut MapInjector(&mut injected));

    let raw = injected.get("baggage")?.trim();
    if raw.is_empty() || raw.len() > 4096 {
        return None;
    }

    // Enforce Rhelma bounds + allowlist so baggage stays low-cardinality and non-PII.
    rhelma_tracing::context::sanitize_baggage_header_value(raw)
}

#[cfg(feature = "otel")]
fn kafka_headers_from_current_otel(env: &EventEnvelope) -> Option<OwnedHeaders> {
    // Extract OTEL context from the current tracing span. If the application didn't install
    // tracing-opentelemetry, this will be empty and injection will not yield traceparent.
    let ctx: Context = tracing::Span::current().context();

    let (tp, tracestate) = otel_trace_headers_from_context(&ctx)?;
    let (trace_id, span_id) = parse_traceparent(&tp)?;

    let mut h = OwnedHeaders::new();

    // Canonical W3C
    h = h.insert(Header {
        key: "traceparent",
        value: Some(tp.as_bytes()),
    });

    if let Some(ts) = tracestate.as_deref() {
        let ts = ts.trim();
        if !ts.is_empty() && ts.len() <= 1024 {
            h = h.insert(Header {
                key: "tracestate",
                value: Some(ts.as_bytes()),
            });
        }
    }

    // Optional W3C baggage.
    //
    // Priority order:
    //  1) Envelope baggage (already Rhelma-sanitized by rhelma-tracing publish helpers)
    //  2) Current OTEL baggage (sanitized/allowlisted here)
    let mut baggage: Option<String> = env
        .trace
        .baggage
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && s.len() <= 2048)
        .map(|s| s.to_string());

    if baggage.is_none() {
        baggage = otel_baggage_header_from_context(&ctx);
    }

    if let Some(bg) = baggage.as_deref() {
        let bg = bg.trim();
        if !bg.is_empty() && bg.len() <= 2048 {
            h = h.insert(Header {
                key: "baggage",
                value: Some(bg.as_bytes()),
            });
        }
    }

    // Legacy compatibility headers from traceparent
    h = h.insert(Header {
        key: "x-trace-id",
        value: Some(trace_id.as_bytes()),
    });
    h = h.insert(Header {
        key: "x-rhelma-trace-id",
        value: Some(trace_id.as_bytes()),
    });
    h = h.insert(Header {
        key: "x-span-id",
        value: Some(span_id.as_bytes()),
    });
    h = h.insert(Header {
        key: "x-rhelma-span-id",
        value: Some(span_id.as_bytes()),
    });

    // Request + correlation (Contract v5.2 canonical keys)
    if let Some(rid) = env.request.request_id.as_deref() {
        h = h.insert(Header {
            key: "x-rhelma-request-id",
            value: Some(rid.as_bytes()),
        });
        h = h.insert(Header {
            key: "x-request-id",
            value: Some(rid.as_bytes()),
        });
    }
    if let Some(cid) = env.request.correlation_id.as_deref() {
        h = h.insert(Header {
            key: "x-rhelma-correlation-id",
            value: Some(cid.as_bytes()),
        });
        h = h.insert(Header {
            key: "x-correlation-id",
            value: Some(cid.as_bytes()),
        });
    }

    // Residency
    h = h.insert(Header {
        key: "x-residency",
        value: Some(env.residency.as_str().as_bytes()),
    });

    if let Some(tid) = env.request.tenant_id.as_deref() {
        h = h.insert(Header {
            key: "x-tenant-id",
            value: Some(tid.as_bytes()),
        });
    }
    if !env.source.region.trim().is_empty() {
        h = h.insert(Header {
            key: "x-region",
            value: Some(env.source.region.as_bytes()),
        });
    }

    if !env.event_id.trim().is_empty() {
        h = h.insert(Header {
            key: "x-rhelma-event-id",
            value: Some(env.event_id.as_bytes()),
        });
    }
    if !env.schema_ref.trim().is_empty() {
        h = h.insert(Header {
            key: "x-rhelma-schema-ref",
            value: Some(env.schema_ref.as_bytes()),
        });
    }

    Some(h)
}

/// Extract v5.2 observability context from Kafka headers.
///
/// This is intentionally transport-focused and tolerant:
/// - Prefer canonical v5.2 keys (x-rhelma-*)
/// - Fall back to legacy keys where needed
/// - Prefer W3C `traceparent` when present
pub fn extract_context_from_kafka_headers<H: Headers>(
    headers: &H,
) -> (EventRequestContext, EventTraceContext, Residency) {
    let traceparent = header_str(headers, "traceparent");
    let tracestate = header_str(headers, "tracestate");
    let baggage = header_str(headers, "baggage");
    let (tp_trace_id, tp_span_id) = traceparent
        .as_deref()
        .and_then(parse_traceparent)
        .map(|(t, s)| (Some(t), Some(s)))
        .unwrap_or((None, None));

    let trace_id = tp_trace_id
        .or_else(|| header_str(headers, "x-trace-id"))
        .or_else(|| header_str(headers, "x-rhelma-trace-id"));

    let span_id = tp_span_id
        .or_else(|| header_str(headers, "x-span-id"))
        .or_else(|| header_str(headers, "x-rhelma-span-id"));

    let request_id =
        header_str(headers, "x-rhelma-request-id").or_else(|| header_str(headers, "x-request-id"));
    let correlation_id = header_str(headers, "x-rhelma-correlation-id")
        .or_else(|| header_str(headers, "x-correlation-id"));

    let tenant_id = header_str(headers, "x-tenant-id");

    let residency = header_str(headers, "x-residency")
        .as_deref()
        .and_then(Residency::parse)
        .unwrap_or(Residency::Global);

    let request = EventRequestContext {
        request_id,
        correlation_id,
        tenant_id,
        user_id: None,
        flags: Default::default(),
    };

    let trace = EventTraceContext {
        trace_id,
        span_id,
        tracestate,
        baggage,
        parent_span_id: None,
    };

    (request, trace, residency)
}

/// Extract an OpenTelemetry context from a case-insensitive header map.
///
/// This is useful for Kafka consumers that want their per-message spans to be
/// children of the upstream trace (end-to-end OTEL correlation).
///
/// Unlike the default global propagator (which may vary per service), this function
/// performs a **local composite extract** of:
/// - W3C Trace Context (`traceparent` + optional `tracestate`)
/// - W3C Baggage (`baggage`)
///
/// That ensures consumer spans have both parentage and any allowed baggage attached
/// in the OTEL `Context`.
#[cfg(feature = "otel")]
pub fn otel_context_from_headers_map(headers: &HashMap<String, String>) -> Context {
    // IMPORTANT:
    // We sanitize `baggage` using Rhelma's allowlist + bounds before attaching it to the
    // OpenTelemetry `Context`. This keeps baggage low-cardinality and avoids accidental
    // PII leakage into OTEL backends.
    let sanitized_baggage: Option<String> = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("baggage"))
        .and_then(|(_, v)| rhelma_tracing::context::sanitize_baggage_header_value(v));

    struct MapExtractor<'a> {
        headers: &'a HashMap<String, String>,
        sanitized_baggage: Option<String>,
    }

    impl<'a> Extractor for MapExtractor<'a> {
        fn get(&self, key: &str) -> Option<&str> {
            if key.eq_ignore_ascii_case("baggage") {
                return self.sanitized_baggage.as_deref();
            }

            self.headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v.as_str())
        }

        fn keys(&self) -> Vec<&str> {
            self.headers.keys().map(|k| k.as_str()).collect()
        }
    }

    let ex = MapExtractor {
        headers,
        sanitized_baggage,
    };

    // 1) Extract trace context (traceparent/tracestate)
    let trace_ctx = TraceContextPropagator::new().extract(&ex);

    // 2) Extract baggage into the same OTEL context.
    //    This requires `TextMapPropagator` for `extract_with_context`.
    BaggagePropagator::new().extract_with_context(&trace_ctx, &ex)
}

fn header_str<H: Headers>(headers: &H, key: &str) -> Option<String> {
    for i in 0..headers.count() {
        let h = headers.get(i);
        if h.key == key {
            return h
                .value
                .and_then(|v| std::str::from_utf8(v).ok())
                .map(|s| s.to_string());
        }
    }
    None
}

fn parse_traceparent(tp: &str) -> Option<(String, String)> {
    // Expected: "00-<32hex trace_id>-<16hex span_id>-<2hex flags>"
    let parts: Vec<&str> = tp.split('-').collect();
    if parts.len() != 4 {
        return None;
    }
    let trace_id = parts[1];
    let span_id = parts[2];
    if trace_id.len() != 32 || span_id.len() != 16 {
        return None;
    }
    Some((trace_id.to_string(), span_id.to_string()))
}

/// Build a normalized header map suitable for `rhelma_tracing::context::scope_with_headers`.
///
/// This map uses the canonical W3C keys (`traceparent`, `tracestate`, `baggage`) plus
/// Rhelma v5.2+ canonical keys (`x-rhelma-*`) for request/correlation and routing.
pub fn context_headers_map_from_envelope(env: &EventEnvelope) -> HashMap<String, String> {
    let mut out = HashMap::new();

    // W3C traceparent (canonical)
    if let (Some(t), Some(s)) = (env.trace.trace_id.as_deref(), env.trace.span_id.as_deref()) {
        out.insert("traceparent".into(), format!("00-{t}-{s}-01"));

        if let Some(ts) = env.trace.tracestate.as_deref() {
            let ts = ts.trim();
            if !ts.is_empty() && ts.len() <= 1024 {
                out.insert("tracestate".into(), ts.to_string());
            }
        }

        if let Some(bg) = env.trace.baggage.as_deref() {
            let bg = bg.trim();
            if !bg.is_empty() && bg.len() <= 2048 {
                out.insert("baggage".into(), bg.to_string());
            }
        }

        // Legacy compatibility headers
        out.insert("x-trace-id".into(), t.to_string());
        out.insert("x-rhelma-trace-id".into(), t.to_string());
        out.insert("x-span-id".into(), s.to_string());
        out.insert("x-rhelma-span-id".into(), s.to_string());
    }

    if let Some(rid) = env.request.request_id.as_deref() {
        out.insert("x-rhelma-request-id".into(), rid.to_string());
        out.insert("x-request-id".into(), rid.to_string());
    }
    if let Some(cid) = env.request.correlation_id.as_deref() {
        out.insert("x-rhelma-correlation-id".into(), cid.to_string());
        out.insert("x-correlation-id".into(), cid.to_string());
    }

    out.insert("x-residency".into(), env.residency.as_str().to_string());

    if let Some(tid) = env.request.tenant_id.as_deref() {
        out.insert("x-tenant-id".into(), tid.to_string());
    }
    if !env.source.region.trim().is_empty() {
        out.insert("x-region".into(), env.source.region.trim().to_string());
    }

    out
}

/// Build a normalized header map for task-scoped correlation, preferring Kafka headers
/// but falling back to envelope fields when headers are missing.
pub fn context_headers_map_from_kafka_headers_and_envelope<H: Headers>(
    headers: &H,
    env: &EventEnvelope,
) -> HashMap<String, String> {
    let mut out = HashMap::new();

    // Prefer W3C if present.
    if let Some(tp) = header_str(headers, "traceparent") {
        let tp = tp.trim();
        if !tp.is_empty() && tp.len() <= 256 {
            out.insert("traceparent".into(), tp.to_string());
        }
    }
    if let Some(ts) = header_str(headers, "tracestate") {
        let ts = ts.trim();
        if !ts.is_empty() && ts.len() <= 1024 {
            out.insert("tracestate".into(), ts.to_string());
        }
    }
    if let Some(bg) = header_str(headers, "baggage") {
        let bg = bg.trim();
        if !bg.is_empty() && bg.len() <= 2048 {
            out.insert("baggage".into(), bg.to_string());
        }
    }

    // Pull a minimal set of Rhelma headers for correlation/routing.
    for k in [
        "x-trace-id",
        "x-rhelma-trace-id",
        "x-span-id",
        "x-rhelma-span-id",
        "x-rhelma-request-id",
        "x-request-id",
        "x-rhelma-correlation-id",
        "x-correlation-id",
        "x-residency",
        "x-tenant-id",
        "x-region",
    ] {
        if let Some(v) = header_str(headers, k) {
            let v = v.trim();
            if !v.is_empty() && v.len() <= 2048 {
                out.insert(k.to_string(), v.to_string());
            }
        }
    }

    // Fill missing from envelope (strong contract).
    if !out.contains_key("traceparent") {
        if let (Some(t), Some(s)) = (env.trace.trace_id.as_deref(), env.trace.span_id.as_deref()) {
            out.insert("traceparent".into(), format!("00-{t}-{s}-01"));
        }
    }
    if !out.contains_key("tracestate") {
        if let Some(ts) = env.trace.tracestate.as_deref() {
            let ts = ts.trim();
            if !ts.is_empty() && ts.len() <= 1024 {
                out.insert("tracestate".into(), ts.to_string());
            }
        }
    }
    if !out.contains_key("baggage") {
        if let Some(bg) = env.trace.baggage.as_deref() {
            let bg = bg.trim();
            if !bg.is_empty() && bg.len() <= 2048 {
                out.insert("baggage".into(), bg.to_string());
            }
        }
    }

    if !out.contains_key("x-rhelma-request-id") {
        if let Some(rid) = env.request.request_id.as_deref() {
            out.insert("x-rhelma-request-id".into(), rid.to_string());
            out.insert("x-request-id".into(), rid.to_string());
        }
    }
    if !out.contains_key("x-rhelma-correlation-id") {
        if let Some(cid) = env.request.correlation_id.as_deref() {
            out.insert("x-rhelma-correlation-id".into(), cid.to_string());
            out.insert("x-correlation-id".into(), cid.to_string());
        }
    }
    if !out.contains_key("x-residency") {
        out.insert("x-residency".into(), env.residency.as_str().to_string());
    }
    if !out.contains_key("x-tenant-id") {
        if let Some(tid) = env.request.tenant_id.as_deref() {
            out.insert("x-tenant-id".into(), tid.to_string());
        }
    }
    if !out.contains_key("x-region") && !env.source.region.trim().is_empty() {
        out.insert("x-region".into(), env.source.region.trim().to_string());
    }

    // If legacy trace headers are missing, derive them from traceparent.
    if (!out.contains_key("x-trace-id") || !out.contains_key("x-span-id"))
        && out.contains_key("traceparent")
    {
        if let Some(tp) = out.get("traceparent") {
            if let Some((t, s)) = parse_traceparent(tp) {
                out.entry("x-trace-id".into()).or_insert(t.clone());
                out.entry("x-rhelma-trace-id".into()).or_insert(t);
                out.entry("x-span-id".into()).or_insert(s.clone());
                out.entry("x-rhelma-span-id".into()).or_insert(s);
            }
        }
    }

    out
}
