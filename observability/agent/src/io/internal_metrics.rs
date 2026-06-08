//! internal_metrics.rs — internal counters for Observability-Agent

use std::sync::atomic::{AtomicU64, Ordering};

/// Counter for total insights published
pub static INSIGHT_SENT: AtomicU64 = AtomicU64::new(0);
/// Counter for total alerts published
pub static ALERT_SENT: AtomicU64 = AtomicU64::new(0);
/// Counter for total incidents proposed
pub static INCIDENT_SENT: AtomicU64 = AtomicU64::new(0);

/// Counter for total heartbeats sent
pub static HEARTBEAT_SENT: AtomicU64 = AtomicU64::new(0);
/// Counter for total heartbeat publish failures
pub static HEARTBEAT_FAILURE: AtomicU64 = AtomicU64::new(0);

/// Counter for total audit events published
pub static AUDIT_OK: AtomicU64 = AtomicU64::new(0);
/// Counter for total audit failures
pub static AUDIT_FAILED: AtomicU64 = AtomicU64::new(0);

/// Counter for total commands executed successfully
pub static COMMAND_SUCCESS: AtomicU64 = AtomicU64::new(0);
/// Counter for total command execution failures
pub static COMMAND_FAILURE: AtomicU64 = AtomicU64::new(0);
/// Counter for total commands denied by allow-list
pub static COMMAND_DENIED: AtomicU64 = AtomicU64::new(0);

/// Counter for total times degraded mode entered
pub static DEGRADE_COUNT: AtomicU64 = AtomicU64::new(0);

/// Counter for total obs.signal received
pub static SIGNAL_RECEIVED: AtomicU64 = AtomicU64::new(0);
/// Counter for total obs.signal dropped (backpressure)
pub static SIGNAL_DROPPED: AtomicU64 = AtomicU64::new(0);
/// Counter for total kafka subscriber retry attempts
pub static KAFKA_RETRY: AtomicU64 = AtomicU64::new(0);
/// Counter for total agent shutdown signals observed
pub static SHUTDOWN_COUNT: AtomicU64 = AtomicU64::new(0);

/// Increments the insight sent counter
pub fn insight_sent() {
    INSIGHT_SENT.fetch_add(1, Ordering::Relaxed);
}

/// Increments the alert sent counter
pub fn alert_sent() {
    ALERT_SENT.fetch_add(1, Ordering::Relaxed);
}

/// Increments the incident sent counter
pub fn incident_sent() {
    INCIDENT_SENT.fetch_add(1, Ordering::Relaxed);
}

/// Increments the heartbeat sent counter
pub fn heartbeat_sent() {
    HEARTBEAT_SENT.fetch_add(1, Ordering::Relaxed);
}

/// Increments the heartbeat failure counter
pub fn heartbeat_failure() {
    HEARTBEAT_FAILURE.fetch_add(1, Ordering::Relaxed);
}

/// Increments the audit success counter
pub fn audit_ok() {
    AUDIT_OK.fetch_add(1, Ordering::Relaxed);
}

/// Increments the audit failure counter
pub fn audit_failed() {
    AUDIT_FAILED.fetch_add(1, Ordering::Relaxed);
}

/// Increments the agent command success counter
pub fn agent_command_success() {
    COMMAND_SUCCESS.fetch_add(1, Ordering::Relaxed);
}

/// Increments the agent command failure counter
pub fn agent_command_failure() {
    COMMAND_FAILURE.fetch_add(1, Ordering::Relaxed);
}

/// Increments the agent command denied counter
pub fn agent_command_denied() {
    COMMAND_DENIED.fetch_add(1, Ordering::Relaxed);
}

/// Increments the agent degraded mode counter
pub fn agent_degraded() {
    DEGRADE_COUNT.fetch_add(1, Ordering::Relaxed);
}

/// Increments the signal received counter
pub fn signal_received() {
    SIGNAL_RECEIVED.fetch_add(1, Ordering::Relaxed);
}

/// Increments the signal dropped counter
pub fn signal_dropped() {
    SIGNAL_DROPPED.fetch_add(1, Ordering::Relaxed);
}

/// Increments the Kafka retry counter
pub fn kafka_retry() {
    KAFKA_RETRY.fetch_add(1, Ordering::Relaxed);
}

/// Increments the agent shutdown counter
pub fn agent_shutdown() {
    SHUTDOWN_COUNT.fetch_add(1, Ordering::Relaxed);
}

/// Export internal counters in Prometheus text exposition format.
///
/// NOTE: This does NOT start an HTTP server; callers can expose this string
/// via any existing /metrics endpoint.
///
/// # Returns
/// String containing Prometheus metrics in text exposition format
pub fn export_prometheus() -> String {
    // Keep output stable for scrapers.
    let mut out = String::new();

    macro_rules! counter {
        ($name:expr, $help:expr, $v:expr) => {{
            out.push_str("# HELP ");
            out.push_str($name);
            out.push(' ');
            out.push_str($help);
            out.push('\n');
            out.push_str("# TYPE ");
            out.push_str($name);
            out.push_str(" counter\n");
            out.push_str($name);
            out.push(' ');
            out.push_str(&$v.load(Ordering::Relaxed).to_string());
            out.push('\n');
        }};
    }

    counter!(
        "rhelma_obs_insight_sent_total",
        "Total obs.insight published",
        &INSIGHT_SENT
    );
    counter!(
        "rhelma_obs_alert_sent_total",
        "Total obs.alert published",
        &ALERT_SENT
    );
    counter!(
        "rhelma_obs_incident_sent_total",
        "Total incidents proposed",
        &INCIDENT_SENT
    );

    counter!(
        "rhelma_obs_heartbeat_sent_total",
        "Total heartbeats sent",
        &HEARTBEAT_SENT
    );
    counter!(
        "rhelma_obs_heartbeat_failure_total",
        "Total heartbeat publish failures",
        &HEARTBEAT_FAILURE
    );

    counter!(
        "rhelma_obs_audit_ok_total",
        "Total audit events published",
        &AUDIT_OK
    );
    counter!(
        "rhelma_obs_audit_failed_total",
        "Total audit failures",
        &AUDIT_FAILED
    );

    counter!(
        "rhelma_obs_command_success_total",
        "Total commands executed successfully",
        &COMMAND_SUCCESS
    );
    counter!(
        "rhelma_obs_command_failure_total",
        "Total command execution failures",
        &COMMAND_FAILURE
    );
    counter!(
        "rhelma_obs_command_denied_total",
        "Total commands denied by allow-list",
        &COMMAND_DENIED
    );

    counter!(
        "rhelma_obs_degraded_total",
        "Total times degraded mode entered",
        &DEGRADE_COUNT
    );

    counter!(
        "rhelma_obs_signal_received_total",
        "Total obs.signal received",
        &SIGNAL_RECEIVED
    );
    counter!(
        "rhelma_obs_signal_dropped_total",
        "Total obs.signal dropped (backpressure)",
        &SIGNAL_DROPPED
    );
    counter!(
        "rhelma_obs_kafka_retry_total",
        "Total kafka subscriber retry attempts",
        &KAFKA_RETRY
    );
    counter!(
        "rhelma_obs_shutdown_total",
        "Total agent shutdown signals observed",
        &SHUTDOWN_COUNT
    );

    out
}
