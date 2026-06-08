#![forbid(unsafe_code)]

use chrono::{TimeZone, Utc};
use rhelma_event::envelope_v52::{
    EventEncryptionV52, EventEnvelopeV52, EventHashV52, EventMetaV52, EventRequestV52,
    EventSignatureV52, EventSourceV52, EventTraceV52,
};
use rhelma_event::{purpose, PolicyMeta};
use serde_json::json;

#[test]
fn event_v52_json_snapshot() {
    let env = EventEnvelopeV52 {
        meta: EventMetaV52 {
            event_id: "018d3ca0-6b3e-7cdd-9c3c-2f4d2c4f9c22".to_string(),
            topic: "obs.heartbeat".to_string(),
            schema_ref: "obs.heartbeat@v2".to_string(),
            payload_type: "rhelma.obs.HeartbeatV2".to_string(),
            published_at: Utc.with_ymd_and_hms(2025, 12, 17, 12, 0, 0).unwrap(),
            source: EventSourceV52 {
                service: "api-gateway".to_string(),
                version: "5.2.0".to_string(),
                region: "eu-central-1".to_string(),
            },
            request: EventRequestV52 {
                request_id: "018d3c9f-2f4a-7d26-9d6f-5e6f8f4e1d10".to_string(),
                correlation_id: "018d3c9f-2f4a-7d26-9d6f-5e6f8f4e1d10".to_string(),
                tenant_id: Some("tenant_123".to_string()),
                user_id: Some("user_456".to_string()),
                residency: "GLOBAL".to_string(),
                traceparent: Some(
                    "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01".to_string(),
                ),
            },
            policy: PolicyMeta::public(purpose::TESTS),
            trace: EventTraceV52 {
                trace_id: Some("4bf92f3577b34da6a3ce929d0e0e4736".to_string()),
                span_id: Some("00f067aa0ba902b7".to_string()),
            },
            hash: Some(EventHashV52 {
                alg: "sha256".to_string(),
                value: "0f4d9e19d9f1b6e4d399dbdf8a4d4c1e3c8e9f0f0d4c2e1a9b8c7d6e5f4a3b2c"
                    .to_string(),
            }),
            signature: Some(EventSignatureV52 {
                alg: "ed25519".to_string(),
                key_id: Some("k1".to_string()),
                value: "BASE64_SIGNATURE==".to_string(),
            }),
            encryption: Some(EventEncryptionV52 {
                alg: "age".to_string(),
                key_id: Some("age1example".to_string()),
            }),
        },
        payload: json!({
            "service": "api-gateway",
            "ok": true,
            "uptime_seconds": 1234
        }),
    };

    insta::assert_json_snapshot!(env);
}
