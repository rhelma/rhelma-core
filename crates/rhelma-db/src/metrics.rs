use std::time::Duration;

#[derive(Clone, Copy, Debug)]
pub enum DbOperation {
    /// Variant `Select`.
    Select,
    /// Variant `Insert`.
    Insert,
    /// Variant `Update`.
    Update,
    /// Variant `Delete`.
    Delete,
    /// Variant `Other`.
    Other(&'static str),
}

impl DbOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            DbOperation::Select => "select",
            DbOperation::Insert => "insert",
            DbOperation::Update => "update",
            DbOperation::Delete => "delete",
            DbOperation::Other(s) => s,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum DbOutcome {
    /// Variant `Success`.
    Success,
    /// Variant `Error`.
    Error,
}

impl DbOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            DbOutcome::Success => "success",
            DbOutcome::Error => "error",
        }
    }
}

/// Rhelma standard DB query metric (via rhelma-metrics public API)
pub fn record(op: DbOperation, outcome: DbOutcome, dur: Duration) {
    rhelma_metrics::record_db_query(dur.as_secs_f64(), op.as_str(), outcome.as_str());
}

pub fn record_conn_error() {
    rhelma_metrics::record_db_connection_error();
}

/// Pool acquire latency (delegated to rhelma-metrics to avoid metrics version conflicts)
pub fn record_pool_acquire(dur: Duration, pool: &'static str, outcome: &'static str) {
    rhelma_metrics::record_db_pool_acquire_duration(dur.as_secs_f64(), pool, outcome);
}

/// Pool gauges (delegated to rhelma-metrics)
pub fn set_pool_gauges(pool: &'static str, size: u32, idle: u32) {
    rhelma_metrics::set_db_pool_size(size as f64, pool);
    rhelma_metrics::set_db_pool_idle(idle as f64, pool);
}
