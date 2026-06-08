//! rhelma-tracing v1.4.0-enterprise-pro
//!
//! Rhelma v5.2-compliant distributed tracing core.
//!
//! Responsibilities:
//! - Build a tracing subscriber (but does NOT auto-install by default).
//! - Optional OTEL exporter layer via exporters/otlp.rs.
//! - Provide stable config + prelude + instrumentation macros.
//!
//! observability-core decides how/when to install the subscriber globally.

pub mod business;
pub mod config;
pub mod context;
pub mod exporters;
pub mod macros;
pub mod prelude;

#[cfg(feature = "kafka")]
pub mod kafka_propagation;

pub use config::{TracingConfig, TracingConfigError};

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry};

/// Main tracing handle for Rhelma services.
#[derive(Clone)]
pub struct RhelmaTracing {
    /// Field `config`.
    pub config: TracingConfig,
}

impl RhelmaTracing {
    /// Initialize from `rhelma-config::UnifiedObservabilityConfig` (v5.2 wiring).
    pub async fn init_from_unified(
        service_name: &str,
        obs: &rhelma_config::UnifiedObservabilityConfig,
    ) -> Result<Self, TracingConfigError> {
        let cfg = TracingConfig::from_unified(obs);
        Self::init(service_name, cfg).await
    }

    /// Convenience initializer used by examples/tests.
    ///
    /// NOTE: This does *not* install a global subscriber. For real services,
    /// observability-core should call `build_subscriber` and manage installation.
    pub async fn init(
        service_name: &str,
        mut cfg: TracingConfig,
    ) -> Result<Self, TracingConfigError> {
        if cfg.service_name.trim().is_empty() {
            cfg.service_name = service_name.to_string();
        }
        cfg.validate()?;
        Ok(Self { config: cfg })
    }

    /// Build a subscriber WITHOUT installing it globally.
    ///
    /// observability-core will handle init().
    pub fn build_subscriber(
        &self,
    ) -> Result<Box<dyn tracing::Subscriber + Send + Sync>, TracingConfigError> {
        self.config.validate()?;

        // ---- Env Filter ----
        let filter = EnvFilter::try_new(self.config.level.clone()).map_err(|e| {
            TracingConfigError::InvalidConfig(format!("invalid tracing filter: {e}"))
        })?;

        // ---- Pretty/Console Layer ----
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_target(false)
            .with_thread_ids(false)
            .with_thread_names(false);

        // ---- Base registry ----
        let base = Registry::default().with(filter).with(fmt_layer);

        // ---- Optional OTEL ----
        let subscriber: Box<dyn tracing::Subscriber + Send + Sync> = {
            #[cfg(feature = "otel")]
            {
                if self.config.otel_enabled {
                    // Fail-open: observability must never take the service down.
                    // If the OTLP endpoint is unavailable at startup, continue without OTEL.
                    match exporters::init_otel_layer::<_>(&self.config) {
                        Ok(otel_layer) => Box::new(base.with(otel_layer)),
                        Err(e) => {
                            eprintln!(
                                "[rhelma-tracing] OTEL exporter init failed; continuing without OTEL: {e}"
                            );
                            Box::new(base)
                        }
                    }
                } else {
                    Box::new(base)
                }
            }

            #[cfg(not(feature = "otel"))]
            {
                if self.config.otel_enabled {
                    return Err(TracingConfigError::Otel(
                        "OTEL is enabled by config but rhelma-tracing was built without the 'otel' feature".into(),
                    ));
                }
                Box::new(base)
            }
        };

        Ok(subscriber)
    }

    /// Install tracing subscriber globally.
    ///
    /// Recommended ONLY for:
    /// - CLI tools
    /// - Non-observability-core services
    pub fn init_global(&self) -> Result<(), TracingConfigError> {
        let subscriber = self.build_subscriber()?;
        tracing::subscriber::set_global_default(subscriber)
            .map_err(|e| TracingConfigError::Setup(e.to_string()))
    }

    /// Instance-based sampling helper that uses `config.sampling_rate`.
    pub fn should_sample(&self) -> bool {
        should_sample(self.config.sampling_rate)
    }
}

/// Head-based sampling helper used by tests and services.
///
/// - rate <= 0.0 → always false
/// - rate >= 1.0 → always true
/// - otherwise   → random(<rate)
pub fn should_sample(rate: f64) -> bool {
    if rate <= 0.0 {
        return false;
    }
    if rate >= 1.0 {
        return true;
    }
    let r: f64 = rand::random();
    r < rate
}
