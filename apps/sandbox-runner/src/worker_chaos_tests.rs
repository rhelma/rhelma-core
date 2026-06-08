#![forbid(unsafe_code)]

use std::sync::Arc;

use chrono::Utc;
use rhelma_ai_attestation::sha256_hex;
use rhelma_ai_contracts::improvements::{
    AiImproveEvaluationV1, AiImproveProposalV1, ImprovementRiskLevel, SCHEMA_IMPROVE_EVALUATION_V1,
    SCHEMA_IMPROVE_PROPOSAL_V1, TOPIC_IMPROVE_EVALUATION, TOPIC_IMPROVE_PROPOSAL,
};
use rhelma_event::{
    generate_event_id, purpose, EventBus, EventBusError, EventEnvelope, EventRequestContext,
    EventRequestFlags, EventSource, EventTraceContext, PolicyMeta, Residency,
};
use rhelma_event_kafka::FallibleEventHandler;
use serde_json::Value;
use tokio::sync::Mutex;

use rhelma_sandbox_runner::{config::SandboxRunnerConfig, runner::SandboxRunner};

use super::{EventSink, ProposalHandler};

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

fn make_valid_proposal(patch: String) -> AiImproveProposalV1 {
    AiImproveProposalV1 {
        proposal_id: generate_event_id(),
        title: "chaos".to_string(),
        target: "apps/sandbox-runner".to_string(),
        patch,
        test_plan: vec!["cargo test".to_string()],
        risk_level: ImprovementRiskLevel::Low,
        actor: "system".to_string(),
        created_at: Utc::now(),
    }
}

#[tokio::test]
async fn sandbox_runner_evaluation_failure_is_observable_and_replayable() {
    let bus = CapturingBus::default();
    let bus_arc: Arc<dyn EventBus> = Arc::new(bus.clone());

    let events = EventSink::new(
        "sandbox-runner".to_string(),
        "test".to_string(),
        "local".to_string(),
        bus_arc,
    );

    // Force a deterministic failure before any filesystem work.
    let cfg = SandboxRunnerConfig {
        max_patch_bytes: 1,
        ..Default::default()
    };
    let runner = SandboxRunner::new(cfg);
    let handler = ProposalHandler::new(runner, events);

    let patch = "diff --git a/apps/sandbox-runner/src/main.rs b/apps/sandbox-runner/src/main.rs\n+// chaos\n".to_string();
    let proposal = make_valid_proposal(patch.clone());

    let parent1 = make_parent_envelope(serde_json::to_value(&proposal).unwrap());
    handler.handle(parent1.clone()).await.unwrap();

    // Second delivery (retry) with a new inbound envelope context.
    let parent2 = make_parent_envelope(serde_json::to_value(&proposal).unwrap());
    handler.handle(parent2.clone()).await.unwrap();

    let published = bus.published.lock().await;
    assert_eq!(published.len(), 2);

    let expected_patch_sha = sha256_hex(patch.as_bytes());
    let expected_plan_sha = sha256_hex("cargo test".as_bytes());
    let expected_results_sha = sha256_hex(b"[]");

    for (i, (parent, out)) in [(parent1, &published[0]), (parent2, &published[1])]
        .into_iter()
        .enumerate()
    {
        assert_eq!(out.topic, TOPIC_IMPROVE_EVALUATION);
        assert_eq!(out.schema_ref, SCHEMA_IMPROVE_EVALUATION_V1);

        // Invariants: request/correlation, trace, and residency must be inherited.
        assert_eq!(
            out.request.request_id, parent.request.request_id,
            "case {i}"
        );
        assert_eq!(
            out.request.correlation_id, parent.request.correlation_id,
            "case {i}"
        );
        assert_eq!(out.trace.trace_id, parent.trace.trace_id, "case {i}");
        assert_eq!(out.trace.parent_span_id, parent.trace.span_id, "case {i}");
        assert_eq!(out.residency, parent.residency, "case {i}");

        let eval: AiImproveEvaluationV1 = serde_json::from_value(out.payload.clone()).unwrap();
        assert!(!eval.ok);
        assert!(
            eval.summary.contains("patch too large"),
            "summary was: {}",
            eval.summary
        );
        assert_eq!(eval.patch_sha256_hex, expected_patch_sha);
        assert_eq!(eval.test_plan_sha256_hex, expected_plan_sha);
        assert_eq!(eval.results_sha256_hex, expected_results_sha);

        // Also enforce that the embedded attested payload is consistent.
        assert_eq!(eval.attested_payload.patch_sha256_hex, expected_patch_sha);
        assert_eq!(
            eval.attested_payload.test_plan_sha256_hex,
            expected_plan_sha
        );
        assert_eq!(
            eval.attested_payload.results_sha256_hex,
            expected_results_sha
        );
        assert!(!eval.attested_payload.ok);
    }
}
