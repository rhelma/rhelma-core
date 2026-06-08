//! kafka_source.rs — Kafka-based ai.command.execute source for Observability-Agent
//!
//! Contract change: NO wildcard. Use explicit topic allow-list.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use rhelma_event::contracts::ai::AiCommandExecute;
use rhelma_event::{EventBus, EventEnvelope};
use rhelma_event_kafka_agent::{EventHandler, KafkaConfig, KafkaSubscriber};

use crate::commands::CommandExecutor;
use crate::error::AgentError;
use crate::io::internal_metrics;
use crate::runtime::ExternalCommandSource;

fn topic_has_forbidden_pattern(topic: &str) -> bool {
    // Contract: wildcard/regex topic subscription is forbidden.
    // We allow common Kafka topic characters (letters, digits, '.', '-', '_'),
    // but reject meta chars often used in regex/glob patterns.
    const BAD: &[char] = &['*', '?', '[', ']', '(', ')', '{', '}', '|', '^', '$', '\\'];
    topic.chars().any(|c| BAD.contains(&c))
}

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

/// Handler: converts EventEnvelope → AiCommandExecute → CommandExecutor
struct CommandEventHandler<B: EventBus + Send + Sync + 'static> {
    executor: Arc<CommandExecutor<B>>,
}

#[async_trait]
impl<B> EventHandler for CommandEventHandler<B>
where
    B: EventBus + Send + Sync + 'static,
{
    async fn handle(&self, env: EventEnvelope) {
        match serde_json::from_value::<AiCommandExecute>(env.payload) {
            Ok(cmd) => {
                if let Err(e) = self.executor.execute(cmd).await {
                    error!("[agent] command execution failed: {e}");
                }
            }
            Err(e) => error!("[agent] decode AiCommandExecute failed: {e}"),
        }
    }
}

/// Kafka command source — NOTE: no `<B>` needed
pub struct KafkaCommandSource {
    cfg: KafkaConfig,
    topics: Vec<String>,
}

impl KafkaCommandSource {
    /// Backward compatible single-topic constructor
    pub fn new(brokers: &str, group: &str, topic: &str) -> Result<Self, AgentError> {
        Self::new_many(brokers, group, vec![topic.to_string()])
    }

    /// New: allow-list topics
    pub fn new_many(brokers: &str, group: &str, topics: Vec<String>) -> Result<Self, AgentError> {
        let topics = normalize_topics(topics, "KafkaCommandSource")?;

        let mut cfg = KafkaConfig {
            brokers: brokers.into(),
            group_id: group.into(),
            ..Default::default()
        };

        // Optional shared topic prefix (must match AI orchestrator)
        if let Ok(prefix) = std::env::var("OBS_AGENT_TOPIC_PREFIX") {
            cfg.topic_prefix = prefix;
        }

        Ok(Self { cfg, topics })
    }
}

impl<B> ExternalCommandSource<B> for KafkaCommandSource
where
    B: EventBus + Send + Sync + 'static,
{
    fn start(
        self: Arc<Self>,
        executor: Arc<CommandExecutor<B>>,
        shutdown: tokio_util::sync::CancellationToken,
    ) -> JoinHandle<()> {
        let cfg = self.cfg.clone();
        let topics = self.topics.clone();

        tokio::spawn(async move {
            let handler = Arc::new(CommandEventHandler { executor });
            let mut backoff_ms: u64 = 500;

            async fn sleep_or_cancel(ms: u64, shutdown: &tokio_util::sync::CancellationToken) {
                tokio::select! {
                    _ = tokio::time::sleep(std::time::Duration::from_millis(ms)) => {}
                    _ = shutdown.cancelled() => {}
                }
            }

            loop {
                if shutdown.is_cancelled() {
                    info!("[agent] KafkaCommandSource shutdown requested; stopping loop");
                    break;
                }
                // ✅ must be mutable because run()/subscribe_* typically take &mut self
                let mut subscriber = match KafkaSubscriber::new(cfg.clone(), handler.clone()) {
                    Ok(s) => s,
                    Err(e) => {
                        error!("[agent] KafkaCommandSource init failed: {e}");
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
                        "[agent] KafkaCommandSource subscribe_many failed topics={:?}: {e}",
                        topics
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
                    "[agent] KafkaCommandSource listening on topics={}",
                    topics.join(",")
                );
                backoff_ms = 500;

                let run_res = tokio::select! {
                    _ = shutdown.cancelled() => {
                        warn!("[agent] KafkaCommandSource shutdown requested; stopping subscriber");
                        return;
                    }
                    r = subscriber.run() => r,
                };

                match run_res {
                    Ok(()) => {
                        warn!("[agent] KafkaCommandSource ended unexpectedly; retrying…");
                        internal_metrics::kafka_retry();
                    }
                    Err(e) => {
                        error!("[agent] Kafka command loop error: {e}");
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
