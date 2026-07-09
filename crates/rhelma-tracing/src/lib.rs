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
use tracing_subscriber::{EnvFilter, Layer, Registry};

/// Env var selecting the log output format for the fmt layer.
/// `json` (default) — structured JSON lines, parseable by promtail/Loki.
/// `text`/`plain`   — compact human-readable single line (legacy).
/// `pretty`         — multi-line developer-friendly output.
pub const LOG_FORMAT_ENV: &str = "RHELMA_LOG_FORMAT";

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

        // ---- Format Layer (JSON by default; env-overridable) ----
        let fmt_layer = fmt_layer_from_env();

        // ---- Base registry ----
        // fmt layer first (it is typed `Layer<Registry>`), filter on top.
        let base = Registry::default().with(fmt_layer).with(filter);

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

/// Build the fmt (formatting) layer, selecting the output format from the
/// `RHELMA_LOG_FORMAT` env var. JSON by default so logs are parseable by
/// promtail/Loki (Step 2 centralized logging); `text`/`plain` and `pretty`
/// remain available for local development.
///
/// The JSON layer flattens event fields and includes the current span's fields
/// (e.g. `trace_id`/`correlation_id` injected by rhelma-http-observability) so
/// each line is self-describing.
pub fn fmt_layer_from_env() -> Box<dyn Layer<Registry> + Send + Sync> {
    let format = std::env::var(LOG_FORMAT_ENV)
        .unwrap_or_else(|_| "json".to_string())
        .to_lowercase();
    match format.as_str() {
        "text" | "plain" => Box::new(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(false)
                .with_thread_names(false),
        ),
        "pretty" => Box::new(tracing_subscriber::fmt::layer().pretty()),
        _ => Box::new(
            tracing_subscriber::fmt::layer()
                .json()
                .flatten_event(true)
                .with_current_span(true)
                .with_span_list(false)
                .with_target(true),
        ),
    }
}

/// Dependency-light global logging bootstrap for services that do NOT wire the
/// full observability-core stack.
///
/// Installs a process-global subscriber whose format follows `RHELMA_LOG_FORMAT`
/// (JSON by default) and whose level filter follows `RUST_LOG`, falling back to
/// `default_filter` (e.g. `"info"`). Idempotent and fail-open: if a global
/// subscriber is already installed this is a no-op, and it never panics — a
/// service must never fail to start because logging setup raced.
///
/// This is the single structured-logging entry point for the ad-hoc services
/// that previously hand-rolled a plain-text `tracing_subscriber::fmt()` init.
pub fn init_fmt_logging(default_filter: &str) {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(default_filter))
        .unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = Registry::default().with(fmt_layer_from_env()).with(filter);
    // Ignore the error: a global subscriber may already be installed (e.g. tests
    // or a double-init). Logging is best-effort and must not abort startup.
    let _ = tracing::subscriber::set_global_default(subscriber);
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
