#![forbid(unsafe_code)]

use std::collections::HashMap;

use opentelemetry::propagation::{BaggagePropagator, Extractor, Injector, TraceContextPropagator};
use opentelemetry::Context;
use rdkafka::message::{Header, Headers, OwnedHeaders};

use crate::context::sanitize_baggage_header_value;

/// Injector for `rdkafka::message::OwnedHeaders`.
pub struct KafkaHeaderInjector<'a> {
    headers: &'a mut OwnedHeaders,
}

impl<'a> KafkaHeaderInjector<'a> {
    /// Create a new injector.
    pub fn new(headers: &'a mut OwnedHeaders) -> Self {
        Self { headers }
    }

    fn put(&mut self, key: &str, value: &str) {
        let cur = std::mem::take(self.headers);
        *self.headers = cur.insert(Header {
            key,
            value: Some(value.as_bytes()),
        });
    }
}

impl<'a> Injector for KafkaHeaderInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        if key.trim().is_empty() {
            return;
        }
        let v = value.trim();
        if v.is_empty() {
            return;
        }
        self.put(key, v);
    }
}

/// Extractor for `rdkafka::message::Headers`.
///
/// Header keys are treated as case-insensitive.
pub struct KafkaHeaderExtractor<'a, H: Headers> {
    headers: &'a H,
    sanitized_baggage: Option<String>,
}

impl<'a, H: Headers> KafkaHeaderExtractor<'a, H> {
    /// Create a new extractor.
    pub fn new(headers: &'a H) -> Self {
        let raw_baggage = header_value_ci(headers, "baggage");
        let sanitized_baggage = raw_baggage.and_then(sanitize_baggage_header_value);
        Self {
            headers,
            sanitized_baggage,
        }
    }
}

impl<'a, H: Headers> Extractor for KafkaHeaderExtractor<'a, H> {
    fn get(&self, key: &str) -> Option<&str> {
        if key.eq_ignore_ascii_case("baggage") {
            return self.sanitized_baggage.as_deref();
        }

        for i in 0..self.headers.count() {
            let h = self.headers.get(i);
            if h.key.eq_ignore_ascii_case(key) {
                return h
                    .value
                    .and_then(|v| std::str::from_utf8(v).ok())
                    .map(str::trim)
                    .filter(|s| !s.is_empty());
            }
        }
        None
    }

    fn keys(&self) -> Vec<&str> {
        Vec::new()
    }
}

/// Inject W3C trace context and sanitized baggage into Kafka headers.
pub fn inject_trace_context(headers: &mut OwnedHeaders, cx: &Context) {
    let mut inj = KafkaHeaderInjector::new(headers);

    // 1) Inject trace context.
    TraceContextPropagator::new().inject_context(cx, &mut inj);

    // 2) Inject sanitized baggage.
    if let Some(bg) = baggage_header_from_context(cx) {
        inj.set("baggage", bg);
    }
}

/// Extract W3C trace context and sanitized baggage from Kafka headers.
pub fn extract_trace_context<H: Headers>(headers: &H) -> Context {
    let ex = KafkaHeaderExtractor::new(headers);

    // 1) Extract trace context.
    let trace_ctx = TraceContextPropagator::new().extract(&ex);

    // 2) Extract baggage into the same context.
    BaggagePropagator::new().extract_with_context(&trace_ctx, &ex)
}

fn header_value_ci<H: Headers>(headers: &H, key: &str) -> Option<String> {
    for i in 0..headers.count() {
        let h = headers.get(i);
        if h.key.eq_ignore_ascii_case(key) {
            return h
                .value
                .and_then(|v| std::str::from_utf8(v).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
        }
    }
    None
}

fn baggage_header_from_context(ctx: &Context) -> Option<String> {
    let mut injected: HashMap<String, String> = HashMap::new();

    struct MapInjector<'a>(&'a mut HashMap<String, String>);
    impl<'a> Injector for MapInjector<'a> {
        fn set(&mut self, key: &str, value: String) {
            self.0.insert(key.to_string(), value);
        }
    }

    BaggagePropagator::new().inject_context(ctx, &mut MapInjector(&mut injected));

    let raw = injected.get("baggage")?.trim();
    if raw.is_empty() || raw.len() > 4096 {
        return None;
    }

    sanitize_baggage_header_value(raw)
}
