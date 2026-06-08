#![forbid(unsafe_code)]

//! rhelma-observability-core — Rhelma v5.2 aligned.
//!
//! Responsibilities (core-only):
//! - Wire rhelma-logger, rhelma-tracing, rhelma-metrics from `UnifiedObservabilityConfig`
//! - Install a default PII redactor
//! - Expose an in-process health snapshot (logger/tracing/metrics)
//!
//! NOTE:
//! Heartbeats, audit, anomaly detection, and command/decision loops belong to
//! `rhelma-observability-agent`.

mod error;
mod health;
mod mapping;
mod trace_metrics;
mod wiring;

pub use error::*;
pub use health::*;
pub use mapping::*;
pub use trace_metrics::*;
pub use wiring::WiredComponents;

use std::sync::{Arc, Mutex};

use rhelma_config::{CentralEnv, UnifiedObservabilityConfig};

/// High-level handle used by services to access observability.
#[derive(Clone)]
pub struct ObservabilityCore {
    /// Final resolved unified configuration.
    pub config: UnifiedObservabilityConfig,
    /// Thread-safe wiring results (tracing/metrics + health).
    pub shared: Arc<Mutex<WiredComponents>>,
}

impl ObservabilityCore {
    fn lock_shared(&self) -> std::sync::MutexGuard<'_, WiredComponents> {
        self.shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Dev/simple path: load from env only.
    ///
    /// # Errors
    /// Returns `ObsError` if configuration cannot be loaded/validated or if wiring fails.
    #[deprecated(
        note = "init_from_env_only is deprecated; prefer init_from_central/init_from_unified"
    )]
    pub async fn init_from_env_only(service_name: &str) -> ObsResult<Self> {
        let central = CentralEnv::from_env();
        Self::init_from_central(&central, service_name).await
    }

    /// Preferred path when using `rhelma-config`: take a resolved `CentralEnv`.
    ///
    /// # Errors
    /// Returns `ObsError` if configuration cannot be loaded/validated or if wiring fails.
    pub async fn init_from_central(central: &CentralEnv, service_name: &str) -> ObsResult<Self> {
        let unified = UnifiedObservabilityConfig::from_central_env(central, service_name);
        Self::init_from_unified(unified).await
    }

    /// Lowest-level path: initialize from an already-built unified config.
    ///
    /// # Errors
    /// Returns `ObsError` if wiring fails (logger/tracing/metrics initialization).
    pub async fn init_from_unified(unified: UnifiedObservabilityConfig) -> ObsResult<Self> {
        // Validation is delegated to the underlying crates (logger/tracing/metrics).
        let wired = wiring::wire_all(&unified).await?;
        Ok(Self {
            config: unified,
            shared: Arc::new(Mutex::new(wired)),
        })
    }

    /// Best-effort runtime toggle for metrics.
    ///
    /// Important: `rhelma-metrics` is a global singleton. We can *enable* metrics
    /// (if not yet initialized) and we can *locally* disable by dropping the handle,
    /// but we cannot safely "uninstall" the global instance at runtime.
    pub fn reload_metrics(&self, cfg: &UnifiedObservabilityConfig) {
        use rhelma_metrics::{
            global as global_metrics, init_global as init_metrics_global, RhelmaMetrics,
        };

        // Compute the new state outside the lock so readers never observe a transient `None`.
        let (new_metrics, new_status) = if !cfg.enable_metrics {
            (None, HealthStatus::Disabled)
        } else if let Some(g) = global_metrics() {
            (Some(g), HealthStatus::Healthy)
        } else {
            // First-time init.
            let metrics_cfg = crate::mapping::to_metrics_config(cfg);
            let metrics = RhelmaMetrics::with_config(metrics_cfg);

            match init_metrics_global(metrics.clone()) {
                Ok(()) => (Some(metrics), HealthStatus::Healthy),
                Err(e) => {
                    // Race recovery: someone else may have initialized in parallel.
                    let g = global_metrics();
                    if g.is_some() {
                        (g, HealthStatus::Healthy)
                    } else {
                        tracing::warn!(error = %e, "metrics reload failed");
                        (None, HealthStatus::Degraded)
                    }
                }
            }
        };

        let mut shared = self.lock_shared();

        shared.metrics = new_metrics;
        shared.health.set_metrics(new_status);
    }

    /// Get a snapshot of current health.
    #[must_use]
    pub fn health(&self) -> CoreHealthSnapshot {
        self.lock_shared().health.snapshot()
    }

    /// Optional tracing handle (if tracing is enabled).
    #[must_use]
    pub fn tracing(&self) -> Option<rhelma_tracing::RhelmaTracing> {
        self.lock_shared().tracing.clone()
    }

    /// Optional metrics handle (if metrics are enabled).
    #[must_use]
    pub fn metrics(&self) -> Option<rhelma_metrics::RhelmaMetrics> {
        self.lock_shared().metrics.clone()
    }
}
