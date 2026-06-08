use serde::{Deserialize, Serialize};

/// Runtime hints for metrics exporter selection (hybrid Prometheus + OTLP).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsRuntimeConfig {
    /// Field `enabled`.
    pub enabled: bool,
    /// Field `otlp_endpoint`.
    pub otlp_endpoint: Option<String>,
    /// Field `prometheus_port`.
    pub prometheus_port: Option<u16>,
}

impl MetricsRuntimeConfig {
    /// Build from process environment.
    #[deprecated(
        note = "from_env() is deprecated; use rhelma-config UnifiedObservabilityConfig instead"
    )]
    pub fn from_env() -> Self {
        let enabled = read_bool(
            &[
                "RHELMA_OBS__METRICS_ENABLED",
                "RHELMA_OBSERVABILITY__METRICS_ENABLED",
                "RHELMA_METRICS_ENABLED",
            ],
            true,
        );

        let otlp_endpoint = read_first_string(&[
            "RHELMA_OBS__OTEL_ENDPOINT",
            "RHELMA_OBSERVABILITY__OTEL_ENDPOINT",
            "RHELMA_OTEL_ENDPOINT",
        ]);

        let prometheus_port = read_first_u16(&[
            "RHELMA_OBS__PROMETHEUS_PORT",
            "RHELMA_OBSERVABILITY__PROMETHEUS_PORT",
            "RHELMA_PROMETHEUS_PORT",
        ]);

        Self {
            enabled,
            otlp_endpoint,
            prometheus_port,
        }
    }

    /// Build runtime hints from rhelma-config unified config.
    #[cfg(feature = "with-config")]
    pub fn from_unified(unified: &rhelma_config::UnifiedObservabilityConfig) -> Self {
        Self {
            enabled: unified.enable_metrics,
            otlp_endpoint: if unified.otel_enabled {
                unified.otel_endpoint.clone()
            } else {
                None
            },
            prometheus_port: Some(unified.prometheus_port),
        }
    }
}

fn read_bool(names: &[&str], default: bool) -> bool {
    for &name in names {
        if let Ok(raw) = std::env::var(name) {
            let v = raw.to_lowercase();
            return matches!(v.as_str(), "1" | "true" | "yes" | "on");
        }
    }
    default
}

fn read_first_string(names: &[&str]) -> Option<String> {
    for &name in names {
        if let Ok(v) = std::env::var(name) {
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

fn read_first_u16(names: &[&str]) -> Option<u16> {
    for &name in names {
        if let Ok(raw) = std::env::var(name) {
            if let Ok(v) = raw.parse::<u16>() {
                return Some(v);
            }
        }
    }
    None
}
