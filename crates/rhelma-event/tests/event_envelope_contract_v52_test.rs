#![forbid(unsafe_code)]

use base64::Engine;
use chrono::Utc;

use rhelma_event::{
    purpose, EventEnvelope, EventRequestContext, EventSource, EventTraceContext, PolicyMeta,
    Residency,
};
use serde_json::json;
use uuid::Uuid;

fn base_envelope(topic: &str) -> EventEnvelope {
    let source = EventSource::new("test-service", "1.0.0", "eu-central-1");

    let request = EventRequestContext {
        request_id: Some(Uuid::now_v7().to_string()),
        correlation_id: Some(Uuid::now_v7().to_string()),
        tenant_id: Some("t_demo".to_string()),
        user_id: Some("u_demo".to_string()),
        ..Default::default()
    };

    EventEnvelope {
        event_id: Uuid::now_v7().to_string(),
        event_version: 1,
        topic: topic.to_string(),
        key: None,
        timestamp: Utc::now(),
        published_at: Utc::now(),

        source,
        request,
        trace: EventTraceContext::default(),

        payload: json!({"ok": true}),
        payload_type: "application/json".to_string(),
        schema_ref: format!("{topic}@v1"),

        policy: PolicyMeta::public(purpose::TESTS),
        residency: Residency::Global,
        encryption: None,
        signature: None,
        hash: None,
    }
}

#[test]
fn context_roundtrip_traceparent() {
    let env = base_envelope("obs.heartbeat")
        .finalize_strict()
        .expect("finalize");

    // trace ids should be generated
    assert!(env.trace.trace_id.as_deref().unwrap().len() == 32);
    assert!(env.trace.span_id.as_deref().unwrap().len() == 16);
}

#[test]
fn audit_requires_signature_and_hash() {
    let mut env = base_envelope("ops.audit.user_action");
    // missing signature should fail
    assert!(env.clone().finalize_strict().is_err());

    // provide a dummy (format-valid) signature; finalize will compute hash
    // ed25519:BASE64(64 bytes) where bytes are all 1
    let sig = vec![1u8; 64];
    let b64 = base64::engine::general_purpose::STANDARD.encode(sig);
    env.signature = Some(format!("ed25519:{b64}"));
    let out = env.finalize_strict().expect("finalize ok");
    assert!(out.hash.is_some());
}
