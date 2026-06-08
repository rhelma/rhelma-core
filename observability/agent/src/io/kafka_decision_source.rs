//! kafka_decision_source.rs — v5.2 Kafka Decision Source
//!
//! Contract change: NO wildcard. Use explicit topic allow-list.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use rhelma_event::{EventBus, EventEnvelope};
use rhelma_event_kafka_agent::{EventHandler, KafkaConfig, KafkaSubscriber};

use crate::agent::ObservabilityAgent;
use crate::ai::{apply_ai_decision, AiIncidentDecision};
use crate::error::AgentError;
use crate::io::internal_metrics;
use crate::runtime::DecisionSource;

fn topic_has_forbidden_pattern(topic: &str) -> bool {
    // Contract: wildcard/regex topic subscription is forbidden.
    const BAD: &[char] = &['*', '?', '[', ']', '(', ')', '{', '}', '|', '^', '$', '\\'];
    topic.chars().any(|c| BAD.contains(&c))
}

/// Normalizes and validates Kafka topics
///
/// # Arguments
/// * `topics` - List of topics to normalize
/// * `ctx` - Context for error messages
///
/// # Returns
/// `Result<Vec<String>, AgentError>` - Normalized topics or error
fn normalize_topics(mut topics: Vec<String>, ctx: &str) -> Result<Vec<String>, AgentError> {
    topics = topics
        .into_iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();

    if topics.is_empty() {
        return Err(AgentError::invalid(format!("{ctx}: topics list is empty")));
    }

    for t in &topics {
        if topic_has_forbidden_pattern(t) {
            return Err(AgentError::invalid(format!(
                "{ctx}: wildcard/regex patterns are not allowed (topic='{t}')"
            )));
        }
    }

    Ok(topics)
}

/// Event handler for AI incident decisions
struct DecisionEventHandler<B: EventBus + Send + Sync + 'static> {
    /// Observability agent instance
    agent: Arc<ObservabilityAgent<B>>,
}

#[async_trait]
impl<B> EventHandler for DecisionEventHandler<B>
where
    B: EventBus + Send + Sync + 'static,
{
    /// Handles incoming decision events
    ///
    /// # Arguments
    /// * `env` - Event envelope containing AI decision
    async fn handle(&self, env: EventEnvelope) {
        match serde_json::from_value::<AiIncidentDecision>(env.payload) {
            Ok(decision) => {
                if let Err(e) = apply_ai_decision(&self.agent, decision).await {
                    error!("[agent] apply_ai_decision failed: {e}");
                }
            }
            Err(e) => error!("[agent] decode AiIncidentDecision failed: {e}"),
        }
    }
}

/// Kafka decision source for AI incident decisions
pub struct KafkaDecisionSource {
    /// Kafka configuration
    cfg: KafkaConfig,
    /// List of topics to subscribe to
    topics: Vec<String>,
}

impl KafkaDecisionSource {
    /// Backward compatible single-topic constructor
    ///
    /// # Arguments
    /// * `brokers` - Kafka brokers
    /// * `group` - Consumer group ID
    /// * `topic` - Topic to subscribe to
    ///
    /// # Returns
    /// `Result<Self, AgentError>` - Decision source or error
    pub fn new(brokers: &str, group: &str, topic: &str) -> Result<Self, AgentError> {
        Self::new_many(brokers, group, vec![topic.to_string()])
    }

    /// New: allow-list topics constructor
    ///
    /// # Arguments
    /// * `brokers` - Kafka brokers
    /// * `group` - Consumer group ID
    /// * `topics` - List of topics to subscribe to
    ///
    /// # Returns
    /// `Result<Self, AgentError>` - Decision source or error
    pub fn new_many(brokers: &str, group: &str, topics: Vec<String>) -> Result<Self, AgentError> {
        let topics = normalize_topics(topics, "KafkaDecisionSource")?;

        let mut cfg = KafkaConfig {
            brokers: brokers.into(),
            group_id: group.into(),
            ..Default::default()
        };

        if let Ok(prefix) = std::env::var("OBS_AGENT_TOPIC_PREFIX") {
            cfg.topic_prefix = prefix;
        }

        Ok(Self { cfg, topics })
    }
}

impl<B> DecisionSource<B> for KafkaDecisionSource
where
    B: EventBus + Send + Sync + 'static,
{
    /// Starts the decision source
    ///
    /// # Arguments
    /// * `agent` - Observability agent instance
    ///
    /// # Returns
    /// Join handle to the spawned task
    fn start(
        self: Arc<Self>,
        agent: Arc<ObservabilityAgent<B>>,
        shutdown: tokio_util::sync::CancellationToken,
    ) -> JoinHandle<()> {
        let cfg = self.cfg.clone();
        let topics = self.topics.clone();

        tokio::spawn(async move {
            let handler = Arc::new(DecisionEventHandler { agent });
            let mut backoff_ms: u64 = 500;

            async fn sleep_or_cancel(ms: u64, shutdown: &tokio_util::sync::CancellationToken) {
                tokio::select! {
                    _ = tokio::time::sleep(std::time::Duration::from_millis(ms)) => {}
                    _ = shutdown.cancelled() => {}
                }
            }

            loop {
                if shutdown.is_cancelled() {
                    info!("[agent] KafkaDecisionSource shutdown requested; stopping loop");
                    break;
                }
                // ✅ must be mutable because run()/subscribe_* typically take &mut self
                let mut subscriber = match KafkaSubscriber::new(cfg.clone(), handler.clone()) {
                    Ok(s) => s,
                    Err(e) => {
                        error!("[agent] KafkaDecisionSource init failed: {e}");
                        internal_metrics::kafka_retry();
                        sleep_or_cancel(backoff_ms.min(30_000), &shutdown).await;
                        if shutdown.is_cancelled() {
                            break;
                        }
                        backoff_ms = (backoff_ms.saturating_mul(2)).min(30_000);
                        continue;
                    }
                };

                if let Err(e) = subscriber
                    .subscribe_many(topics.iter().map(String::as_str))
                    .await
                {
                    error!(
                        "[agent] KafkaDecisionSource subscribe_many failed topics={topics:?}: {e}"
                    );
                    internal_metrics::kafka_retry();
                    sleep_or_cancel(backoff_ms.min(30_000), &shutdown).await;
                    if shutdown.is_cancelled() {
                        break;
                    }
                    backoff_ms = (backoff_ms.saturating_mul(2)).min(30_000);
                    continue;
                }

                info!(
                    "[agent] KafkaDecisionSource listening on topics={}",
                    topics.join(",")
                );
                backoff_ms = 500;

                let run_res = tokio::select! {
                    _ = shutdown.cancelled() => {
                        warn!("[agent] KafkaDecisionSource shutdown requested; stopping subscriber");
                        return;
                    }
                    r = subscriber.run() => r,
                };

                match run_res {
                    Ok(()) => {
                        warn!("[agent] KafkaDecisionSource ended unexpectedly; retrying…");
                        internal_metrics::kafka_retry();
                    }
                    Err(e) => {
                        error!("[agent] decision loop error: {e}");
                        internal_metrics::kafka_retry();
                    }
                }

                sleep_or_cancel(backoff_ms.min(30_000), &shutdown).await;
                if shutdown.is_cancelled() {
                    break;
                }
                backoff_ms = (backoff_ms.saturating_mul(2)).min(30_000);
            }
        })
    }
}
