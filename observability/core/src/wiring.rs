//! Wiring for logger/tracing/metrics for `rhelma-observability-core` (v5.2).

use crate::error::{ObsResult, ObservabilityError};
use crate::health::{HealthCenter, HealthMetadata, HealthStatus};
use crate::mapping::{to_metrics_config, to_tracing_config};

use rhelma_config::UnifiedObservabilityConfig;
use rhelma_logger::{set_redactor, DefaultPiiRedactor, RhelmaLogger};
use rhelma_metrics::{global as global_metrics, init_global as init_metrics_global, RhelmaMetrics};
use rhelma_tracing::RhelmaTracing;

use std::io::{self, Write};

/// Writes a line to stderr.
fn write_stderr_line(msg: &str) {
    let _ = io::stderr().write_all(msg.as_bytes());
    let _ = io::stderr().write_all(b"\n");
    let _ = io::stderr().flush();
}

/// All wired components (core-only).
#[derive(Clone)]
pub struct WiredComponents {
    /// Tracing component.
    pub tracing: Option<RhelmaTracing>,
    /// Metrics component.
    pub metrics: Option<RhelmaMetrics>,
    /// Health center.
    pub health: HealthCenter,
}

/// Full wiring process.
///
/// # Arguments
/// * `cfg` - Unified observability configuration
///
/// # Returns
/// `ObsResult<WiredComponents>` - Wired components or error
#[allow(clippy::too_many_lines)]
pub async fn wire_all(cfg: &UnifiedObservabilityConfig) -> ObsResult<WiredComponents> {
    let health = HealthCenter::new_with_metadata(HealthMetadata {
        service_name: Some(cfg.service_name.clone()),
        environment: Some(crate::mapping::env_to_string(&cfg.environment)),
        region: Some(cfg.region.clone()),
        service_version: Some(cfg.service_version.clone()),
    });

    let mut tracing_opt: Option<RhelmaTracing> = None;
    let mut metrics_opt: Option<RhelmaMetrics> = None;

    // 1) LOGGER (fatal)
    {
        set_redactor(Box::new(DefaultPiiRedactor));

        match RhelmaLogger::init_from_unified(cfg, None) {
            Ok(()) => health.set_logger(HealthStatus::Healthy),
            Err(e) => {
                write_stderr_line(&format!(
                    "[FATAL][rhelma-observability-core] logger init failed: {e}"
                ));
                health.set_logger(HealthStatus::Down);
                return Err(ObservabilityError::Logger(e));
            }
        }
    }

    // 2) TRACING (best-effort unless required by policy)
    {
        let tracing_cfg = to_tracing_config(cfg);

        match tracing_cfg.validate() {
            Ok(()) => match RhelmaTracing::init(&cfg.service_name, tracing_cfg).await {
                Ok(t) => {
                    // Install a process-global subscriber (required for distributed tracing).
                    match t.init_global() {
                        Ok(()) => {
                            tracing_opt = Some(t);
                            health.set_tracing(HealthStatus::Healthy);
                        }
                        Err(e) => {
                            // If someone already installed a subscriber, treat as ok.
                            let msg = e.to_string();
                            if msg.contains("global default subscriber")
                                || msg.contains("already been set")
                            {
                                tracing_opt = Some(t);
                                health.set_tracing(HealthStatus::Healthy);
                            } else if cfg.otel_required {
                                write_stderr_line(&format!(
                                    "[FATAL][rhelma-observability-core] tracing required but init failed: {e}"
                                ));
                                health.set_tracing(HealthStatus::Down);
                                return Err(ObservabilityError::Tracing(e));
                            } else {
                                write_stderr_line(&format!(
                                    "[WARN][rhelma-observability-core] tracing init failed: {e}"
                                ));
                                health.set_tracing(HealthStatus::Degraded);
                                tracing_opt = Some(t);
                            }
                        }
                    }
                }
                Err(e) => {
                    if cfg.otel_required {
                        write_stderr_line(&format!(
                            "[FATAL][rhelma-observability-core] tracing required but init failed: {e}"
                        ));
                        health.set_tracing(HealthStatus::Down);
                        return Err(ObservabilityError::Tracing(e));
                    }
                    write_stderr_line(&format!(
                        "[WARN][rhelma-observability-core] tracing init failed: {e}"
                    ));
                    health.set_tracing(HealthStatus::Degraded);
                }
            },
            Err(err) => {
                if cfg.otel_required {
                    write_stderr_line(&format!(
                        "[FATAL][rhelma-observability-core] tracing required but config invalid: {err}"
                    ));
                    health.set_tracing(HealthStatus::Down);
                    return Err(ObservabilityError::Tracing(err));
                }
                write_stderr_line(&format!(
                    "[WARN][rhelma-observability-core] tracing config invalid: {err}"
                ));
                health.set_tracing(HealthStatus::Degraded);
            }
        }
    }

    // 3) METRICS (best-effort)
    {
        if cfg.enable_metrics {
            if let Some(g) = global_metrics() {
                metrics_opt = Some(g);
                health.set_metrics(HealthStatus::Healthy);
            } else {
                let metrics_cfg = to_metrics_config(cfg);
                let metrics = RhelmaMetrics::with_config(metrics_cfg);

                match init_metrics_global(metrics.clone()) {
                    Ok(()) => {
                        metrics_opt = Some(metrics);
                        health.set_metrics(HealthStatus::Healthy);
                    }
                    Err(e) => {
                        // Best-effort recovery: someone may have initialized metrics concurrently.
                        metrics_opt = global_metrics();
                        if metrics_opt.is_some() {
                            health.set_metrics(HealthStatus::Healthy);
                        } else {
                            write_stderr_line(&format!(
                                "[WARN][rhelma-observability-core] metrics init failed: {e}"
                            ));
                            health.set_metrics(HealthStatus::Degraded);
                        }
                    }
                }
            }
        } else {
            health.set_metrics(HealthStatus::Disabled);
        }
    }

    Ok(WiredComponents {
        tracing: tracing_opt,
        metrics: metrics_opt,
        health,
    })
}
