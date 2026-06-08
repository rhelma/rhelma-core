//! Anomaly detector module for signal processing and incident escalation

use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;

use rhelma_event::{
    generate_event_id, EventBus, EventEnvelope, EventSource, EventTraceContext, Residency,
};

use crate::agent::config::ResidencyMode;
// ✅ بهترین مسیر برای سازگاری (همان چیزی که کامپایلر پیشنهاد داد)
use crate::ObservabilityAgentConfig;

use crate::agent::context::system_request_context;
use crate::agent::ObservabilityAgent;
use crate::ai::{AiIncidentProposed, TOPIC_AI_INCIDENT_PROPOSED};
use crate::error::AgentError;
use crate::io::internal_metrics;
use crate::reflex::signals::SignalPayload;

/// Output of sync phase (no async allowed)
pub struct SignalAction {
    /// Optional insight event
    pub insight: Option<EventEnvelope>,
    /// Optional alert event
    pub alert: Option<EventEnvelope>,
    /// Optional incident event
    pub incident: Option<EventEnvelope>,
}

// Hard safety cap (even if config is mis-set)
const HARD_MAX_WINDOW_SIZE: usize = 10_000;

/// Naive anomaly detector for signal processing and incident escalation
pub struct NaiveAnomalyDetector<B: EventBus + Send + Sync + 'static> {
    /// Service name
    pub service: String,
    /// Service version
    pub service_version: String,
    /// Environment name
    pub environment: String,
    /// Region name
    pub region: String,
    /// Residency mode
    pub residency: ResidencyMode,
    /// Event bus for publishing events
    pub bus: Arc<B>,

    /// Maximum number of signals in the window (soft cap)
    pub window_limit: u32,
    /// Duration of the sampling window
    pub window_duration: Duration,
    /// Samples in the current window
    pub window_samples: Vec<(Instant, String)>,

    /// Timestamp of last incident
    pub last_incident_at: Option<Instant>,
    /// Minimum interval between incidents
    pub min_incident_interval: Duration,
}

impl<B> NaiveAnomalyDetector<B>
where
    B: EventBus + Send + Sync + 'static,
{
    /// Convenience constructor that takes the agent config (Arc) and bus.
    ///
    /// This is used heavily in integration tests and keeps call sites consistent
    /// with `ObservabilityAgent::new(cfg: Arc<_>, ...)`.
    pub fn new_with_bus(cfg: Arc<ObservabilityAgentConfig>, bus: Arc<B>) -> Self {
        let mut det = Self::new(
            cfg.service_name.clone(),
            cfg.service_version.clone(),
            cfg.environment.clone(),
            cfg.region.clone(),
            cfg.residency_mode.clone(),
            bus,
            cfg.anomaly_window_size,
        );

        // Optional: tie escalation interval to stale threshold (best-effort).
        // If stale_threshold_secs isn't meant for this in your design, you can remove this line.
        if cfg.stale_threshold_secs > 0 {
            det.min_incident_interval = Duration::from_secs(cfg.stale_threshold_secs);
        }

        det
    }

    /// Creates a new anomaly detector
    ///
    /// # Arguments
    /// * `service` - Service name
    /// * `version` - Service version
    /// * `env` - Environment name
    /// * `region` - Region name
    /// * `residency` - Residency mode
    /// * `bus` - Event bus instance
    /// * `window_limit` - Maximum signals in window
    ///
    /// # Returns
    /// A new anomaly detector instance
    pub fn new(
        service: String,
        version: String,
        env: String,
        region: String,
        residency: ResidencyMode,
        bus: Arc<B>,
        window_limit: u32,
    ) -> Self {
        Self {
            service,
            service_version: version,
            environment: env,
            region,
            residency,
            bus,
            window_limit,
            window_duration: Duration::from_secs(60),
            window_samples: Vec::new(),
            last_incident_at: None,
            min_incident_interval: Duration::from_secs(60),
        }
    }

    /// Processes a signal synchronously and returns recommended actions
    pub fn process_signal_sync(
        &mut self,
        agent: &ObservabilityAgent<B>,
        signal: SignalPayload,
    ) -> Result<SignalAction, AgentError> {
        let now = Instant::now();

        self.prune_window(now);

        // Keep a small rolling sample window for debugging/diagnostics.
        self.window_samples.push((now, signal.kind.clone()));

        // ✅ Enforce both soft cap (config) and hard cap (safety).
        self.enforce_window_cap();

        let severity = agent.effective_severity_sync(&signal.severity);

        let mut action = SignalAction {
            insight: None,
            alert: None,
            incident: None,
        };

        // INFO
        if severity == "info" {
            action.insight = Some(self.build_obs_envelope("obs.insight", &signal));
            internal_metrics::insight_sent();
            return Ok(action);
        }

        // WARNING
        if severity == "warning" {
            action.alert = Some(self.build_obs_envelope("obs.alert", &signal));
            internal_metrics::alert_sent();
            return Ok(action);
        }

        // CRITICAL
        if severity == "critical" {
            action.alert = Some(self.build_obs_envelope("obs.alert", &signal));
            internal_metrics::alert_sent();

            if self.can_escalate(now) {
                self.last_incident_at = Some(now);
                action.incident = Some(self.build_incident(&signal));
                internal_metrics::incident_sent();
            }
        }

        Ok(action)
    }

    /// Executes the action asynchronously by publishing the event
    pub async fn execute_action_async(&self, env: EventEnvelope) -> Result<(), AgentError> {
        let env = env.finalize_strict()?;
        self.bus.publish(env).await?;
        Ok(())
    }

    fn build_obs_envelope(&self, topic: &str, s: &SignalPayload) -> EventEnvelope {
        let now = Utc::now();

        EventEnvelope {
            // Identity
            event_id: generate_event_id(),
            event_version: 1,

            // Routing
            topic: topic.to_string(),
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
            request: system_request_context(&self.residency),
            trace: EventTraceContext {
                trace_id: None,
                span_id: None,
                tracestate: None,
                baggage: None,
                parent_span_id: None,
            },

            // Payload
            payload: serde_json::json!({
                "service": self.service,
                "service_version": self.service_version,
                "environment": self.environment,
                "region": self.region,
                "kind": s.kind,
                "severity": s.severity,
                "message": s.message,
                "metrics": s.metrics,
            }),
            payload_type: "rhelma.obs.SignalPayload".to_string(),
            schema_ref: format!("{topic}@v1"),

            // Policy
            policy: rhelma_event::PolicyMeta::public(rhelma_event::purpose::OBSERVABILITY_AGENT),

            // Residency & security
            residency: to_event_residency(&self.residency),
            encryption: None,

            // Integrity
            signature: None,
            hash: None,
        }
    }

    fn build_incident(&self, s: &SignalPayload) -> EventEnvelope {
        let detected_at = Utc::now();

        let incident = AiIncidentProposed {
            incident_id: generate_event_id(),
            service: self.service.clone(),
            service_version: self.service_version.clone(),
            environment: self.environment.clone(),
            region: self.region.clone(),
            detected_at,
            kind: s.kind.clone(),
            severity: "critical".into(),
            message: s.message.clone(),
            metrics: s.metrics.clone(),
            category: None,
            tags: None,
            confidence: Some(1.0),
            version: Some(1),
            dedupe_key: None,
            candidates: None,
            trace_id: None,
            span_id: None,
        };

        EventEnvelope {
            event_id: generate_event_id(),
            event_version: 1,
            topic: TOPIC_AI_INCIDENT_PROPOSED.to_string(),
            key: Some(self.service.clone()),
            timestamp: detected_at,
            published_at: Utc::now(),

            source: EventSource {
                service: self.service.clone(),
                version: self.service_version.clone(),
                region: self.region.clone(),
            },
            request: system_request_context(&self.residency),
            trace: EventTraceContext {
                trace_id: None,
                span_id: None,
                tracestate: None,
                baggage: None,
                parent_span_id: None,
            },

            payload: serde_json::to_value(&incident).unwrap(),
            payload_type: "rhelma.ai.AiIncidentProposed".to_string(),
            schema_ref: "ai.incident.proposed@v1".to_string(),

            // Policy
            policy: rhelma_event::PolicyMeta::public(rhelma_event::purpose::OBSERVABILITY_AGENT),

            residency: to_event_residency(&self.residency),
            encryption: None,

            signature: None,
            hash: None,
        }
    }

    fn prune_window(&mut self, now: Instant) {
        self.window_samples
            .retain(|(t, _)| now.duration_since(*t) <= self.window_duration);
    }

    fn enforce_window_cap(&mut self) {
        // Soft cap from config (if non-zero), but always bounded by HARD_MAX_WINDOW_SIZE.
        let soft = if self.window_limit == 0 {
            HARD_MAX_WINDOW_SIZE
        } else {
            self.window_limit as usize
        };
        let cap = soft.min(HARD_MAX_WINDOW_SIZE);

        if self.window_samples.len() > cap {
            let drop = self.window_samples.len() - cap;
            self.window_samples.drain(0..drop);
        }
    }

    fn can_escalate(&self, now: Instant) -> bool {
        match self.last_incident_at {
            None => true,
            Some(last) => now.duration_since(last) >= self.min_incident_interval,
        }
    }

    /// Window length accessor for integration tests.
    ///
    /// Kept public because integration tests live outside the crate.
    #[doc(hidden)]
    #[allow(dead_code)]
    pub fn window_len_for_test(&self) -> usize {
        self.window_samples.len()
    }
}

/// Converts residency mode to event residency
pub fn to_event_residency(mode: &ResidencyMode) -> Residency {
    match mode {
        ResidencyMode::Global => Residency::Global,
        ResidencyMode::RegionalPreferred => Residency::RegionalOnly,
        ResidencyMode::RegionalStrict => Residency::RegionStrict,
    }
}
