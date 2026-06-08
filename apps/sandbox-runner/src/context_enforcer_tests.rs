#![forbid(unsafe_code)]

use std::sync::Arc;

use chrono::Utc;
use rhelma_event::{
    generate_event_id, purpose, EventBus, EventBusError, EventEnvelope, EventRequestContext,
    EventRequestFlags, EventSource, EventTraceContext, PolicyMeta, Residency,
};
use serde_json::Value;
use tokio::sync::Mutex;

use rhelma_ai_contracts::improvements::{
    SCHEMA_IMPROVE_EVALUATION_V1, SCHEMA_IMPROVE_PROPOSAL_V1, TOPIC_IMPROVE_EVALUATION,
    TOPIC_IMPROVE_PROPOSAL,
};

use super::EventSink;

#[derive(Clone, Default)]
struct CapturingBus {
    published: Arc<Mutex<Vec<EventEnvelope>>>,
}

#[async_trait::async_trait]
impl EventBus for CapturingBus {
    async fn publish(&self, event: EventEnvelope) -> Result<(), EventBusError> {
        self.published.lock().await.push(event);
        Ok(())
    }
}

fn make_parent_envelope(payload: Value) -> EventEnvelope {
    let now = Utc::now();
    EventEnvelope {
        event_id: generate_event_id(),
        event_version: 52,
        topic: TOPIC_IMPROVE_PROPOSAL.to_string(),
        key: Some("p".to_string()),
        timestamp: now,
        published_at: now,
        source: EventSource::new(
            "ai-orchestrator".to_string(),
            "test".to_string(),
            "local".to_string(),
        ),
        request: EventRequestContext {
            request_id: Some(generate_event_id()),
            correlation_id: Some(generate_event_id()),
            tenant_id: None,
            user_id: None,
            flags: EventRequestFlags {
                system: true,
                ai_safe: true,
                read_only: false,
            },
        },
        trace: EventTraceContext::generate(),
        payload,
        payload_type: "application/json".to_string(),
        schema_ref: SCHEMA_IMPROVE_PROPOSAL_V1.to_string(),
        policy: PolicyMeta::public(purpose::SANDBOX_RUNNER),
        residency: Residency::Global,
        encryption: None,
        signature: None,
        hash: None,
    }
}

#[tokio::test]
async fn sandbox_runner_evaluation_envelope_inherits_request_and_trace_context() {
    let bus = CapturingBus::default();
    let bus_arc: Arc<dyn EventBus> = Arc::new(bus);

    let events = EventSink::new(
        "sandbox-runner".to_string(),
        "test".to_string(),
        "local".to_string(),
        bus_arc,
    );

    let parent = make_parent_envelope(serde_json::json!({"proposal_id": "p"}));

    // We call the internal helper directly (no need to run a real sandbox evaluation here).
    let out = events.envelope_inherited(
        &parent,
        TOPIC_IMPROVE_EVALUATION,
        Some("p".to_string()),
        SCHEMA_IMPROVE_EVALUATION_V1,
        serde_json::json!({"ok": true}),
    );

    // Enforced: request/correlation are inherited from inbound envelope.
    assert_eq!(out.request.request_id, parent.request.request_id);
    assert_eq!(out.request.correlation_id, parent.request.correlation_id);

    // Enforced: trace_id is preserved, span_id is new, and parent_span_id points at inbound span.
    assert_eq!(out.trace.trace_id, parent.trace.trace_id);
    assert_ne!(out.trace.span_id, parent.trace.span_id);
    assert_eq!(out.trace.parent_span_id, parent.trace.span_id);

    // Enforced: residency is inherited.
    assert_eq!(out.residency, parent.residency);
}
