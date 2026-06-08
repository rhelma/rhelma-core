use metrics::{counter, gauge, histogram};
use smallvec::SmallVec;

/// Rhelma v5.1-compliant DB metrics (zero allocation).
///
/// Low-level API used by MetricRegistry.
pub fn record_db_query_with_labels(
    duration_seconds: f64,
    operation: &'static str,
    outcome: &'static str,
    extra_labels: &[(&'static str, &'static str)],
) {
    debug_assert!(
        2 + extra_labels.len() <= 32,
        "too many labels; violates Rhelma cardinality rules"
    );
    let mut labels: SmallVec<[(&'static str, &'static str); 32]> = SmallVec::new();
    labels.extend_from_slice(&[("operation", operation), ("outcome", outcome)]);
    labels.extend_from_slice(extra_labels);

    histogram!("rhelma_db_query_duration_seconds", labels.as_slice()).record(duration_seconds);
    counter!("rhelma_db_query_total", labels.as_slice()).increment(1);
}

/// Legacy API (fallback for older code),
/// Now SAFE: no Box::leak, but operation/outcome must be static.
pub fn record_db_query(duration_seconds: f64, operation: &'static str, outcome: &'static str) {
    record_db_query_with_labels(duration_seconds, operation, outcome, &[]);
}

/// Rhelma v5.1-compliant DB connection error metric.
pub fn record_db_connection_error_with_labels(extra_labels: &[(&'static str, &'static str)]) {
    counter!("rhelma_db_connection_errors_total", extra_labels).increment(1);
}

/// Legacy fallback.
/// No leaks; no dynamic labels.
pub fn record_db_connection_error() {
    counter!("rhelma_db_connection_errors_total").increment(1);
}

pub fn record_db_pool_acquire_duration(
    duration_seconds: f64,
    pool: &'static str,
    outcome: &'static str,
) {
    let labels = [("pool", pool), ("outcome", outcome)];
    histogram!("rhelma_db_pool_acquire_duration_seconds", &labels).record(duration_seconds);
}

pub fn set_db_pool_size(size: f64, pool: &'static str) {
    let labels = [("pool", pool)];
    gauge!("rhelma_db_pool_size", &labels).set(size);
}

pub fn set_db_pool_idle(idle: f64, pool: &'static str) {
    let labels = [("pool", pool)];
    gauge!("rhelma_db_pool_idle", &labels).set(idle);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_metrics_basic() {
        record_db_query(0.05, "select", "success");
        record_db_query_with_labels(0.10, "insert", "error", &[("shard", "primary")]);
        record_db_connection_error();
        record_db_connection_error_with_labels(&[("pool", "main")]);
    }
}
