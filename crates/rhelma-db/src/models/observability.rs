use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sqlx::FromRow;

/// Table: observability_defaults
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ObservabilityDefaults {
    /// Field `id`.
    pub id: i32,
    /// Field `json_logs`.
    pub json_logs: Option<bool>,
    /// Field `console_logs`.
    pub console_logs: Option<bool>,
    /// Field `log_level`.
    pub log_level: Option<String>,
    /// Field `sampling_rate`.
    pub sampling_rate: Option<f64>,
    /// Field `otel_enabled`.
    pub otel_enabled: Option<bool>,
    /// Field `otel_endpoint`.
    pub otel_endpoint: Option<String>,
    /// Field `metrics_enabled`.
    pub metrics_enabled: Option<bool>,
    /// Field `prometheus_port`.
    pub prometheus_port: Option<i32>,
    /// Field `performance_profile`.
    pub performance_profile: Option<String>,
}

/// Table: observability_regions
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ObservabilityRegionConfig {
    /// Field `region`.
    pub region: String,
    /// Field `json_logs`.
    pub json_logs: Option<bool>,
    /// Field `console_logs`.
    pub console_logs: Option<bool>,
    /// Field `log_level`.
    pub log_level: Option<String>,
    /// Field `sampling_rate`.
    pub sampling_rate: Option<f64>,
    /// Field `otel_enabled`.
    pub otel_enabled: Option<bool>,
    /// Field `otel_endpoint`.
    pub otel_endpoint: Option<String>,
    /// Field `metrics_enabled`.
    pub metrics_enabled: Option<bool>,
    /// Field `prometheus_port`.
    pub prometheus_port: Option<i32>,
    /// Field `performance_profile`.
    pub performance_profile: Option<String>,
}

/// Table: observability_services
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ObservabilityServiceConfig {
    /// Field `region`.
    pub region: String,
    /// Field `service_name`.
    pub service_name: String,
    /// Field `json_logs`.
    pub json_logs: Option<bool>,
    /// Field `console_logs`.
    pub console_logs: Option<bool>,
    /// Field `log_level`.
    pub log_level: Option<String>,
    /// Field `sampling_rate`.
    pub sampling_rate: Option<f64>,
    /// Field `otel_enabled`.
    pub otel_enabled: Option<bool>,
    /// Field `otel_endpoint`.
    pub otel_endpoint: Option<String>,
    /// Field `metrics_enabled`.
    pub metrics_enabled: Option<bool>,
    /// Field `prometheus_port`.
    pub prometheus_port: Option<i32>,
    /// Field `performance_profile`.
    pub performance_profile: Option<String>,
}

pub trait ObservabilityRow {
    /// fn `json_logs`.
    fn json_logs(&self) -> Option<bool>;
    /// fn `console_logs`.
    fn console_logs(&self) -> Option<bool>;
    /// fn `log_level`.
    fn log_level(&self) -> Option<&str>;
    /// fn `sampling_rate`.
    fn sampling_rate(&self) -> Option<f64>;
    /// fn `otel_enabled`.
    fn otel_enabled(&self) -> Option<bool>;
    /// fn `otel_endpoint`.
    fn otel_endpoint(&self) -> Option<&str>;
    /// fn `metrics_enabled`.
    fn metrics_enabled(&self) -> Option<bool>;
    /// fn `prometheus_port`.
    fn prometheus_port(&self) -> Option<i32>;
    /// fn `performance_profile`.
    fn performance_profile(&self) -> Option<&str>;
}

macro_rules! impl_row {
    ($t:ty) => {
        impl ObservabilityRow for $t {
            fn json_logs(&self) -> Option<bool> {
                self.json_logs
            }
            fn console_logs(&self) -> Option<bool> {
                self.console_logs
            }
            fn log_level(&self) -> Option<&str> {
                self.log_level.as_deref()
            }
            fn sampling_rate(&self) -> Option<f64> {
                self.sampling_rate
            }
            fn otel_enabled(&self) -> Option<bool> {
                self.otel_enabled
            }
            fn otel_endpoint(&self) -> Option<&str> {
                self.otel_endpoint.as_deref()
            }
            fn metrics_enabled(&self) -> Option<bool> {
                self.metrics_enabled
            }
            fn prometheus_port(&self) -> Option<i32> {
                self.prometheus_port
            }
            fn performance_profile(&self) -> Option<&str> {
                self.performance_profile.as_deref()
            }
        }
    };
}

impl_row!(ObservabilityDefaults);
impl_row!(ObservabilityRegionConfig);
impl_row!(ObservabilityServiceConfig);

/// DB row -> partial override JSON (only fields that are Some(..))
pub fn map_row_to_value<R: ObservabilityRow>(row: &R) -> Value {
    let mut m = Map::new();

    if let Some(v) = row.json_logs() {
        m.insert("json_enabled".into(), json!(v));
    }
    if let Some(v) = row.console_logs() {
        m.insert("console_enabled".into(), json!(v));
    }
    if let Some(v) = row.log_level() {
        m.insert("log_level".into(), json!(v));
    }
    if let Some(v) = row.sampling_rate() {
        m.insert("sampling_rate".into(), json!(v));
    }
    if let Some(v) = row.otel_enabled() {
        m.insert("otel_enabled".into(), json!(v));
    }
    if let Some(v) = row.otel_endpoint() {
        m.insert("otel_endpoint".into(), json!(v));
    }
    if let Some(v) = row.metrics_enabled() {
        m.insert("enable_metrics".into(), json!(v));
    }
    if let Some(v) = row.prometheus_port() {
        if v > 0 {
            m.insert("prometheus_port".into(), json!(v as u16));
        }
    }
    if let Some(v) = row.performance_profile() {
        m.insert("performance_profile".into(), json!(v));
    }

    Value::Object(m)
}
