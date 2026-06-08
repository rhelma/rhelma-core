use crate::config::MetricsConfig;
use crate::RhelmaMetrics;

/// Fluent builder for RhelmaMetrics.
///
/// توجه: این builder فقط structهای داخلی را می‌سازد.
/// هیچ exporter یا recorderای را مقداردهی اولیه نمی‌کند.
pub struct MetricsBuilder {
    pub(crate) config: MetricsConfig,
}

impl MetricsBuilder {
    /// Create a new builder for the given service name.
    pub fn new(service_name: &str) -> Self {
        Self {
            config: MetricsConfig::new(service_name),
        }
    }

    /// Override namespace.
    pub fn namespace(mut self, namespace: &str) -> Self {
        self.config.namespace = namespace.to_owned();
        self
    }

    /// Add a default label.
    pub fn with_label(mut self, key: &str, value: &str) -> Self {
        self.config
            .default_labels
            .push((key.to_owned(), value.to_owned()));
        self
    }

    /// Finalize the builder and create a RhelmaMetrics instance.
    pub fn build(self) -> RhelmaMetrics {
        RhelmaMetrics::with_config(self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_builds_metrics() {
        let metrics = MetricsBuilder::new("svc")
            .namespace("custom")
            .with_label("env", "dev")
            .build();

        metrics.record_http_request("GET", "/x", 200, 0.01);
    }
}
