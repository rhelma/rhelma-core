#![forbid(unsafe_code)]

use chrono::Utc;
use serde_json::json;

use rhelma_event::{
    EventEnvelope, EventRequestContext, EventSource, EventTraceContext, PolicyMeta, Residency,
};
use rhelma_event_kafka::{
    context_headers_map_from_envelope, context_headers_map_from_kafka_headers_and_envelope,
    kafka_headers_from_envelope,
};

#[tokio::test]
async fn kafka_headers_bind_task_local_trace_context() {
    // Arrange: a well-formed envelope (ids are fixed to make assertions stable).
    let env = EventEnvelope {
        event_id: uuid::Uuid::new_v4().to_string(),
        event_version: 1,
        topic: "test.topic".to_string(),
        key: None,
        timestamp: Utc::now(),
        published_at: Utc::now(),
        source: EventSource::new("test-svc", "0.0.0", "eu-west-1"),
        request: EventRequestContext {
            request_id: Some(uuid::Uuid::new_v4().to_string()),
            correlation_id: Some(uuid::Uuid::new_v4().to_string()),
            tenant_id: Some("t1".to_string()),
            user_id: None,
            flags: Default::default(),
        },
        trace: EventTraceContext {
            trace_id: Some("0123456789abcdef0123456789abcdef".to_string()),
            span_id: Some("0123456789abcdef".to_string()),
            tracestate: Some("rojo=00f067aa0ba902b7".to_string()),
            baggage: Some("rhelma.operation=credit_earn".to_string()),
            parent_span_id: None,
        },
        payload: json!({"ok": true}),
        payload_type: "test".to_string(),
        schema_ref: "test.v1".to_string(),
        policy: PolicyMeta::default(),
        residency: Residency::Global,
        encryption: None,
        signature: None,
        hash: None,
    };

    let headers = kafka_headers_from_envelope(&env);
    let ctx_headers = context_headers_map_from_kafka_headers_and_envelope(&headers, &env);

    // Act + Assert: within scope, rhelma-tracing exposes the expected values.
    rhelma_tracing::context::scope_with_headers(&ctx_headers, async move {
        assert_eq!(
            rhelma_tracing::context::current_traceparent().as_deref(),
            Some("00-0123456789abcdef0123456789abcdef-0123456789abcdef-01")
        );
        assert_eq!(
            rhelma_tracing::context::current_tracestate().as_deref(),
            Some("rojo=00f067aa0ba902b7")
        );
        assert_eq!(
            rhelma_tracing::context::current_baggage().as_deref(),
            Some("rhelma.operation=credit_earn")
        );
    })
    .await;
}

#[tokio::test]
async fn fallback_to_envelope_when_kafka_headers_missing() {
    let env = EventEnvelope {
        event_id: uuid::Uuid::new_v4().to_string(),
        event_version: 1,
        topic: "test.topic".to_string(),
        key: None,
        timestamp: Utc::now(),
        published_at: Utc::now(),
        source: EventSource::new("test-svc", "0.0.0", "eu-west-1"),
        request: EventRequestContext {
            request_id: Some(uuid::Uuid::new_v4().to_string()),
            correlation_id: Some(uuid::Uuid::new_v4().to_string()),
            tenant_id: None,
            user_id: None,
            flags: Default::default(),
        },
        trace: EventTraceContext {
            trace_id: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
            span_id: Some("bbbbbbbbbbbbbbbb".to_string()),
            tracestate: None,
            baggage: None,
            parent_span_id: None,
        },
        payload: json!({}),
        payload_type: "test".to_string(),
        schema_ref: "test.v1".to_string(),
        policy: PolicyMeta::default(),
        residency: Residency::Global,
        encryption: None,
        signature: None,
        hash: None,
    };

    let ctx_headers = context_headers_map_from_envelope(&env);

    rhelma_tracing::context::scope_with_headers(&ctx_headers, async move {
        assert_eq!(
            rhelma_tracing::context::current_traceparent().as_deref(),
            Some("00-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-bbbbbbbbbbbbbbbb-01")
        );
    })
    .await;
}
