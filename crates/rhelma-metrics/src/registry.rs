use crate::config::MetricsConfig;
use crate::metrics::{ErrorMetrics, HttpMetrics, SystemMetrics};
use crate::{cache, db};
use smallvec::SmallVec;

/// Central registry that owns all metric helpers for a Rhelma service.
///
/// Responsibilities:
/// - Inject service/environment/region into all metric helpers.
/// - Forward DB/Cache/EventBus helper calls with correct labels.
/// - Provide a unified hot-path API for Rhelma v5.1 Observability.
#[derive(Debug, Clone)]
pub struct MetricRegistry {
    /// Field `http`.
    pub http: HttpMetrics,
    /// Field `system`.
    pub system: SystemMetrics,
    /// Field `errors`.
    pub errors: ErrorMetrics,

    /// Field `service_name`.
    pub service_name: &'static str,
    /// Field `environment`.
    pub environment: &'static str,
    /// Field `region`.
    pub region: &'static str,

    /// Field `namespace`.
    pub namespace: String,
    /// Field `default_labels`.
    pub default_labels: Vec<(&'static str, &'static str)>,
}

impl MetricRegistry {
    pub fn new(cfg: &MetricsConfig) -> Self {
        let service_static = Box::leak(cfg.service_name.clone().into_boxed_str());
        let environment_static = Box::leak(cfg.environment.clone().into_boxed_str());
        let region_static = Box::leak(
            cfg.region
                .clone()
                .unwrap_or_else(|| "unknown".into())
                .into_boxed_str(),
        );

        let mut default_labels_static: Vec<(&'static str, &'static str)> = Vec::new();

        for (k, v) in &cfg.default_labels {
            let key_static = Box::leak(k.clone().into_boxed_str());
            let value_static = Box::leak(v.clone().into_boxed_str());
            default_labels_static.push((key_static, value_static));
        }

        MetricRegistry {
            http: HttpMetrics::new(service_static, environment_static, Some(region_static)),
            system: SystemMetrics::new(service_static, environment_static, Some(region_static)),
            errors: ErrorMetrics::new(service_static, environment_static, Some(region_static)),

            service_name: service_static,
            environment: environment_static,
            region: region_static,

            namespace: cfg.namespace.clone(),
            default_labels: default_labels_static,
        }
    }

    /// Register all Rhelma metric descriptors.
    pub fn register_all(&self) {
        crate::metrics::register_descriptors();
    }

    // ----------------------------------------------------
    // DB metrics
    // ----------------------------------------------------
    pub fn record_db_query(
        &self,
        duration_seconds: f64,
        operation: &'static str,
        outcome: &'static str,
    ) {
        db::record_db_query_with_labels(duration_seconds, operation, outcome, &self.default_labels);
    }

    pub fn record_db_connection_error(&self) {
        db::record_db_connection_error_with_labels(&self.default_labels);
    }

    // ----------------------------------------------------
    // EventBus metrics (Message Fabric)
    // ----------------------------------------------------
    pub fn record_event_publish(
        &self,
        topic: &'static str,
        outcome: crate::eventbus::EventBusOutcome,
    ) {
        // service/environment/region labels
        let extra = [
            ("service", self.service_name),
            ("environment", self.environment),
            ("region", self.region),
        ];

        crate::eventbus::record_event_publish_with_labels(topic, outcome, &extra);
    }

    pub fn record_event_publish_success(&self, topic: &'static str) {
        self.record_event_publish(topic, crate::eventbus::EventBusOutcome::Success);
    }

    pub fn record_event_publish_error(&self, topic: &'static str) {
        self.record_event_publish(topic, crate::eventbus::EventBusOutcome::Error);
    }

    pub fn record_event_publish_duration(
        &self,
        topic: &'static str,
        outcome: crate::eventbus::EventBusOutcome,
        duration_secs: f64,
    ) {
        let extra = [
            ("service", self.service_name),
            ("environment", self.environment),
            ("region", self.region),
        ];

        crate::eventbus::record_event_publish_duration_with_labels(
            topic,
            outcome,
            duration_secs,
            &extra,
        );
    }

    // ----------------------------------------------------
    // Cache metrics
    // ----------------------------------------------------
    pub fn record_cache_hit(
        &self,
        backend: &'static str,
        operation: &'static str,
        key_space: &'static str,
    ) {
        cache::record_cache_hit(backend, operation, key_space);
    }

    pub fn record_cache_miss(
        &self,
        backend: &'static str,
        operation: &'static str,
        key_space: &'static str,
    ) {
        cache::record_cache_miss(backend, operation, key_space);
    }

    pub fn record_cache_error(
        &self,
        backend: &'static str,
        operation: &'static str,
        key_space: &'static str,
    ) {
        cache::record_cache_error(backend, operation, key_space);
    }

    // ----------------------------------------------------
    // HTTP metrics
    // ----------------------------------------------------
    pub fn record_http_request(
        &self,
        method: &str,
        endpoint: &str,
        status: u16,
        duration_secs: f64,
    ) {
        self.http.record(method, endpoint, status, duration_secs);
    }

    pub fn record_http_request_with_bytes(
        &self,
        method: &str,
        endpoint: &str,
        status: u16,
        duration_secs: f64,
        request_bytes: u64,
        response_bytes: u64,
    ) {
        self.http.record_with_bytes(
            method,
            endpoint,
            status,
            duration_secs,
            request_bytes,
            response_bytes,
        );
    }

    pub fn record_http_request_with_labels(
        &self,
        method: &str,
        endpoint: &'static str,
        status: u16,
        duration_secs: f64,
        extra: &[(&'static str, &'static str)],
    ) {
        self.http
            .record_with_labels(method, endpoint, status, duration_secs, extra);
    }

    // ----------------------------------------------------
    // Error metrics
    // ----------------------------------------------------
    pub fn record_error(&self, error_type: &'static str, source: &'static str) {
        self.errors.record_error(error_type, source);
    }

    pub fn record_database_error(&self, operation: &'static str) {
        self.errors.record_database_error(operation);
    }

    pub fn record_network_error(&self, endpoint: &'static str) {
        self.errors.record_network_error(endpoint);
    }

    pub fn record_timeout(&self, operation: &'static str) {
        self.errors.record_timeout(operation);
    }

    // ----------------------------------------------------
    // System metrics
    // ----------------------------------------------------
    pub fn set_active_connections(&self, count: u64) {
        self.system.set_active_connections(count);
    }

    pub fn increment_connections(&self) {
        self.system.increment_connections();
    }

    pub fn decrement_connections(&self) {
        self.system.decrement_connections();
    }

    pub fn set_memory_usage_bytes(&self, bytes: u64) {
        self.system.set_memory_usage_bytes(bytes);
    }

    pub fn set_cpu_usage_percent(&self, percent: f64) {
        self.system.set_cpu_usage_percent(percent);
    }

    pub fn add_disk_read_bytes(&self, bytes: u64) {
        self.system.add_disk_read_bytes(bytes);
    }

    pub fn add_disk_write_bytes(&self, bytes: u64) {
        self.system.add_disk_write_bytes(bytes);
    }

    pub fn add_network_received_bytes(&self, bytes: u64) {
        self.system.add_network_received_bytes(bytes);
    }

    pub fn add_network_sent_bytes(&self, bytes: u64) {
        self.system.add_network_sent_bytes(bytes);
    }

    // ----------------------------------------------------
    // Business metrics (custom)
    // ----------------------------------------------------
    pub fn record_business_metric(
        &self,
        name: &'static str,
        value: u64,
        extra: &[(&'static str, &'static str)],
    ) {
        use metrics::counter;

        debug_assert!(
            3 + self.default_labels.len() + extra.len() <= 32,
            "too many labels; violates Rhelma cardinality rules"
        );
        let mut labels: SmallVec<[(&'static str, &'static str); 32]> = SmallVec::new();
        labels.extend_from_slice(&[
            ("service", self.service_name),
            ("environment", self.environment),
            ("region", self.region),
        ]);
        labels.extend_from_slice(&self.default_labels);
        labels.extend_from_slice(extra);

        counter!(name, labels.as_slice()).increment(value);
    }
}
