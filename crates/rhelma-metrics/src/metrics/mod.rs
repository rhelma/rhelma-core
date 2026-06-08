//! Metric types + stable descriptor registrations for Rhelma v5.1.
//!
//! IMPORTANT:
//! - All metric names MUST be prefixed with `rhelma_`.
//! - No exporter/recorder logic here (handled by observability-core).
//! - This file defines ONLY the semantic contract of platform metrics.

mod error;
mod http;
mod system;

pub use error::ErrorMetrics;
pub use http::HttpMetrics;
pub use system::SystemMetrics;

use metrics::{describe_counter, describe_gauge, describe_histogram};

/// Register ALL metric descriptors of the Rhelma platform.
///
/// This MUST be called exactly once during RhelmaMetrics::with_config.
pub fn register_descriptors() {
    // ------------------------
    // HTTP Layer
    // ------------------------
    describe_counter!(
        "rhelma_http_requests_total",
        "Total number of HTTP requests"
    );

    describe_histogram!(
        "rhelma_http_request_duration_seconds",
        "HTTP request duration in seconds"
    );

    describe_counter!(
        "rhelma_http_request_bytes_total",
        "Total HTTP request bytes"
    );

    describe_counter!(
        "rhelma_http_response_bytes_total",
        "Total HTTP response bytes"
    );

    describe_counter!(
        "rhelma_http_endpoint_cardinality_clamped_total",
        "Number of times HTTP endpoint labels were clamped to /other due to cardinality limits"
    );

    describe_gauge!(
        "rhelma_http_endpoint_unique",
        "Number of unique HTTP endpoint labels observed in-process"
    );

    // ------------------------
    // System Metrics
    // ------------------------
    describe_gauge!(
        "rhelma_system_active_connections",
        "Number of active TCP connections"
    );

    describe_gauge!(
        "rhelma_system_memory_usage_bytes",
        "Current memory usage in bytes"
    );

    describe_gauge!(
        "rhelma_system_cpu_usage_ratio",
        "CPU usage ratio in [0.0, 1.0]"
    );

    describe_counter!(
        "rhelma_system_disk_read_bytes_total",
        "Total disk read bytes"
    );

    describe_counter!(
        "rhelma_system_disk_write_bytes_total",
        "Total disk write bytes"
    );

    describe_counter!(
        "rhelma_system_network_received_bytes_total",
        "Total network received bytes"
    );

    describe_counter!(
        "rhelma_system_network_sent_bytes_total",
        "Total network sent bytes"
    );

    // ------------------------
    // Error Metrics
    // ------------------------
    describe_counter!(
        "rhelma_errors_total",
        "Total number of errors in the service"
    );

    // ------------------------
    // Database
    // ------------------------
    describe_counter!("rhelma_db_query_total", "Total number of DB queries");

    describe_histogram!(
        "rhelma_db_query_duration_seconds",
        "DB query latency in seconds"
    );

    describe_counter!("rhelma_db_connection_errors_total", "DB connection errors");

    // ------------------------
    // Cache
    // ------------------------
    describe_counter!("rhelma_cache_hit_total", "Cache hits");
    describe_counter!("rhelma_cache_miss_total", "Cache misses");

    // ------------------------
    // EventBus (Message Fabric)
    // ------------------------
    describe_counter!(
        "rhelma_eventbus_publish_total",
        "Total number of eventbus publish calls"
    );

    describe_counter!(
        "rhelma_eventbus_publish_error_total",
        "Number of failed eventbus publish calls"
    );

    describe_histogram!(
        "rhelma_eventbus_publish_duration_seconds",
        "Latency of eventbus publish operations"
    );

    // ------------------------
    // Logger (rhelma-logger integration)
    // ------------------------
    describe_counter!(
        "rhelma_logger_events_total",
        "Total number of log events recorded"
    );

    describe_counter!(
        "rhelma_logger_dropped_total",
        "Number of dropped log events (backpressure)"
    );
}
