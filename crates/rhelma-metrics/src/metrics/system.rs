use metrics::{counter, gauge};

/// Rhelma v5.1 System Metrics
///
/// This component exposes stable, low-cardinality metrics describing:
/// - CPU usage
/// - Memory usage
/// - Active connections
/// - Disk I/O
/// - Network I/O
///
/// All labels MUST be `'static` and canonical.
/// All values are raw, non-derived, and Prometheus-compatible.
///
/// Metric naming follows:
///   rhelma_system_<subsystem>_<metric>
///   - counters end with `_total`
///   - gauges expose instantaneous values
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    service_name: &'static str,
    environment: &'static str,
    region: &'static str,
}

impl SystemMetrics {
    /// Create instance — all arguments must be `'static`.
    ///
    /// MetricRegistry is responsible for converting user config
    /// into `'static` using Box::leak exactly once.
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

    // =========================================================================
    // ACTIVE CONNECTIONS
    // =========================================================================

    /// Set absolute connection count.
    pub fn set_active_connections(&self, count: u64) {
        gauge!(
            "rhelma_system_active_connections",
            "service" => self.service_name,
            "environment" => self.environment,
            "region" => self.region,
        )
        .set(count as f64);
    }

    /// Increment active connections by 1.
    pub fn increment_connections(&self) {
        gauge!(
            "rhelma_system_active_connections",
            "service" => self.service_name,
            "environment" => self.environment,
            "region" => self.region,
        )
        .increment(1.0);
    }

    /// Decrement active connections by 1.
    pub fn decrement_connections(&self) {
        gauge!(
            "rhelma_system_active_connections",
            "service" => self.service_name,
            "environment" => self.environment,
            "region" => self.region,
        )
        .decrement(1.0);
    }

    // =========================================================================
    // MEMORY
    // =========================================================================

    /// Report memory usage in bytes (RSS or process memory).
    pub fn set_memory_usage_bytes(&self, bytes: u64) {
        gauge!(
            "rhelma_system_memory_usage_bytes",
            "service" => self.service_name,
            "environment" => self.environment,
            "region" => self.region,
        )
        .set(bytes as f64);
    }

    // =========================================================================
    // CPU
    // =========================================================================

    /// CPU usage ratio in [0.0, 1.0]
    pub fn set_cpu_usage_ratio(&self, ratio: f64) {
        let r = ratio.clamp(0.0, 1.0);

        gauge!(
            "rhelma_system_cpu_usage_ratio",
            "service" => self.service_name,
            "environment" => self.environment,
            "region" => self.region,
        )
        .set(r);
    }

    /// CPU usage percent in [0.0, 100.0]
    pub fn set_cpu_usage_percent(&self, percent: f64) {
        self.set_cpu_usage_ratio(percent / 100.0);
    }

    // =========================================================================
    // DISK I/O
    // =========================================================================

    pub fn add_disk_read_bytes(&self, bytes: u64) {
        counter!(
            "rhelma_system_disk_read_bytes_total",
            "service" => self.service_name,
            "environment" => self.environment,
            "region" => self.region,
        )
        .increment(bytes);
    }

    pub fn add_disk_write_bytes(&self, bytes: u64) {
        counter!(
            "rhelma_system_disk_write_bytes_total",
            "service" => self.service_name,
            "environment" => self.environment,
            "region" => self.region,
        )
        .increment(bytes);
    }

    // =========================================================================
    // NETWORK I/O
    // =========================================================================

    pub fn add_network_received_bytes(&self, bytes: u64) {
        counter!(
            "rhelma_system_network_received_bytes_total",
            "service" => self.service_name,
            "environment" => self.environment,
            "region" => self.region,
        )
        .increment(bytes);
    }

    pub fn add_network_sent_bytes(&self, bytes: u64) {
        counter!(
            "rhelma_system_network_sent_bytes_total",
            "service" => self.service_name,
            "environment" => self.environment,
            "region" => self.region,
        )
        .increment(bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::SystemMetrics;

    #[test]
    fn system_metrics_basic() {
        let m = SystemMetrics::new("svc", "dev", Some("eu-west-1"));

        m.set_active_connections(10);
        m.increment_connections();
        m.decrement_connections();

        m.set_memory_usage_bytes(1024 * 1024);
        m.set_cpu_usage_percent(50.0);
        m.set_cpu_usage_ratio(0.75);

        m.add_disk_read_bytes(4096);
        m.add_disk_write_bytes(2048);

        m.add_network_received_bytes(8192);
        m.add_network_sent_bytes(4096);
    }
}
