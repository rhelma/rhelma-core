#![forbid(unsafe_code)]

use chrono::Utc;
use rdkafka::message::{Headers, OwnedHeaders};
use rhelma_event::{
    purpose, EventEnvelope, EventRequestContext, EventRequestFlags, EventSource, EventTraceContext,
    PolicyMeta, Residency,
};
use rhelma_event_kafka::kafka_headers_from_envelope;
use serde_json::json;

fn header_val(h: &OwnedHeaders, key: &str) -> Option<String> {
    for i in 0..h.count() {
        let hdr = h.get(i);
        if hdr.key == key {
            return hdr
                .value
                .and_then(|v| std::str::from_utf8(v).ok())
                .map(|s| s.to_string());
        }
    }
    None
}

#[test]
fn kafka_headers_include_contract_v52_correlation_and_residency() {
    let rid = "018c1f23-9e4a-7b7b-9c1a-9f4b0f0a0001".to_string();
    let cid = "018c1f23-9e4a-7b7b-9c1a-9f4b0f0a0002".to_string();

    let env = EventEnvelope {
        event_id: "018c1f23-9e4a-7b7b-9c1a-9f4b0f0a0003".into(),
        event_version: 1,
        topic: "rhelma.test.headers".into(),
        key: None,

        timestamp: Utc::now(),
        published_at: Utc::now(),

        source: EventSource {
            service: "rhelma-event-kafka-test".into(),
            version: "0.0.0-test".into(),
            region: "eu".into(),
        },

        request: EventRequestContext {
            request_id: Some(rid.clone()),
            correlation_id: Some(cid.clone()),
            tenant_id: Some("t_test".into()),
            user_id: None,
            flags: EventRequestFlags::default(),
        },

        trace: EventTraceContext {
            trace_id: Some("4bf92f3577b34da6a3ce929d0e0e4736".into()),
            span_id: Some("00f067aa0ba902b7".into()),
            tracestate: None,
            baggage: None,
            parent_span_id: None,
        },

        payload: json!({"ok": true}),
        payload_type: "application/json".into(),
        schema_ref: "rhelma.test.headers@v1".into(),

        policy: PolicyMeta::public(purpose::KAFKA),
        residency: Residency::Global,
        encryption: None,
        signature: None,
        hash: None,
    };

    let headers = kafka_headers_from_envelope(&env);

    assert_eq!(
        header_val(&headers, "x-rhelma-request-id").as_deref(),
        Some(rid.as_str())
    );
    assert_eq!(
        header_val(&headers, "x-request-id").as_deref(),
        Some(rid.as_str())
    );

    assert_eq!(
        header_val(&headers, "x-rhelma-correlation-id").as_deref(),
        Some(cid.as_str())
    );
    assert_eq!(
        header_val(&headers, "x-correlation-id").as_deref(),
        Some(cid.as_str())
    );

    assert_eq!(
        header_val(&headers, "x-residency").as_deref(),
        Some("GLOBAL")
    );

    // traceparent must exist and match the canonical format.
    assert_eq!(
        header_val(&headers, "traceparent").as_deref(),
        Some("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01")
    );
}
