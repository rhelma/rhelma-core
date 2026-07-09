#![forbid(unsafe_code)]

use chrono::Utc;
use rhelma_event::platform::{
    contains_obvious_secret_material, MemoryPlatformEventStore, PlatformEventEnvelope,
    PlatformEventStore,
};
use rhelma_event::EventSource;
use serde_json::json;

fn source() -> EventSource {
    EventSource::new("patch-applier", "test", "local")
}

fn envelope(payload: serde_json::Value) -> PlatformEventEnvelope {
    PlatformEventEnvelope::new(
        "platform.improvement.applied.v1",
        1,
        Utc::now(),
        source(),
        None,
        None,
        None,
        "corr-1",
        Some("proposal-1".to_string()),
        payload,
    )
    .expect("event")
}

#[test]
fn platform_event_envelope_serializes_and_deserializes() {
    let event = envelope(json!({"proposal_id": "proposal-1", "patch_sha": "a".repeat(64)}));
    let encoded = serde_json::to_string(&event).expect("serialize");
    let decoded: PlatformEventEnvelope = serde_json::from_str(&encoded).expect("deserialize");

    assert_eq!(decoded.event_id, event.event_id);
    assert_eq!(decoded.event_type, "platform.improvement.applied.v1");
    assert_eq!(decoded.payload_sha256, event.payload_sha256);
    decoded.validate().expect("valid envelope");
}

#[test]
fn platform_event_required_fields_validation_works() {
    let mut event = envelope(json!({"proposal_id": "proposal-1"}));
    event.correlation_id.clear();

    let err = event.validate().expect_err("missing correlation rejected");
    assert!(err.to_string().contains("correlation_id"));
}

#[test]
fn payload_and_event_hashes_are_stable() {
    let occurred_at = Utc::now();
    let payload = json!({"b": 2, "a": 1});
    let event = PlatformEventEnvelope::new(
        "platform.improvement.applied.v1",
        1,
        occurred_at,
        source(),
        None,
        None,
        None,
        "corr-1",
        None,
        payload,
    )
    .expect("event");
    let event_2 = PlatformEventEnvelope {
        event_hash: event.compute_event_hash(),
        ..event.clone()
    };

    assert_eq!(event.payload_sha256, event_2.payload_sha256);
    assert_eq!(event.event_hash, event_2.event_hash);
}

#[test]
fn event_hash_changes_when_payload_changes() {
    let event = envelope(json!({"proposal_id": "proposal-1", "status": "ok"}));
    let mut changed = event.clone();
    changed.payload = json!({"proposal_id": "proposal-1", "status": "changed"});
    changed.payload_sha256 =
        rhelma_event::canonicalization::canonical_payload_hash_hex(&changed.payload);
    changed.event_hash = changed.compute_event_hash();

    assert_ne!(event.payload_sha256, changed.payload_sha256);
    assert_ne!(event.event_hash, changed.event_hash);
}

#[tokio::test]
async fn memory_store_appends_with_hash_chain() {
    let store = MemoryPlatformEventStore::new();
    let first = store
        .append(envelope(json!({"proposal_id": "proposal-1"})))
        .await
        .expect("append first")
        .event;
    let second = store
        .append(envelope(json!({"proposal_id": "proposal-2"})))
        .await
        .expect("append second")
        .event;

    assert!(first.previous_event_hash.is_none());
    assert_eq!(
        second.previous_event_hash.as_deref(),
        Some(first.event_hash.as_str())
    );
    assert_eq!(store.events().expect("events").len(), 2);
}

#[test]
fn platform_event_payload_rejects_obvious_secret_paths_and_tokens() {
    assert!(contains_obvious_secret_material(&json!({"path": ".env"})));
    assert!(contains_obvious_secret_material(
        &json!({"api_token": "abc123"})
    ));
    assert!(!contains_obvious_secret_material(&json!({
        "proposal_id": "proposal-1",
        "patch_sha": "a".repeat(64)
    })));
}
