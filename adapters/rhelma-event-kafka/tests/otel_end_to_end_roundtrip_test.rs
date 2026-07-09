#![forbid(unsafe_code)]

// End-to-end correlation test without a real Kafka broker.
//
// Validates:
//  - Producer prefers current OTEL context for trace propagation.
//  - Producer injects W3C baggage from OTEL context.
//  - Rhelma baggage sanitization (allowlist + bounds) is enforced.
//  - Consumer extraction attaches both trace + sanitized baggage to OTEL Context.

#[cfg(feature = "otel")]
mod otel {
    use std::collections::HashMap;

    use chrono::Utc;
    use opentelemetry::propagation::{BaggagePropagator, Injector};
    use opentelemetry::trace::{TraceContextExt, TracerProvider as _};
    use opentelemetry_sdk::trace as sdktrace;
    use rdkafka::message::Headers;
    use rhelma_event::{
        EventEnvelope, EventRequestContext, EventSource, EventTraceContext, PolicyMeta, Residency,
    };
    use rhelma_event_kafka::{
        extract_context_from_kafka_headers, kafka_headers_from_envelope_prefer_current_otel,
        otel_context_from_headers_map,
    };
    use tracing_opentelemetry::OpenTelemetrySpanExt;
    use tracing_subscriber::prelude::*;

    fn minimal_env_no_baggage() -> EventEnvelope {
        EventEnvelope {
            event_id: uuid::Uuid::now_v7().to_string(),
            event_version: 1,
            topic: "obs.test".to_string(),
            key: None,
            timestamp: Utc::now(),
            published_at: Utc::now(),
            source: EventSource::new("test-producer", "0.0.0", "eu-west-1"),
            request: EventRequestContext {
                request_id: Some(uuid::Uuid::now_v7().to_string()),
                correlation_id: Some(uuid::Uuid::now_v7().to_string()),
                tenant_id: Some("t1".to_string()),
                user_id: None,
                flags: Default::default(),
            },
            // Intentionally conflicting / wrong trace data: we expect producer OTEL injection to win.
            trace: EventTraceContext {
                trace_id: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
                span_id: Some("bbbbbbbbbbbbbbbb".to_string()),
                tracestate: None,
                baggage: None,
                parent_span_id: None,
            },
            payload: serde_json::json!({"ok": true}),
            payload_type: "obs.test".to_string(),
            schema_ref: "obs.test.v1".to_string(),
            policy: PolicyMeta::public("tests"),
            residency: Residency::Global,
            encryption: None,
            signature: None,
            hash: None,
        }
    }

    fn headers_to_map<H: Headers>(headers: &H) -> HashMap<String, String> {
        let mut out = HashMap::new();
        for i in 0..headers.count() {
            if let Some(h) = headers.get(i) {
                if let Some(v) = h.value.and_then(|b| std::str::from_utf8(b).ok()) {
                    out.insert(h.key.to_string(), v.to_string());
                }
            }
        }
        out
    }

    #[test]
    fn otel_trace_and_sanitized_baggage_roundtrip_producer_to_consumer() {
        // Upstream context contains both trace + baggage, including a non-allowlisted key.
        let mut upstream = HashMap::new();
        upstream.insert(
            "traceparent".to_string(),
            "00-0123456789abcdef0123456789abcdef-0123456789abcdef-01".to_string(),
        );
        upstream.insert(
            "tracestate".to_string(),
            "rojo=00f067aa0ba902b7".to_string(),
        );
        upstream.insert(
            "baggage".to_string(),
            "rhelma.operation=credit_earn,rhelma.tenant=t1,rhelma.value.amount=42,user.email=alice@example.com"
                .to_string(),
        );

        let parent_ctx = otel_context_from_headers_map(&upstream);

        // Install a minimal OTEL tracer + tracing subscriber so spans carry OTEL context.
        let provider = sdktrace::TracerProvider::builder().build();
        let tracer = provider.tracer("rhelma-event-kafka-otel-e2e-test");

        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
        let subscriber = tracing_subscriber::registry().with(otel_layer);

        tracing::subscriber::with_default(subscriber, || {
            let span = tracing::info_span!("producer_span");
            span.set_parent(parent_ctx);

            let _guard = span.enter();

            // Producer: build Kafka headers, preferring current OTEL.
            let env = minimal_env_no_baggage();
            let kafka_headers = kafka_headers_from_envelope_prefer_current_otel(&env);

            // Quick check: producer-side headers should include sanitized baggage.
            let (_req, trace, _res) = extract_context_from_kafka_headers(&kafka_headers);
            assert_eq!(
                trace.trace_id.as_deref(),
                Some("0123456789abcdef0123456789abcdef")
            );
            let bg = trace.baggage.clone().unwrap_or_default();
            assert!(bg.contains("rhelma.operation=credit_earn"));
            assert!(bg.contains("rhelma.tenant=t1"));
            assert!(bg.contains("rhelma.value.amount=42"));
            assert!(!bg.contains("user.email"));

            // Consumer: extract OTEL context from headers map.
            let map = headers_to_map(&kafka_headers);
            let consumer_ctx = otel_context_from_headers_map(&map);

            // Parentage must match the upstream trace.
            let sc = consumer_ctx.span().span_context();
            assert!(sc.is_valid());
            assert_eq!(
                sc.trace_id().to_string(),
                "0123456789abcdef0123456789abcdef"
            );
            assert_eq!(sc.trace_state().header(), "rojo=00f067aa0ba902b7");

            // Baggage must be present, and non-allowlisted keys must be removed.
            let mut out: HashMap<String, String> = HashMap::new();
            struct MapInjector<'a>(&'a mut HashMap<String, String>);
            impl<'a> Injector for MapInjector<'a> {
                fn set(&mut self, key: &str, value: String) {
                    self.0.insert(key.to_string(), value);
                }
            }

            BaggagePropagator::new().inject_context(&consumer_ctx, &mut MapInjector(&mut out));
            let baggage = out.get("baggage").cloned().unwrap_or_default();
            assert!(baggage.contains("rhelma.operation=credit_earn"));
            assert!(baggage.contains("rhelma.tenant=t1"));
            assert!(baggage.contains("rhelma.value.amount=42"));
            assert!(!baggage.contains("user.email"));
        });

        drop(provider);
    }
}
