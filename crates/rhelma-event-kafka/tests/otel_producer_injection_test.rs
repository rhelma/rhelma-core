#![forbid(unsafe_code)]

#[cfg(feature = "otel")]
mod otel {
    use std::collections::HashMap;

    use chrono::Utc;
    use opentelemetry::trace::TracerProvider as _;
    use opentelemetry_sdk::trace as sdktrace;
    use rhelma_event::{
        EventEnvelope, EventRequestContext, EventSource, EventTraceContext, PolicyMeta, Residency,
    };
    use rhelma_event_kafka::{
        extract_context_from_kafka_headers, kafka_headers_from_envelope_prefer_current_otel,
        otel_context_from_headers_map,
    };
    use tracing_opentelemetry::OpenTelemetrySpanExt;
    use tracing_subscriber::prelude::*;

    fn minimal_env_with_conflicting_trace() -> EventEnvelope {
        EventEnvelope {
            event_id: uuid::Uuid::now_v7().to_string(),
            event_version: 1,
            topic: "obs.test".to_string(),
            key: None,
            timestamp: Utc::now(),
            published_at: Utc::now(),
            source: EventSource::new("test-producer", "0.0.0", "eu-west-1"),
            request: EventRequestContext {
                request_id: None,
                correlation_id: None,
                tenant_id: None,
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

    #[test]
    fn producer_prefers_current_otel_trace_over_envelope_fields() {
        // Build a deterministic upstream parent OTEL context from a traceparent header.
        let mut upstream = HashMap::new();
        upstream.insert(
            "traceparent".to_string(),
            "00-0123456789abcdef0123456789abcdef-0123456789abcdef-01".to_string(),
        );
        let parent_ctx = otel_context_from_headers_map(&upstream);

        // Install a minimal OTEL tracer + tracing subscriber so spans carry OTEL context.
        let provider = sdktrace::TracerProvider::builder().build();
        let tracer = provider.tracer("rhelma-event-kafka-test");

        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
        let subscriber = tracing_subscriber::registry().with(otel_layer);

        tracing::subscriber::with_default(subscriber, || {
            let span = tracing::info_span!("producer_test");
            span.set_parent(parent_ctx);

            let _guard = span.enter();

            let env = minimal_env_with_conflicting_trace();
            let headers = kafka_headers_from_envelope_prefer_current_otel(&env);

            let (_req, trace, _res) = extract_context_from_kafka_headers(&headers);

            // Trace-id must match upstream OTEL trace-id, not the envelope's conflicting value.
            assert_eq!(
                trace.trace_id.as_deref(),
                Some("0123456789abcdef0123456789abcdef")
            );

            // Span-id should be present and should not be the conflicting envelope span-id.
            let sid = trace.span_id.as_deref().expect("span_id must exist");
            assert_ne!(sid, "bbbbbbbbbbbbbbbb");
            assert_eq!(sid.len(), 16);
        });

        drop(provider);
    }
}
