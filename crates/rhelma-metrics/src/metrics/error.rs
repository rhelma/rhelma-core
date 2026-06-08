use metrics::counter;

/// Error-related metrics for Rhelma services, aligned with v5.1.
///
/// Naming convention:
///   rhelma_errors_total{
///       type="",       // canonical error class
///       source="",     // static subsystem identifier
///       service="",
///       environment="",
///       region=""
///   }
///
/// Labels:
///   - type:   canonical error class ("db", "network", "timeout", "panic", ...)
///   - source: static identifier for subsystem / operation
///   - service / environment / region: static deployment metadata
///
/// Design:
/// - Zero-allocation hot path.
/// - NO dynamic or user-provided strings allowed.
/// - Only static & canonical identifiers.
/// - Cardinality always bounded.
#[derive(Debug, Clone)]
pub struct ErrorMetrics {
    service_name: &'static str,
    environment: &'static str,
    region: &'static str,
}

impl ErrorMetrics {
    /// Create new instance — all inputs MUST already be `'static`.
    ///
    /// NOTE:
    ///   MetricRegistry is responsible for Box::leak() exactly once
    ///   during startup, so ErrorMetrics never performs allocations.
    pub fn new(
        service_name: &'static str,
        environment: &'static str,
        region: Option<&'static str>,
    ) -> Self {
        Self {
            service_name,
            environment,
            region: region.unwrap_or("unknown"),
        }
    }

    /// Record a generic error with canonical type + source.
    ///
    /// Both error_type and source MUST be `'static str`.
    pub fn record_error(&self, error_type: &'static str, source: &'static str) {
        counter!(
            "rhelma_errors_total",
            "type" => error_type,
            "source" => source,
            "service" => self.service_name,
            "environment" => self.environment,
            "region" => self.region,
        )
        .increment(1);
    }

    // ------------------------------------------------------------------------
    // Canonical helpers (recommended)
    // ------------------------------------------------------------------------

    /// Database-related error (query, transaction, connection, etc.)
    pub fn record_database_error(&self, operation: &'static str) {
        self.record_error("db", operation);
    }

    /// Network-related error (HTTP call, upstream dependency, etc.)
    pub fn record_network_error(&self, endpoint: &'static str) {
        self.record_error("network", endpoint);
    }

    /// Timeout failures (DB, network, cache, etc.)
    pub fn record_timeout(&self, op: &'static str) {
        self.record_error("timeout", op);
    }

    /// Unexpected panic inside component or subsystem.
    pub fn record_panic(&self, component: &'static str) {
        self.record_error("panic", component);
    }

    /// Internal error unrelated to external I/O.
    pub fn record_internal_error(&self, component: &'static str) {
        self.record_error("internal", component);
    }
}

#[cfg(test)]
mod tests {
    use super::ErrorMetrics;

    #[test]
    fn basic_error_metrics() {
        let m = ErrorMetrics::new("svc", "dev", Some("eu"));

        m.record_error("custom", "somewhere");
        m.record_database_error("select_users");
        m.record_network_error("/api/external");
        m.record_timeout("db_query");
        m.record_internal_error("state_machine");
        m.record_panic("worker_loop");
    }
}
