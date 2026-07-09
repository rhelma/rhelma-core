#![forbid(unsafe_code)]

//! Regression coverage for the observability-agent -> ai-orchestrator bridge.
//!
//! The agent publishes `ai.incident.proposed` with a `message` field (and a
//! superset of fields), while the orchestrator consumes the canonical
//! `IncidentProposedEvent`, which names that field `description`. The
//! `#[serde(alias = "message")]` on `description` is what keeps these in sync;
//! this test fails loudly if that alias is ever dropped.

use rhelma_event::contracts::obs::IncidentProposedEvent;
use serde_json::json;

#[test]
fn deserializes_agent_payload_with_message_alias() {
    // Mirrors the shape published by `AiIncidentProposed` in the
    // observability-agent (message + agent-only extra fields).
    let agent_payload = json!({
        "incident_id": "inc-123",
        "service": "api-gateway",
        "service_version": "5.2.0",
        "environment": "prod",
        "region": "eu-central-1",
        "detected_at": "2026-06-16T10:00:00Z",
        "kind": "latency_spike",
        "severity": "critical",
        "message": "p99 latency exceeded SLO for 5m",
        "metrics": {"p99_ms": 1800},
        "category": "performance",
        "tags": ["slo", "latency"],
        "confidence": 0.92,
    });

    let event: IncidentProposedEvent =
        serde_json::from_value(agent_payload).expect("agent payload should deserialize");

    assert_eq!(event.incident_id, "inc-123");
    assert_eq!(event.service, "api-gateway");
    assert_eq!(event.region, "eu-central-1");
    assert_eq!(event.severity, "critical");
    // `message` is mapped onto `description` via the serde alias.
    assert_eq!(event.description, "p99 latency exceeded SLO for 5m");
}

#[test]
fn deserializes_canonical_payload_with_description() {
    // The canonical field name must keep working alongside the alias.
    let canonical = json!({
        "incident_id": "inc-456",
        "service": "billing",
        "region": "us-east-1",
        "severity": "moderate",
        "description": "checkout error rate elevated",
        "detected_at": "2026-06-16T10:00:00Z",
    });

    let event: IncidentProposedEvent =
        serde_json::from_value(canonical).expect("canonical payload should deserialize");

    assert_eq!(event.description, "checkout error rate elevated");
}
