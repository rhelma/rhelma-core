#![forbid(unsafe_code)]

use base64::Engine;

use chrono::Utc;
use rhelma_event::{
    purpose, verify_audit_envelope, AuditKeyRing, AuditSigner, EventEnvelope, EventRequestContext,
    EventSource, EventTraceContext, PolicyMeta, Residency,
};
use uuid::Uuid;

mod common;

#[test]
fn audit_sign_and_verify_roundtrip() {
    // Env is process-global; isolate to prevent flakes when tests run in parallel.
    common::with_isolated_prefix_env("RHELMA_AUDIT", || {
        // Deterministic seed (32 bytes) for test only.
        let seed = [7u8; 32];
        std::env::set_var(
            "RHELMA_AUDIT_SIGNING_KEY",
            base64::engine::general_purpose::STANDARD.encode(seed),
        );
        std::env::set_var("RHELMA_AUDIT_KEY_ID", "k1");

        // Build keyring from the corresponding public key.
        let signing = ed25519_dalek::SigningKey::from_bytes(&seed);
        let pubkey = signing.verifying_key().to_bytes();
        std::env::set_var(
            "RHELMA_AUDIT_PUBKEYS",
            format!(
                "k1:{}",
                base64::engine::general_purpose::STANDARD.encode(pubkey)
            ),
        );
        std::env::set_var("RHELMA_AUDIT_DEFAULT_KEY_ID", "k1");

        let signer = AuditSigner::from_env().expect("signer");
        let ring = AuditKeyRing::from_env().expect("keyring");

        let mut env = EventEnvelope {
            event_id: Uuid::now_v7().to_string(),
            event_version: 1,
            topic: "ops.audit.user_action".to_string(),
            key: None,
            timestamp: Utc::now(),
            published_at: Utc::now(),

            source: EventSource::new("svc", "1.0.0", "eu"),
            request: EventRequestContext {
                request_id: Some(Uuid::now_v7().to_string()),
                correlation_id: Some(Uuid::now_v7().to_string()),
                ..Default::default()
            },
            trace: EventTraceContext::default(),
            payload: serde_json::json!({"b":2,"a":1}),
            payload_type: "application/json".to_string(),
            schema_ref: "ops.audit.user_action@v1".to_string(),
            policy: PolicyMeta::public(purpose::TESTS),
            residency: Residency::Global,
            encryption: None,

            signature: None,
            hash: None,
        };

        // finalize computes hash requirement + generates trace ids
        env = env.finalize_strict().expect("finalize");

        // Sign digest and attach signature.
        let digest = rhelma_event::audit_crypto::audit_payload_digest(&env.payload);
        env.signature = Some(signer.sign_digest(&digest));

        // finalize again to enforce signature format + hash match
        env = env.finalize_strict().expect("finalize with signature");

        verify_audit_envelope(&env, &ring).expect("verify ok");
    })
}
