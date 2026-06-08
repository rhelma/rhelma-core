//! Rhelma v5.1 Cache Metrics
//!
//! Provides zero-allocation cache metric helpers for Redis, memory cache,
//! and other backends.
//!
//! Naming Convention:
//!   rhelma_cache_hit_total
//!   rhelma_cache_miss_total
//!   rhelma_cache_error_total
//!   rhelma_cache_op_duration_seconds
//!
//! Required labels:
//!   backend   - "redis" | "memory" | ...
//!   operation - "get" | "set" | "delete" | "mget" | ...
//!   key_space - "session" | "rate_limit" | ...
//!
//! Extra labels (via MetricRegistry):
//!   service, environment, region, version, etc.

use metrics::{counter, histogram};
use smallvec::SmallVec;

/// Zero-allocation, v5.1-compliant helper.
///
/// توجه: در این API فرض می‌کنیم backend/operation/key_space
/// مقدارهای canonical و با کاردینالیتی محدود هستند.
pub fn record_cache_hit_with_labels(
    backend: &'static str,
    operation: &'static str,
    key_space: &'static str,
    extra_labels: &[(&'static str, &'static str)],
) {
    debug_assert!(
        3 + extra_labels.len() <= 32,
        "too many labels; violates Rhelma cardinality rules"
    );
    let mut labels: SmallVec<[(&'static str, &'static str); 32]> = SmallVec::new();
    labels.extend_from_slice(&[
        ("backend", backend),
        ("operation", operation),
        ("key_space", key_space),
    ]);
    labels.extend_from_slice(extra_labels);

    counter!("rhelma_cache_hit_total", labels.as_slice()).increment(1);
}

pub fn record_cache_miss_with_labels(
    backend: &'static str,
    operation: &'static str,
    key_space: &'static str,
    extra_labels: &[(&'static str, &'static str)],
) {
    debug_assert!(
        3 + extra_labels.len() <= 32,
        "too many labels; violates Rhelma cardinality rules"
    );
    let mut labels: SmallVec<[(&'static str, &'static str); 32]> = SmallVec::new();
    labels.extend_from_slice(&[
        ("backend", backend),
        ("operation", operation),
        ("key_space", key_space),
    ]);
    labels.extend_from_slice(extra_labels);

    counter!("rhelma_cache_miss_total", labels.as_slice()).increment(1);
}

pub fn record_cache_error_with_labels(
    backend: &'static str,
    operation: &'static str,
    key_space: &'static str,
    extra_labels: &[(&'static str, &'static str)],
) {
    debug_assert!(
        3 + extra_labels.len() <= 32,
        "too many labels; violates Rhelma cardinality rules"
    );
    let mut labels: SmallVec<[(&'static str, &'static str); 32]> = SmallVec::new();
    labels.extend_from_slice(&[
        ("backend", backend),
        ("operation", operation),
        ("key_space", key_space),
    ]);
    labels.extend_from_slice(extra_labels);

    counter!("rhelma_cache_error_total", labels.as_slice()).increment(1);
}

pub fn record_cache_op_duration_with_labels(
    backend: &'static str,
    operation: &'static str,
    key_space: &'static str,
    duration_secs: f64,
    extra_labels: &[(&'static str, &'static str)],
) {
    debug_assert!(
        3 + extra_labels.len() <= 32,
        "too many labels; violates Rhelma cardinality rules"
    );
    let mut labels: SmallVec<[(&'static str, &'static str); 32]> = SmallVec::new();
    labels.extend_from_slice(&[
        ("backend", backend),
        ("operation", operation),
        ("key_space", key_space),
    ]);
    labels.extend_from_slice(extra_labels);

    histogram!("rhelma_cache_op_duration_seconds", labels.as_slice()).record(duration_secs);
}

// ---------------------------------------------------------------------
// Legacy API (backwards compatibility)
//
// ⚠️ این امضاها را هم به &'static str تغییر می‌دهیم تا:
//  - نیازی به Box::leak نداشته باشیم
//  - compile-time تضمین کنیم که labelها canonical و محدودند.
// ---------------------------------------------------------------------

pub fn record_cache_hit(backend: &'static str, operation: &'static str, key_space: &'static str) {
    counter!(
        "rhelma_cache_hit_total",
        "backend" => backend,
        "operation" => operation,
        "key_space" => key_space,
    )
    .increment(1);
}

pub fn record_cache_miss(backend: &'static str, operation: &'static str, key_space: &'static str) {
    counter!(
        "rhelma_cache_miss_total",
        "backend" => backend,
        "operation" => operation,
        "key_space" => key_space,
    )
    .increment(1);
}

pub fn record_cache_error(backend: &'static str, operation: &'static str, key_space: &'static str) {
    counter!(
        "rhelma_cache_error_total",
        "backend" => backend,
        "operation" => operation,
        "key_space" => key_space,
    )
    .increment(1);
}

pub fn record_cache_op_duration(
    backend: &'static str,
    operation: &'static str,
    key_space: &'static str,
    duration_secs: f64,
) {
    histogram!(
        "rhelma_cache_op_duration_seconds",
        "backend" => backend,
        "operation" => operation,
        "key_space" => key_space,
    )
    .record(duration_secs);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_metrics_basic() {
        record_cache_hit("redis", "get", "session");
        record_cache_miss("redis", "get", "session");
        record_cache_error("redis", "get", "session");
        record_cache_op_duration("redis", "get", "session", 0.01);
    }
}
