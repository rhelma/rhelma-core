//! heartbeat.rs — Rhelma v5.2 Enterprise Heartbeat Emitter
//!
//! Responsibilities:
//!   ✔ Emit heartbeat periodically
//!   ✔ Include residency, region, environment
//!   ✔ Do NOT propagate trace context
//!   ✔ Use v5.2 unified system request context

use std::sync::Arc;
use std::time::Instant;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use rhelma_event::{
    generate_event_id, EventBus, EventEnvelope, EventSource, EventTraceContext, Residency,
};

use crate::agent::config::{ObservabilityAgentConfig, ResidencyMode};
use crate::agent::context::system_request_context;
use crate::io::eventbus_metrics::{
    record_event_publish, record_event_publish_duration, EventBusOutcome,
};
use crate::io::internal_metrics;

/// Topic for heartbeat events
pub const TOPIC_OBS_HEARTBEAT: &str = "obs.heartbeat";

/// Payload for heartbeat events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatPayload {
    /// Service name
    pub service: String,
    /// Service version
    pub service_version: String,
    /// Environment name
    pub environment: String,
    /// Region name
    pub region: String,

    /// Residency mode
    pub residency: String,
    /// Status string
    pub status: String,

    /// Timestamp of heartbeat
    pub timestamp: DateTime<Utc>,
    /// Optional trace identifier
    pub trace_id: Option<String>,
    /// Optional span identifier
    pub span_id: Option<String>,
}

impl HeartbeatPayload {
    /// Creates a new heartbeat payload
    ///
    /// # Arguments
    /// * `cfg` - Agent configuration
    /// * `status` - Status string
    ///
    /// # Returns
    /// A new heartbeat payload instance
    pub fn new(cfg: &ObservabilityAgentConfig, status: String) -> Self {
        Self {
            service: cfg.service_name.clone(),
            service_version: cfg.service_version.clone(),
            environment: cfg.environment.clone(),
            region: cfg.region.clone(),
            residency: cfg.residency_mode.as_str().to_string(),
            status,
            timestamp: Utc::now(),
            trace_id: None,
            span_id: None,
        }
    }

    /// Converts the payload to an event envelope
    ///
    /// # Arguments
    /// * `residency` - Residency mode
    ///
    /// # Returns
    /// Event envelope containing the heartbeat
    pub fn to_envelope(&self, residency: &ResidencyMode) -> EventEnvelope {
        let now = self.timestamp;

        EventEnvelope {
            // Identity
            event_id: generate_event_id(),
            event_version: 1,

            // Routing
            topic: TOPIC_OBS_HEARTBEAT.to_string(),
            key: Some(self.service.clone()),

            // Timestamps
            timestamp: now,
            published_at: now,

            // Source & context
            source: EventSource {
                service: self.service.clone(),
                version: self.service_version.clone(),
                region: self.region.clone(),
            },
            request: system_request_context(residency),

            // Do NOT propagate trace context
            trace: EventTraceContext {
                trace_id: None,
                span_id: None,
                tracestate: None,
                baggage: None,
                parent_span_id: None,
            },

            // Payload
            payload: serde_json::to_value(self).unwrap(),
            payload_type: "rhelma.obs.HeartbeatPayload".to_string(),
            schema_ref: "obs.heartbeat@v1".to_string(),

            // Policy
            policy: rhelma_event::PolicyMeta::public(rhelma_event::purpose::OBSERVABILITY_AGENT),

            // Residency & security
            residency: to_event_residency(residency),
            encryption: None,

            // Integrity
            signature: None,
            hash: None,
        }
    }
}

/// Heartbeat client for emitting periodic heartbeat events
pub struct HeartbeatClient<B: EventBus + Send + Sync + 'static> {
    /// Agent configuration
    pub cfg: Arc<ObservabilityAgentConfig>,
    /// Event bus for publishing
    pub bus: Arc<B>,
    /// Optional status function
    pub status_fn: Option<Arc<dyn Fn() -> String + Send + Sync>>,
}

impl<B> HeartbeatClient<B>
where
    B: EventBus + Send + Sync + 'static,
{
    /// Creates a new heartbeat client
    ///
    /// # Arguments
    /// * `cfg` - Agent configuration
    /// * `bus` - Event bus instance
    ///
    /// # Returns
    /// A new heartbeat client
    pub fn new(cfg: Arc<ObservabilityAgentConfig>, bus: Arc<B>) -> Self {
        Self {
            cfg,
            bus,
            status_fn: None,
        }
    }

    /// Sets a custom status function
    ///
    /// # Arguments
    /// * `f` - Function that returns a status string
    ///
    /// # Returns
    /// Self for method chaining
    pub fn with_status_fn(mut self, f: Arc<dyn Fn() -> String + Send + Sync>) -> Self {
        self.status_fn = Some(f);
        self
    }

    /// Spawns heartbeat loop without shutdown capability
    ///
    /// # Returns
    /// Join handle to the spawned task
    pub fn spawn_loop(&self) -> JoinHandle<()> {
        // Backwards-compatible: no external shutdown.
        self.spawn_loop_with_shutdown(CancellationToken::new())
    }

    /// Spawn heartbeat loop that stops when `shutdown` is cancelled.
    ///
    /// # Arguments
    /// * `shutdown` - Cancellation token for graceful shutdown
    ///
    /// # Returns
    /// Join handle to the spawned task
    pub fn spawn_loop_with_shutdown(&self, shutdown: CancellationToken) -> JoinHandle<()> {
        let cfg = self.cfg.clone();
        let bus = self.bus.clone();
        let status_fn = self.status_fn.clone();
        let interval = cfg.heartbeat_interval();

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            // Make the first tick fire immediately.
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

            loop {
                tokio::select! {
                    _ = shutdown.cancelled() => {
                        internal_metrics::agent_shutdown();
                        tracing::info!("heartbeat loop stopped (shutdown)");
                        break;
                    }
                    _ = ticker.tick() => {
                        let status = status_fn
                            .as_ref()
                            .map(|f| f())
                            .unwrap_or_else(|| "healthy".into());

                        if status != "healthy" {
                            internal_metrics::agent_degraded();
                        }

                        let payload = HeartbeatPayload::new(&cfg, status);
                        let env = payload.to_envelope(&cfg.residency_mode);

                        // publish with metrics
                        let start = Instant::now();

                        let result = {
                            let env = match env.finalize_strict() {
                                Ok(e) => e,
                                Err(e) => {
                                    internal_metrics::heartbeat_failure();
                                    tracing::warn!(error = %e, "heartbeat envelope validation failed");
                                    continue;
                                }
                            };

                            bus.publish(env).await
                        };

                        let elapsed = start.elapsed().as_secs_f64();

                        match result {
                            Ok(_) => {
                                internal_metrics::heartbeat_sent();
                                record_event_publish(TOPIC_OBS_HEARTBEAT, EventBusOutcome::Success);
                                record_event_publish_duration(
                                    TOPIC_OBS_HEARTBEAT,
                                    EventBusOutcome::Success,
                                    elapsed,
                                );
                            }
                            Err(_err) => {
                                internal_metrics::heartbeat_failure();
                                record_event_publish(TOPIC_OBS_HEARTBEAT, EventBusOutcome::Error);
                                record_event_publish_duration(
                                    TOPIC_OBS_HEARTBEAT,
                                    EventBusOutcome::Error,
                                    elapsed,
                                );
                            }
                        }
                    }
                }
            }
        })
    }
}

/// Converts residency mode to event residency
///
/// # Arguments
/// * `mode` - Residency mode
///
/// # Returns
/// Event residency
fn to_event_residency(mode: &ResidencyMode) -> Residency {
    match mode {
        ResidencyMode::Global => Residency::Global,
        ResidencyMode::RegionalPreferred => Residency::RegionalOnly,
        ResidencyMode::RegionalStrict => Residency::RegionStrict,
    }
}
