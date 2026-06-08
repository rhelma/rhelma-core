#![forbid(unsafe_code)]

use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use rhelma_event::{
    purpose, EventEnvelope, EventRequestContext, EventSource, EventTraceContext, PolicyMeta,
    Residency,
};
use rhelma_event_kafka::{extract_context_from_kafka_headers, kafka_headers_from_envelope};

#[test]
fn transport_context_roundtrip_preserves_ids_and_residency() {
    let rid = Uuid::now_v7().to_string();
    let cid = Uuid::now_v7().to_string();

    let inbound = EventEnvelope {
        event_id: Uuid::now_v7().to_string(),
        event_version: 1,
        topic: "ai.test.in".to_string(),
        key: None,
        timestamp: Utc::now(),
        published_at: Utc::now(),
        source: EventSource::new("ai-orchestrator", "0.0.0-test", "eu-test-1"),
        request: EventRequestContext {
            request_id: Some(rid.clone()),
            correlation_id: Some(cid.clone()),
            tenant_id: Some("tenant-test".to_string()),
            user_id: None,
            flags: Default::default(),
        },
        trace: EventTraceContext {
            trace_id: Some("0123456789abcdef0123456789abcdef".to_string()),
            span_id: Some("0123456789abcdef".to_string()),
            tracestate: None,
            baggage: None,
            parent_span_id: None,
        },
        payload: json!({"ok": true}),
        payload_type: "test".to_string(),
        schema_ref: "ai.test.in.v1".to_string(),
        policy: PolicyMeta::public(purpose::KAFKA),
        residency: Residency::RegionStrict,
        encryption: None,
        signature: None,
        hash: None,
    }
    .finalize_strict()
    .expect("inbound finalize_strict");

    let inbound_headers = kafka_headers_from_envelope(&inbound);
    let (req_ctx, trace_ctx, res) = extract_context_from_kafka_headers(&inbound_headers);

    assert_eq!(req_ctx.request_id.as_deref(), Some(rid.as_str()));
    assert_eq!(req_ctx.correlation_id.as_deref(), Some(cid.as_str()));
    assert_eq!(req_ctx.tenant_id.as_deref(), Some("tenant-test"));
    assert_eq!(
        trace_ctx.trace_id.as_deref(),
        inbound.trace.trace_id.as_deref()
    );
    assert_eq!(
        trace_ctx.span_id.as_deref(),
        inbound.trace.span_id.as_deref()
    );
    assert_eq!(res, Residency::RegionStrict);

    // Simulate a worker producing a result event:
    // - Request/correlation MUST be preserved
    // - Trace MUST preserve trace_id and create a child span
    let out_request = EventRequestContext::inherit_or_generate(&req_ctx);
    let out_trace = EventTraceContext::child_of(&trace_ctx);

    assert_eq!(out_request.request_id.as_deref(), Some(rid.as_str()));
    assert_eq!(out_request.correlation_id.as_deref(), Some(cid.as_str()));
    assert_eq!(
        out_trace.trace_id.as_deref(),
        inbound.trace.trace_id.as_deref()
    );
    assert_ne!(
        out_trace.span_id.as_deref(),
        inbound.trace.span_id.as_deref()
    );
    assert_eq!(
        out_trace.parent_span_id.as_deref(),
        inbound.trace.span_id.as_deref()
    );

    let outbound = EventEnvelope {
        event_id: Uuid::now_v7().to_string(),
        event_version: 1,
        topic: "ai.test.out".to_string(),
        key: None,
        timestamp: Utc::now(),
        published_at: Utc::now(),
        source: EventSource::new("patch-applier", "0.0.0-test", "eu-test-1"),
        request: out_request,
        trace: out_trace,
        payload: json!({"ok": true}),
        payload_type: "test".to_string(),
        schema_ref: "ai.test.out.v1".to_string(),
        policy: PolicyMeta::public(purpose::KAFKA),
        residency: res,
        encryption: None,
        signature: None,
        hash: None,
    }
    .finalize_strict()
    .expect("outbound finalize_strict");

    let outbound_headers = kafka_headers_from_envelope(&outbound);
    let (req2, trace2, res2) = extract_context_from_kafka_headers(&outbound_headers);

    assert_eq!(req2.request_id.as_deref(), Some(rid.as_str()));
    assert_eq!(req2.correlation_id.as_deref(), Some(cid.as_str()));
    assert_eq!(
        trace2.trace_id.as_deref(),
        outbound.trace.trace_id.as_deref()
    );
    assert_eq!(trace2.span_id.as_deref(), outbound.trace.span_id.as_deref());
    assert_eq!(res2, Residency::RegionStrict);
}
