//! sandbox-runner — Rhelma Self-Improvement Sandbox Worker (Contract v6.0)
//!
//! This service consumes `ai.improve.proposal` events, evaluates them in an isolated
//! workspace (local or Docker), and publishes `ai.improve.evaluation` results.

#![forbid(unsafe_code)]

use std::env;
use std::sync::Arc;

use anyhow::Context;
use chrono::Utc;
use rhelma_config::central_env::CentralEnv;
use rhelma_event::{
    generate_event_id, publish_with_observability, purpose, EventBus, EventEnvelope,
    EventRequestContext, EventSource, EventTraceContext, PolicyMeta,
};
use rhelma_event_kafka::{
    FallibleEventHandler, KafkaConfig, KafkaEventBus, KafkaProducerWrapper, KafkaSubscriber,
};
use serde_json::Value;
use tracing::{info, warn};

use rhelma_ai_attestation::sha256_hex;
use rhelma_ai_contracts::improvements::{
    AiImproveEvaluationV1, AiImproveProposalV1, EvaluationAttestedPayloadV1,
    SCHEMA_IMPROVE_EVALUATION_V1, TOPIC_IMPROVE_EVALUATION, TOPIC_IMPROVE_PROPOSAL,
};
use rhelma_sandbox_runner::{config::SandboxRunnerConfig, runner::SandboxRunner};

const EVENT_VERSION: i32 = 52;
const PAYLOAD_TYPE_JSON: &str = "application/json";

#[cfg(test)]
mod context_enforcer_tests;

#[cfg(test)]
mod worker_chaos_tests;

#[derive(Clone)]
struct EventSink {
    source: EventSource,
    bus: Arc<dyn EventBus>,
}

impl EventSink {
    fn new(service: String, version: String, region: String, bus: Arc<dyn EventBus>) -> Self {
        Self {
            source: EventSource::new(service, version, region),
            bus,
        }
    }

    fn envelope_inherited(
        &self,
        parent: &EventEnvelope,
        topic: &str,
        key: Option<String>,
        schema_ref: &str,
        payload: Value,
    ) -> EventEnvelope {
        let now = Utc::now();
        EventEnvelope {
            event_id: generate_event_id(),
            event_version: EVENT_VERSION,
            topic: topic.to_string(),
            key,
            timestamp: now,
            published_at: now,
            source: self.source.clone(),
            request: EventRequestContext::inherit_or_generate(&parent.request),
            trace: EventTraceContext::child_of(&parent.trace),
            payload,
            payload_type: PAYLOAD_TYPE_JSON.to_string(),
            schema_ref: schema_ref.to_string(),
            policy: PolicyMeta::derived_from(&parent.policy, purpose::SANDBOX_RUNNER),
            residency: parent.residency,
            encryption: None,
            signature: None,
            hash: None,
        }
    }

    async fn send_inherited(
        &self,
        parent: &EventEnvelope,
        topic: &str,
        key: Option<String>,
        schema_ref: &str,
        payload: Value,
    ) -> Result<(), rhelma_event::EventBusError> {
        let env = self.envelope_inherited(parent, topic, key, schema_ref, payload);
        publish_with_observability(self.bus.as_ref(), env).await
    }
}

struct ProposalHandler {
    runner: SandboxRunner,
    events: EventSink,
}

impl ProposalHandler {
    fn new(runner: SandboxRunner, events: EventSink) -> Self {
        Self { runner, events }
    }

    fn de_value<T>(payload: Value) -> Result<T, rhelma_event::EventBusError>
    where
        T: serde::de::DeserializeOwned,
    {
        serde_json::from_value(payload)
            .map_err(|e| rhelma_event::EventBusError::Serialization(e.to_string()))
    }
}

#[async_trait::async_trait]
impl FallibleEventHandler for ProposalHandler {
    async fn handle(&self, env: EventEnvelope) -> Result<(), rhelma_event::EventBusError> {
        if env.topic.as_str() != TOPIC_IMPROVE_PROPOSAL {
            return Ok(());
        }

        // `serde_json::from_value` consumes the `Value`, but we still need the full envelope
        // for send_inherited (trace/request/policy propagation).
        let proposal: AiImproveProposalV1 = Self::de_value(env.payload.clone())?;
        let eval: AiImproveEvaluationV1 = match self.runner.evaluate_proposal(&proposal).await {
            Ok(v) => v,
            Err(e) => {
                let patch_sha = sha256_hex(proposal.patch.as_bytes());
                let plan_joined = proposal.test_plan.join("\n");
                let plan_sha = sha256_hex(plan_joined.as_bytes());
                let empty_results: Vec<rhelma_ai_contracts::improvements::SandboxCommandResultV1> =
                    Vec::new();
                let results_sha = sha256_hex(
                    serde_json::to_vec(&empty_results)
                        .unwrap_or_else(|_| b"[]".to_vec())
                        .as_slice(),
                );
                let evaluated_at = Utc::now();
                let mode = if self.runner_mode_is_docker() {
                    "docker".to_string()
                } else {
                    "local".to_string()
                };

                let attested_payload = EvaluationAttestedPayloadV1 {
                    proposal_id: proposal.proposal_id.clone(),
                    patch_sha256_hex: patch_sha.clone(),
                    test_plan_sha256_hex: plan_sha.clone(),
                    results_sha256_hex: results_sha.clone(),
                    ok: false,
                    mode: mode.clone(),
                    evaluated_at,
                };

                AiImproveEvaluationV1 {
                    proposal_id: proposal.proposal_id.clone(),
                    ok: false,
                    patch_sha256_hex: patch_sha,
                    test_plan_sha256_hex: plan_sha,
                    results_sha256_hex: results_sha,
                    mode,
                    results: empty_results,
                    summary: format!("evaluation error: {e}"),
                    attested_payload,
                    attestation: None,
                    evaluated_at,
                }
            }
        };

        let payload = serde_json::to_value(&eval).unwrap_or_else(|_| serde_json::json!({}));
        self.events
            .send_inherited(
                &env,
                TOPIC_IMPROVE_EVALUATION,
                Some(eval.proposal_id.clone()),
                SCHEMA_IMPROVE_EVALUATION_V1,
                payload,
            )
            .await?;

        Ok(())
    }
}

impl ProposalHandler {
    fn runner_mode_is_docker(&self) -> bool {
        // This is informational only.
        env::var("RHELMA_SANDBOX_RUNNER__DOCKER_ENABLED")
            .ok()
            .map(|v| {
                matches!(
                    v.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "y" | "on"
                )
            })
            .unwrap_or(false)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let service = env::var("RHELMA_SERVICE_NAME").unwrap_or_else(|_| "sandbox-runner".to_string());
    let region = CentralEnv::from_env_with_defaults("eu-west", "development", "0.0.0-dev").region;

    let brokers = env::var("RHELMA_SANDBOX_RUNNER__KAFKA_BROKERS")
        .or_else(|_| env::var("RHELMA_KAFKA_BROKERS"))
        .unwrap_or_else(|_| "localhost:9092".to_string());

    let prefix = env::var("RHELMA_SANDBOX_RUNNER__KAFKA_TOPIC_PREFIX")
        .or_else(|_| env::var("RHELMA_KAFKA_TOPIC_PREFIX"))
        .unwrap_or_else(|_| "rhelma.".to_string());

    let kafka_cfg = KafkaConfig {
        brokers,
        group_id: format!("{}-consumer", service),
        topic_prefix: prefix,
        ..Default::default()
    };

    let producer = KafkaProducerWrapper::new(kafka_cfg.clone()).context("kafka producer")?;
    let bus = KafkaEventBus::new(Arc::new(producer));
    let events = EventSink::new(
        service.clone(),
        env!("CARGO_PKG_VERSION").to_string(),
        region,
        Arc::new(bus),
    );

    let runner_cfg = SandboxRunnerConfig::from_env();
    let runner = SandboxRunner::new(runner_cfg);

    let handler = Arc::new(ProposalHandler::new(runner, events));
    let mut subscriber =
        KafkaSubscriber::new_fallible(kafka_cfg, handler).context("kafka subscriber")?;
    subscriber.subscribe(TOPIC_IMPROVE_PROPOSAL).await?;

    info!(topic = %TOPIC_IMPROVE_PROPOSAL, "sandbox-runner subscriber started");

    tokio::select! {
        r = subscriber.run() => {
            if let Err(e) = r {
                warn!("subscriber exited: {e}");
            }
        }
        _ = shutdown_signal() => {
            warn!("shutdown signal");
        }
    }

    Ok(())
}

fn init_tracing() {
    // Minimal default; uses RUST_LOG if present.
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .init();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate()).expect("sigterm handler");
        sigterm.recv().await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
