//! rhelma-metrics v0.5.0 (aligned with Rhelma v5.1)
//!
//! Thin, deterministic metrics helpers for the Rhelma platform.
//!
//! Design:
//! - No exporter/recorder config here (handled by observability-core / agent).
//! - Provides stable schemas + optimized hot-path helpers.
//! - Integrates cleanly with Rhelma v5.1 config, tenancy, and region semantics.

mod auth;
mod cache;
mod config;
mod db;
mod env;
mod eventbus;
mod metrics;
mod registry;

pub use auth::*;
pub use cache::*;
pub use config::MetricsConfig;
pub use db::*;
pub use env::MetricsRuntimeConfig;
pub use eventbus::*;
pub use metrics::{ErrorMetrics, HttpMetrics, SystemMetrics};
pub use registry::MetricRegistry;

use std::sync::OnceLock;

/// Lightweight snapshot for service metadata.
#[derive(Debug, Clone)]
pub struct MetricsStateSnapshot {
    /// Field `service_name`.
    pub service_name: String,
    /// Field `namespace`.
    pub namespace: String,
    /// Field `environment`.
    pub environment: String,
    /// Field `region`.
    pub region: Option<String>,
}

/// Root metrics handle for Rhelma services.
/// Created by observability-core and shared globally.
#[derive(Clone)]
pub struct RhelmaMetrics {
    /// Field `config`.
    pub config: MetricsConfig,
    /// Field `registry`.
    pub registry: MetricRegistry,
}

static GLOBAL_METRICS: OnceLock<RhelmaMetrics> = OnceLock::new();

impl RhelmaMetrics {
    /// Create metrics using a unified config.
    pub fn with_config(config: MetricsConfig) -> Self {
        let registry = MetricRegistry::new(&config);
        registry.register_all();
        Self { config, registry }
    }

    pub fn registry(&self) -> &MetricRegistry {
        &self.registry
    }

    // --------------------------------------------------------------------
    // Convenience forwarders (service-level metrics API)
    // --------------------------------------------------------------------

    pub fn record_http_request(
        &self,
        method: &str,
        endpoint: &str,
        status: u16,
        duration_secs: f64,
    ) {
        self.registry
            .record_http_request(method, endpoint, status, duration_secs);
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
        self.registry.record_http_request_with_bytes(
            method,
            endpoint,
            status,
            duration_secs,
            request_bytes,
            response_bytes,
        );
    }

    pub fn record_error(&self, error_type: &'static str, source: &'static str) {
        self.registry.record_error(error_type, source);
    }

    pub fn record_db_query(
        &self,
        duration_seconds: f64,
        operation: &'static str,
        outcome: &'static str,
    ) {
        self.registry
            .record_db_query(duration_seconds, operation, outcome);
    }
    // ----------------------------------------------------
    // EventBus Metrics
    // ----------------------------------------------------

    pub fn record_event_publish(
        &self,
        topic: &'static str,
        outcome: crate::eventbus::EventBusOutcome,
    ) {
        self.registry.record_event_publish(topic, outcome);
    }

    pub fn record_event_publish_success(&self, topic: &'static str) {
        self.registry.record_event_publish_success(topic);
    }

    pub fn record_event_publish_error(&self, topic: &'static str) {
        self.registry.record_event_publish_error(topic);
    }

    pub fn record_event_publish_duration(
        &self,
        topic: &'static str,
        outcome: crate::eventbus::EventBusOutcome,
        duration_secs: f64,
    ) {
        self.registry
            .record_event_publish_duration(topic, outcome, duration_secs);
    }
    // ----------------------------------------------------
    // cache  Metrics
    // ----------------------------------------------------
    pub fn record_cache_hit(
        &self,
        backend: &'static str,
        operation: &'static str,
        key_space: &'static str,
    ) {
        self.registry
            .record_cache_hit(backend, operation, key_space);
    }

    pub fn record_cache_miss(
        &self,
        backend: &'static str,
        operation: &'static str,
        key_space: &'static str,
    ) {
        self.registry
            .record_cache_miss(backend, operation, key_space);
    }

    pub fn record_cache_error(
        &self,
        backend: &'static str,
        operation: &'static str,
        key_space: &'static str,
    ) {
        self.registry
            .record_cache_error(backend, operation, key_space);
    }

    pub fn set_active_connections(&self, count: u64) {
        self.registry.set_active_connections(count);
    }

    pub fn set_memory_usage_bytes(&self, bytes: u64) {
        self.registry.set_memory_usage_bytes(bytes);
    }
}

// ----------------------------------------------------------------------
// Global accessors
// ----------------------------------------------------------------------

/// Install a global metrics instance.
/// Returns error if already initialized.
pub fn init_global(m: RhelmaMetrics) -> Result<(), &'static str> {
    GLOBAL_METRICS
        .set(m)
        .map_err(|_| "metrics already initialized")
}

/// Get global metrics instance.
pub fn global() -> Option<RhelmaMetrics> {
    GLOBAL_METRICS.get().cloned()
}

/// Return a snapshot of global metrics state.
pub fn global_snapshot() -> Option<MetricsStateSnapshot> {
    GLOBAL_METRICS.get().map(|m| MetricsStateSnapshot {
        service_name: m.config.service_name.clone(),
        namespace: m.config.namespace.clone(),
        environment: m.config.environment.clone(),
        region: m.config.region.clone(),
    })
}

#[cfg(feature = "with-config")]
pub fn init_from_unified(
    service_name: &str,
    unified: &rhelma_config::UnifiedObservabilityConfig,
) -> Result<(), &'static str> {
    let cfg = MetricsConfig::from_unified(service_name, unified);
    let runtime = MetricsRuntimeConfig::from_unified(unified);

    if !runtime.enabled {
        return Ok(());
    }
    let metrics = RhelmaMetrics::with_config(cfg);
    init_global(metrics)
}
