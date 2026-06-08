//! agent.rs — Core state of Observability-Agent (Enterprise)
//!
//! Holds shared state such as:
//!   - degraded mode
//!   - sampling rate
//!   - AI override table
//!   - config access
//!   - active AI decision state (v5.2)

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use chrono::{DateTime, Utc};
use rhelma_event::{EventBus, EventEnvelope};
use tracing::warn;

use crate::agent::config::ObservabilityAgentConfig;
use crate::error::AgentError;

/// Severity override applied by AI Decisions
#[derive(Debug, Clone)]
pub enum EffectiveSeverity {
    /// Critical severity level
    Critical,
    /// Warning severity level
    Warning,
    /// Info severity level
    Info,
    /// Skip publishing
    SkipPublish,
}

/// Canonical AI decision state applied to the agent (Rhelma v5.2)
#[derive(Debug, Clone)]
pub struct AiDecisionState {
    /// Incident identifier
    pub incident_id: String,
    /// Time when decision was received
    pub received_at: DateTime<Utc>,

    /// Optional severity override
    pub override_severity: Option<EffectiveSeverity>,
    /// Optional degraded mode override
    pub degraded_mode: Option<bool>,
    /// Optional sampling override (0–100)
    pub sampling_override: Option<u32>,

    /// Optional expiration time
    pub expires_at: Option<DateTime<Utc>>,
}

/// Core agent shared state
pub struct ObservabilityAgent<B: EventBus + Send + Sync + 'static> {
    /// Agent configuration
    pub cfg: Arc<ObservabilityAgentConfig>,
    /// Event bus for publishing
    pub bus: Arc<B>,

    /// Latest active AI decision (if any)
    ai_decision: Arc<RwLock<Option<AiDecisionState>>>,

    /// Legacy per-incident severity overrides (kept for backward compatibility)
    pub ai_overrides: Arc<Mutex<HashMap<String, EffectiveSeverity>>>,

    /// Degraded mode flag
    pub degraded: Arc<AtomicBool>,

    /// Sampling override for observability (percentage 0–100)
    pub sampling: Arc<AtomicU32>,
}

impl<B> ObservabilityAgent<B>
where
    B: EventBus + Send + Sync + 'static,
{
    /// Creates a new observability agent
    ///
    /// # Arguments
    /// * `cfg` - Agent configuration
    /// * `bus` - Event bus instance
    ///
    /// # Returns
    /// A new observability agent instance
    pub fn new(cfg: Arc<ObservabilityAgentConfig>, bus: Arc<B>) -> Self {
        let degraded_initial = cfg.degraded_mode_initial;

        Self {
            cfg: cfg.clone(),
            bus,
            ai_decision: Arc::new(RwLock::new(None)),
            ai_overrides: Arc::new(Mutex::new(HashMap::new())),
            degraded: Arc::new(AtomicBool::new(degraded_initial)),
            sampling: Arc::new(AtomicU32::new(100)),
        }
    }

    // ─────────────────────────────────────────────────────────────
    // AI Decision handling (v5.2)
    // ─────────────────────────────────────────────────────────────

    /// Store a new AI decision and apply its side-effects atomically
    ///
    /// # Arguments
    /// * `decision` - AI decision to apply
    pub fn apply_ai_decision_state(&self, decision: AiDecisionState) {
        // persist decision
        {
            let mut guard = self.ai_decision.write().unwrap();
            *guard = Some(decision.clone());
        }

        // apply degraded mode override
        if let Some(d) = decision.degraded_mode {
            self.degraded.store(d, Ordering::Relaxed);
        }

        // apply sampling override
        if let Some(s) = decision.sampling_override {
            self.sampling.store(s.min(100), Ordering::Relaxed);
        }

        // apply severity override (scoped by incident)
        if let Some(sev) = decision.override_severity.clone() {
            self.ai_overrides
                .lock()
                .unwrap()
                .insert(decision.incident_id.clone(), sev);
        }
    }

    /// Get the currently active AI decision (if any and not expired)
    ///
    /// # Returns
    /// Active AI decision or None if expired or not present
    pub fn active_ai_decision(&self) -> Option<AiDecisionState> {
        let guard = self.ai_decision.read().unwrap();
        let decision = guard.clone()?;

        if let Some(exp) = decision.expires_at {
            if Utc::now() > exp {
                return None;
            }
        }

        Some(decision)
    }

    // ─────────────────────────────────────────────────────────────
    // Publishing & severity helpers
    // ─────────────────────────────────────────────────────────────

    /// Should this severity be skipped in degraded mode?
    ///
    /// # Arguments
    /// * `severity` - Severity level to check
    ///
    /// # Returns
    /// `true` if publishing should be skipped, `false` otherwise
    pub fn should_skip_publish(&self, severity: &str) -> bool {
        if self.degraded.load(Ordering::Relaxed) {
            // enterprise degraded mode: drop insights & warnings
            matches!(severity, "info" | "warning")
        } else {
            false
        }
    }

    /// AI override lookup by incident id
    ///
    /// # Arguments
    /// * `incident_id` - Incident identifier
    ///
    /// # Returns
    /// Effective severity override if exists
    pub fn get_ai_override_for(&self, incident_id: &str) -> Option<EffectiveSeverity> {
        self.ai_overrides.lock().unwrap().get(incident_id).cloned()
    }

    /// Async helper used by KafkaSignalSource for safe publish.
    ///
    /// Best-effort:
    /// - A failure to finalize/publish one envelope will be logged, but will not prevent
    ///   the rest of the action from being executed.
    ///
    /// # Arguments
    /// * `action` - Signal action to execute
    ///
    /// # Returns
    /// `Result<(), AgentError>` - First error encountered or success
    pub async fn execute_detector_async(
        &self,
        action: crate::reflex::anomaly::SignalAction,
    ) -> Result<(), AgentError> {
        async fn publish_one<B: EventBus + Send + Sync + 'static>(
            bus: &B,
            env: EventEnvelope,
            label: &'static str,
        ) -> Result<(), AgentError> {
            let env = match env.finalize_strict() {
                Ok(e) => e,
                Err(e) => {
                    warn!(stage = "finalize", kind = label, error = %e, "detector envelope dropped");
                    return Err(AgentError::from(e));
                }
            };

            match bus.publish(env).await {
                Ok(()) => Ok(()),
                Err(e) => {
                    warn!(stage = "publish", kind = label, error = %e, "detector publish failed");
                    Err(AgentError::from(e))
                }
            }
        }

        let mut first_err: Option<AgentError> = None;

        if let Some(env) = action.insight {
            if let Err(e) = publish_one(self.bus.as_ref(), env, "insight").await {
                first_err.get_or_insert(e);
            }
        }
        if let Some(env) = action.alert {
            if let Err(e) = publish_one(self.bus.as_ref(), env, "alert").await {
                first_err.get_or_insert(e);
            }
        }
        if let Some(env) = action.incident {
            if let Err(e) = publish_one(self.bus.as_ref(), env, "incident").await {
                first_err.get_or_insert(e);
            }
        }

        if let Some(e) = first_err {
            return Err(e);
        }

        Ok(())
    }

    /// Sync severity normalization used by anomaly detectors
    ///
    /// # Arguments
    /// * `sev` - Original severity string
    ///
    /// # Returns
    /// Effective severity string considering degraded mode
    pub fn effective_severity_sync(&self, sev: &str) -> String {
        if self.degraded.load(Ordering::Relaxed) {
            match sev {
                "critical" => "critical".into(),
                "warning" => "warning".into(),
                _ => "info".into(),
            }
        } else {
            sev.to_string()
        }
    }
}
