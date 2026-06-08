//! kafka_signal_source.rs — v5.2 Reflex Signal Ingress
//!
//! Contract change: NO wildcard. Use explicit topic allow-list.
//!
//! Reliability/operability improvements:
//! - Bounded in-memory queue for backpressure
//! - Single worker task to serialize detector state updates
//! - Kafka subscriber reconnect loop with exponential backoff
//! - Cooperative shutdown via CancellationToken

#![forbid(unsafe_code)]

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use rhelma_event::{EventBus, EventEnvelope};
use rhelma_event_kafka_agent::{EventHandler, KafkaConfig, KafkaSubscriber};

use crate::agent::ObservabilityAgent;
use crate::error::AgentError;
use crate::io::internal_metrics;
use crate::reflex::anomaly::NaiveAnomalyDetector;
use crate::reflex::signals::SignalPayload;

const DEFAULT_QUEUE_CAPACITY: usize = 1024;

fn topic_has_forbidden_pattern(topic: &str) -> bool {
    // Contract: wildcard/regex topic subscription is forbidden.
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

/// Kafka-based ingress for Reflex signals.
///
/// Signals are decoded into [`SignalPayload`] and then fed through a bounded queue into a
/// single worker task to keep detector state consistent (no interleaving).
pub struct KafkaSignalSource<B>
where
    B: EventBus + Send + Sync + 'static,
{
    cfg: KafkaConfig,
    topics: Vec<String>,
    agent: Arc<ObservabilityAgent<B>>,
    detector: Arc<Mutex<NaiveAnomalyDetector<B>>>,

    /// Shared cancellation token to stop subscriber + worker.
    shutdown: CancellationToken,

    /// Bounded queue capacity for backpressure.
    queue_capacity: usize,
}

impl<B> KafkaSignalSource<B>
where
    B: EventBus + Send + Sync + 'static,
{
    /// Backward-compatible single-topic constructor.
    pub fn new(
        cfg: KafkaConfig,
        topic: impl Into<String>,
        agent: Arc<ObservabilityAgent<B>>,
        detector: Arc<Mutex<NaiveAnomalyDetector<B>>>,
    ) -> Result<Self, AgentError> {
        Self::new_many(cfg, vec![topic.into()], agent, detector)
    }

    /// New: explicit allow-list topics (NO wildcard).
    pub fn new_many(
        cfg: KafkaConfig,
        topics: Vec<String>,
        agent: Arc<ObservabilityAgent<B>>,
        detector: Arc<Mutex<NaiveAnomalyDetector<B>>>,
    ) -> Result<Self, AgentError> {
        let topics = normalize_topics(topics, "KafkaSignalSource")?;

        Ok(Self {
            cfg,
            topics,
            agent,
            detector,
            shutdown: CancellationToken::new(),
            queue_capacity: DEFAULT_QUEUE_CAPACITY,
        })
    }

    /// Attach a shared shutdown token.
    pub fn with_shutdown(mut self, shutdown: CancellationToken) -> Self {
        self.shutdown = shutdown;
        self
    }

    /// Configure bounded queue capacity.
    ///
    /// This controls max in-memory buffering and provides backpressure via drop-on-full.
    pub fn with_queue_capacity(mut self, queue_capacity: usize) -> Self {
        self.queue_capacity = queue_capacity.max(1);
        self
    }

    /// Start Kafka consumer + signal worker.
    ///
    /// Returns the join handle for the Kafka subscriber loop. (The worker runs as an
    /// internal task and stops when shutdown triggers or channel closes.)
    pub fn start(self: Arc<Self>) -> JoinHandle<()> {
        let this = self.clone();
        let shutdown = this.shutdown.clone();
        let queue_capacity = this.queue_capacity;

        // Bounded queue: provides backpressure and bounds memory.
        let (tx, mut rx) = mpsc::channel::<SignalPayload>(queue_capacity);

        // Single worker task: ensures detector state isn't interleaved across signals.
        let worker_agent = this.agent.clone();
        let worker_detector = this.detector.clone();
        let worker_shutdown = shutdown.clone();
        tokio::spawn(async move {
            loop {
                let next = tokio::select! {
                    _ = worker_shutdown.cancelled() => None,
                    msg = rx.recv() => msg,
                };

                let Some(signal) = next else {
                    break;
                };

                // ---- sync phase (serialized) ----
                let action = {
                    let mut det = worker_detector.lock().await;
                    match det.process_signal_sync(&worker_agent, signal) {
                        Ok(a) => a,
                        Err(e) => {
                            error!("process_signal_sync failed: {e:?}");
                            continue;
                        }
                    }
                };

                // ---- async phase ----
                if let Err(e) = worker_agent.execute_detector_async(action).await {
                    error!("execute_detector_async error: {e:?}");
                }
            }

            info!("[agent] signal worker stopped");
        });

        tokio::spawn(async move {
            let topics = this.topics.clone();
            let handler = Arc::new(SignalEventHandler { tx });

            let mut backoff_ms: u64 = 500;

            loop {
                if shutdown.is_cancelled() {
                    internal_metrics::agent_shutdown();
                    info!("[agent] KafkaSignalSource stopping (shutdown)");
                    break;
                }

                // Create subscriber inside the loop (reconnect on failure).
                let mut subscriber = match KafkaSubscriber::new(this.cfg.clone(), handler.clone()) {
                    Ok(s) => s,
                    Err(e) => {
                        error!("[agent] KafkaSignalSource init failed: {e}");
                        internal_metrics::kafka_retry();
                        backoff_ms = backoff_sleep(backoff_ms, shutdown.clone()).await;
                        continue;
                    }
                };

                if let Err(e) = subscriber
                    .subscribe_many(topics.iter().map(|t| t.as_str()))
                    .await
                {
                    error!("[agent] KafkaSignalSource subscribe failed topics={topics:?}: {e}");
                    internal_metrics::kafka_retry();
                    backoff_ms = backoff_sleep(backoff_ms, shutdown.clone()).await;
                    continue;
                }

                info!(
                    "[agent] KafkaSignalSource listening on topics={}",
                    topics.join(",")
                );

                let run_res = tokio::select! {
                    _ = shutdown.cancelled() => Ok(()),
                    res = subscriber.run() => res,
                };

                match run_res {
                    Ok(()) => {
                        if shutdown.is_cancelled() {
                            internal_metrics::agent_shutdown();
                            info!("[agent] KafkaSignalSource stopped (shutdown)");
                            break;
                        }
                        // Normal stop (unexpected): retry.
                        warn!("[agent] KafkaSignalSource ended unexpectedly; retrying…");
                        internal_metrics::kafka_retry();
                        backoff_ms = backoff_sleep(backoff_ms, shutdown.clone()).await;
                    }
                    Err(e) => {
                        error!("[agent] KafkaSignalSource loop error: {e}");
                        internal_metrics::kafka_retry();
                        backoff_ms = backoff_sleep(backoff_ms, shutdown.clone()).await;
                    }
                }
            }
        })
    }
}

struct SignalEventHandler {
    tx: mpsc::Sender<SignalPayload>,
}

#[async_trait]
impl EventHandler for SignalEventHandler {
    async fn handle(&self, env: EventEnvelope) {
        let signal = match serde_json::from_value::<SignalPayload>(env.payload) {
            Ok(s) => s,
            Err(e) => {
                error!("invalid SignalPayload: {e}");
                return;
            }
        };

        internal_metrics::signal_received();

        match self.tx.try_send(signal) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_s)) => {
                internal_metrics::signal_dropped();
                warn!("[agent] signal queue is full; dropping signal (backpressure)");
            }
            Err(mpsc::error::TrySendError::Closed(_s)) => {
                internal_metrics::signal_dropped();
                warn!("[agent] signal queue is closed; dropping signal");
            }
        }
    }
}

async fn backoff_sleep(mut backoff_ms: u64, shutdown: CancellationToken) -> u64 {
    // Exponential backoff with cap.
    let sleep_for = std::time::Duration::from_millis(backoff_ms.min(30_000));
    tokio::select! {
        _ = shutdown.cancelled() => {}
        _ = tokio::time::sleep(sleep_for) => {}
    }
    // Next backoff.
    backoff_ms = (backoff_ms.saturating_mul(2)).min(30_000);
    backoff_ms
}
