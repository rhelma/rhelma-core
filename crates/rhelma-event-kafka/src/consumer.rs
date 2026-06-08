#![forbid(unsafe_code)]

use async_trait::async_trait;
use chrono::Utc;
use rand::{thread_rng, Rng};
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use base64::Engine;

use rdkafka::config::ClientConfig;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::BorrowedMessage;
use rdkafka::message::Headers; // <-- IMPORTANT: for headers.count()/get()
use rdkafka::Message;

use rhelma_event::{EventBusError, EventEnvelope};
use rhelma_tracing::context as trace_ctx;
use tracing::Instrument;

#[cfg(feature = "otel")]
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::config::KafkaConfig;
use crate::dlq::{DlqPublisher, DlqRecord};
#[cfg(feature = "otel")]
use crate::headers::otel_context_from_headers_map;
use crate::headers::{
    context_headers_map_from_envelope, context_headers_map_from_kafka_headers_and_envelope,
};
use crate::idempotency::IdempotencyCache;
use crate::metrics;
use crate::reason::DlqReason;
use crate::topic::resolve_topic;

use tokio_util::sync::CancellationToken;

// Needed for StreamConsumer::stream()
use futures::StreamExt;

/// Infallible event handler (back-compat).
#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: EventEnvelope);
}

/// Fallible handler for retry/DLQ flows.
#[async_trait]
pub trait FallibleEventHandler: Send + Sync {
    async fn handle(&self, event: EventEnvelope) -> Result<(), EventBusError>;
}

enum HandlerKind {
    Infallible(Arc<dyn EventHandler>),
    Fallible(Arc<dyn FallibleEventHandler>),
}

/// Kafka subscriber loop.
pub struct KafkaSubscriber {
    cfg: KafkaConfig,
    consumer: StreamConsumer,
    handler: HandlerKind,
    topics: Vec<String>,
    dlq: Option<Arc<DlqPublisher>>,
    idem: Option<Arc<IdempotencyCache>>,
}

/// Context required to publish an event to the DLQ.
///
/// Grouping these fields avoids clippy's `too_many_arguments` lint when building with
/// `-D warnings`.
struct DlqPublishCtx<'a> {
    reason: DlqReason,
    err: String,
    msg: &'a BorrowedMessage<'a>,
    payload: &'a [u8],
    env: Option<&'a EventEnvelope>,
    attempts: Option<u32>,
    retryable: Option<bool>,
}

impl KafkaSubscriber {
    pub fn new(cfg: KafkaConfig, handler: Arc<dyn EventHandler>) -> Result<Self, EventBusError> {
        Self::new_inner(cfg, HandlerKind::Infallible(handler))
    }

    pub fn new_fallible(
        cfg: KafkaConfig,
        handler: Arc<dyn FallibleEventHandler>,
    ) -> Result<Self, EventBusError> {
        Self::new_inner(cfg, HandlerKind::Fallible(handler))
    }

    fn new_inner(cfg: KafkaConfig, handler: HandlerKind) -> Result<Self, EventBusError> {
        cfg.validate_for_consumer()?;

        let mut cc = ClientConfig::new();
        cc.set("bootstrap.servers", &cfg.brokers);
        cc.set("group.id", &cfg.group_id);
        cc.set("enable.partition.eof", "false");
        cc.set("enable.auto.commit", "false");
        cc.set("auto.offset.reset", &cfg.consumer_auto_offset_reset);

        let consumer: StreamConsumer = cc
            .create()
            .map_err(|e| EventBusError::Transport(e.to_string()))?;

        let dlq = if cfg.dlq_enabled {
            let pubr = DlqPublisher::new(cfg.clone()).map_err(EventBusError::Transport)?;
            Some(Arc::new(pubr))
        } else {
            None
        };

        let idem = if cfg.idempotency_enabled {
            Some(Arc::new(IdempotencyCache::new(
                cfg.idempotency_ttl_secs,
                cfg.idempotency_max_entries,
            )))
        } else {
            None
        };

        Ok(Self {
            cfg,
            consumer,
            handler,
            topics: Vec::new(),
            dlq,
            idem,
        })
    }

    /// Subscribe to a single topic (NO wildcard / regex).
    pub async fn subscribe(&mut self, topic: &str) -> Result<(), EventBusError> {
        self.subscribe_many([topic]).await
    }

    /// Subscribe to multiple topics (NO wildcard / regex).
    pub async fn subscribe_many<'a, I>(&mut self, topics: I) -> Result<(), EventBusError>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut real_topics = Vec::new();
        for t in topics {
            let real = resolve_topic(self.cfg.topic_prefix.as_str(), t)?;
            real_topics.push(real);
        }

        if real_topics.is_empty() {
            return Err(EventBusError::Serialization("no topics provided".into()));
        }

        real_topics.sort();
        real_topics.dedup();

        self.consumer
            .subscribe(&real_topics.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .map_err(|e| EventBusError::Transport(e.to_string()))?;

        self.topics = real_topics;
        Ok(())
    }

    /// Run the subscriber loop.
    pub async fn run(&self) -> Result<(), EventBusError> {
        // Back-compat: infinite loop unless the underlying consumer stream ends.
        self.run_with_shutdown(CancellationToken::new()).await
    }

    /// Run the subscriber loop until a shutdown signal is triggered.
    pub async fn run_with_shutdown(
        &self,
        shutdown: CancellationToken,
    ) -> Result<(), EventBusError> {
        let mut stream = self.consumer.stream();

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    tracing::info!(topics=?self.topics, "kafka subscriber shutdown requested");
                    // Best-effort unsubscribe; the consumer will be dropped by the caller.
                    self.consumer.unsubscribe();
                    break;
                }

                maybe = stream.next() => {
                    let Some(result) = maybe else {
                        // Stream ended.
                        break;
                    };

                    let msg = match result {
                        Ok(m) => m,
                        Err(e) => {
                            tracing::error!(error=?e, "kafka consume error");
                            continue;
                        }
                    };

                    if let Err(e) = self.process_message(&msg).await {
                        tracing::error!(error=?e, "kafka message processing failed");
                    }
                }
            }
        }

        Ok(())
    }

    async fn process_message(&self, msg: &BorrowedMessage<'_>) -> Result<(), EventBusError> {
        let payload = match msg.payload() {
            Some(p) => p,
            None => {
                // no payload, commit and continue
                metrics::inc_consume_empty();
                self.commit(msg)?;
                return Ok(());
            }
        };

        metrics::inc_consume();

        // First, attempt decode
        let env: EventEnvelope = match serde_json::from_slice(payload) {
            Ok(e) => e,
            Err(e) => {
                metrics::inc_decode_error();
                self.dlq_decode(DlqReason::DecodeError, e.to_string(), msg, payload)
                    .await;
                self.commit(msg)?;
                return Ok(());
            }
        };

        // Idempotency by event_id
        if let Some(idem) = &self.idem {
            let first = idem.check_and_mark(env.event_id.as_str()).await;
            if !first {
                metrics::inc_idempotency_duplicate();
                tracing::debug!(event_id=%env.event_id, "duplicate event skipped");
                self.commit(msg)?;
                return Ok(());
            }
        }

        // Dispatch (bind Kafka envelope headers into task-local trace context for correlation).
        let ctx_headers = msg
            .headers()
            .map(|h| context_headers_map_from_kafka_headers_and_envelope(h, &env))
            .unwrap_or_else(|| context_headers_map_from_envelope(&env));

        trace_ctx::scope_with_headers(&ctx_headers, async move {
            let span = tracing::info_span!(
                "kafka.event",
                topic = %msg.topic(),
                partition = msg.partition(),
                offset = msg.offset(),
                event_id = %env.event_id,
                schema_ref = %env.schema_ref,
                residency = %env.residency.as_str(),
                region = %env.source.region
            );

            // If the service installed tracing-opentelemetry, attach the upstream
            // OTEL trace context as the parent for this per-message span.
            // This produces proper end-to-end trace graphs in OTLP backends.
            #[cfg(feature = "otel")]
            {
                let parent = otel_context_from_headers_map(&ctx_headers);
                span.set_parent(parent);
            }

            async move {
                match &self.handler {
                    HandlerKind::Infallible(h) => {
                        let t0 = Instant::now();
                        h.handle(env).await;
                        metrics::observe_handler_latency_ms(t0.elapsed().as_millis() as u64);
                        metrics::inc_handled();
                        self.commit(msg)?;
                        Ok(())
                    }

                    HandlerKind::Fallible(h) => {
                        let mut attempt = 0u32;
                        loop {
                            attempt += 1;
                            let t0 = Instant::now();

                            match h.handle(env.clone()).await {
                                Ok(()) => {
                                    metrics::observe_handler_latency_ms(
                                        t0.elapsed().as_millis() as u64
                                    );
                                    metrics::inc_handled();
                                    self.commit(msg)?;
                                    return Ok(());
                                }

                                Err(err) => {
                                    metrics::observe_handler_latency_ms(
                                        t0.elapsed().as_millis() as u64
                                    );

                                    let retryable_now = is_retryable(&err);
                                    if attempt >= self.cfg.handler_retry_max_attempts
                                        || !retryable_now
                                    {
                                        let retryable = Some(retryable_now);
                                        metrics::inc_handler_failure(retryable);
                                        self.dlq_handler(DlqPublishCtx {
                                            reason: DlqReason::HandlerError,
                                            err: format!("{err:?}"),
                                            msg,
                                            payload,
                                            env: Some(&env),
                                            attempts: Some(attempt),
                                            retryable,
                                        })
                                        .await;
                                        self.commit(msg)?;
                                        return Ok(());
                                    }

                                    metrics::inc_handler_retry();
                                    let sleep = backoff(
                                        self.cfg.handler_retry_base_ms,
                                        self.cfg.handler_retry_max_ms,
                                        attempt,
                                    );

                                    tracing::warn!(
                                        attempt,
                                        sleep_ms = sleep.as_millis() as u64,
                                        error=?err,
                                        "handler failed; retrying"
                                    );

                                    tokio::time::sleep(sleep).await;
                                }
                            }
                        }
                    }
                }
            }
            .instrument(span)
            .await
        })
        .await
    }

    fn commit(&self, msg: &BorrowedMessage<'_>) -> Result<(), EventBusError> {
        self.consumer
            .commit_message(msg, CommitMode::Async)
            .map_err(|e| EventBusError::Transport(e.to_string()))
    }

    async fn dlq_decode(
        &self,
        reason: DlqReason,
        err: String,
        msg: &BorrowedMessage<'_>,
        payload: &[u8],
    ) {
        self.dlq_handler(DlqPublishCtx {
            reason,
            err,
            msg,
            payload,
            env: None,
            attempts: None,
            retryable: Some(false),
        })
        .await;
    }

    async fn dlq_handler<'a>(&self, ctx: DlqPublishCtx<'a>) {
        self.publish_dlq(ctx).await;
    }

    async fn publish_dlq<'a>(&self, ctx: DlqPublishCtx<'a>) {
        let DlqPublishCtx {
            reason,
            err,
            msg,
            payload,
            env,
            attempts,
            retryable,
        } = ctx;
        let Some(dlq) = &self.dlq else {
            return;
        };

        metrics::inc_dlq_record(reason, retryable);

        let (payload_b64, truncated) = encode_payload(payload, dlq.max_payload_bytes());

        let hints = DlqHints::from_kafka_message(msg);

        // Prefer decoded envelope fields (strong contract) but fall back to Kafka headers
        // so decode failures still carry identity/correlation.
        let event_id = env
            .map(|e| e.event_id.clone())
            .or_else(|| hints.event_id.clone());
        let schema_ref = env
            .map(|e| e.schema_ref.clone())
            .or_else(|| hints.schema_ref.clone());

        let traceparent = env
            .and_then(|e| {
                e.trace
                    .trace_id
                    .as_deref()
                    .zip(e.trace.span_id.as_deref())
                    .map(|(t, s)| format!("00-{t}-{s}-01"))
            })
            .or_else(|| hints.traceparent.clone());

        let (trace_id, span_id) =
            match env.and_then(|e| e.trace.trace_id.as_deref().zip(e.trace.span_id.as_deref())) {
                Some((t, s)) => (Some(t.to_string()), Some(s.to_string())),
                None => (hints.trace_id.clone(), hints.span_id.clone()),
            };

        let request_id = env
            .and_then(|e| e.request.request_id.clone())
            .or_else(|| hints.request_id.clone());
        let correlation_id = env
            .and_then(|e| e.request.correlation_id.clone())
            .or_else(|| hints.correlation_id.clone());
        let tenant_id = env
            .and_then(|e| e.request.tenant_id.clone())
            .or_else(|| hints.tenant_id.clone());
        let region = env
            .map(|e| e.source.region.clone())
            .filter(|s| !s.trim().is_empty())
            .or_else(|| hints.region.clone());

        let original_timestamp_ms = match msg.timestamp() {
            rdkafka::message::Timestamp::NotAvailable => None,
            rdkafka::message::Timestamp::CreateTime(ms) => Some(ms),
            rdkafka::message::Timestamp::LogAppendTime(ms) => Some(ms),
        };

        let record = DlqRecord {
            reason,
            retryable,
            attempts,
            error: err.to_string(),
            original_topic: Some(msg.topic().to_string()), // <-- FIX: msg.topic() is &str
            partition: Some(msg.partition()),
            offset: Some(msg.offset()),
            original_timestamp_ms,

            event_id,
            schema_ref,
            traceparent,
            trace_id,
            span_id,
            request_id,
            correlation_id,
            tenant_id,
            region,
            dlq_at: Utc::now(),
            payload_b64,
            truncated,
        };

        let key = format!("{}:{}:{}", msg.topic(), msg.partition(), msg.offset()); // <-- FIX
        if let Err(e) = dlq.publish(&record, &key).await {
            tracing::error!(error=%e, "failed to publish DLQ record");
        } else {
            tracing::warn!(reason=%record.reason.as_str(), "message sent to DLQ");
        }
    }
}

fn is_retryable(err: &EventBusError) -> bool {
    matches!(err, EventBusError::Transport(_))
}

fn backoff(base_ms: u64, max_ms: u64, attempt: u32) -> Duration {
    // exponential backoff with jitter
    let exp = 2u64.saturating_pow(attempt.saturating_sub(1).min(16));
    let raw = base_ms.saturating_mul(exp).min(max_ms);
    let jitter: u64 = thread_rng().gen_range(0..=raw / 3 + 1);
    Duration::from_millis(raw + jitter)
}

fn encode_payload(payload: &[u8], max: usize) -> (String, bool) {
    let truncated = payload.len() > max;
    let slice = if truncated { &payload[..max] } else { payload };
    (
        base64::engine::general_purpose::STANDARD.encode(slice),
        truncated,
    )
}

#[derive(Debug, Default, Clone)]
struct DlqHints {
    event_id: Option<String>,
    schema_ref: Option<String>,

    traceparent: Option<String>,
    trace_id: Option<String>,
    span_id: Option<String>,

    request_id: Option<String>,
    correlation_id: Option<String>,
    tenant_id: Option<String>,
    region: Option<String>,
}

impl DlqHints {
    fn from_kafka_message(msg: &BorrowedMessage<'_>) -> Self {
        let traceparent = header_str(msg, "traceparent");
        let (tp_trace_id, tp_span_id) = traceparent
            .as_deref()
            .and_then(parse_traceparent)
            .map(|(t, s)| (Some(t), Some(s)))
            .unwrap_or((None, None));

        let trace_id = tp_trace_id
            .or_else(|| header_str(msg, "x-trace-id"))
            .or_else(|| header_str(msg, "x-rhelma-trace-id"));
        let span_id = tp_span_id
            .or_else(|| header_str(msg, "x-span-id"))
            .or_else(|| header_str(msg, "x-rhelma-span-id"));

        Self {
            event_id: header_str(msg, "x-rhelma-event-id"),
            schema_ref: header_str(msg, "x-rhelma-schema-ref"),
            traceparent,
            trace_id,
            span_id,
            request_id: header_str(msg, "x-rhelma-request-id")
                .or_else(|| header_str(msg, "x-request-id")),
            correlation_id: header_str(msg, "x-rhelma-correlation-id")
                .or_else(|| header_str(msg, "x-correlation-id")),
            tenant_id: header_str(msg, "x-tenant-id"),
            region: header_str(msg, "x-region"),
        }
    }
}

fn header_str(msg: &BorrowedMessage<'_>, key: &str) -> Option<String> {
    let headers = msg.headers()?;

    for i in 0..headers.count() {
        let h = headers.get(i); // <-- FIX: get(i) returns Header, not Option
        if h.key == key {
            return h
                .value
                .and_then(|v| std::str::from_utf8(v).ok())
                .map(|s| s.to_string());
        }
    }

    None
}

fn parse_traceparent(tp: &str) -> Option<(String, String)> {
    // Expected: "00-<32hex trace_id>-<16hex span_id>-<2hex flags>"
    let parts: Vec<&str> = tp.split('-').collect();
    if parts.len() != 4 {
        return None;
    }
    let trace_id = parts[1];
    let span_id = parts[2];
    if trace_id.len() != 32 || span_id.len() != 16 {
        return None;
    }
    Some((trace_id.to_string(), span_id.to_string()))
}
