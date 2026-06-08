//! Metrics hooks for rhelma-auth.
//!
//! IMPORTANT: No direct dependency on `metrics` crate here.
//! Everything goes through `rhelma-metrics`.

/// Record a login attempt.
pub fn record_login(outcome: &'static str, method: &'static str) {
    rhelma_metrics::record_auth_login(outcome, method);
}

/// Record token verify.
pub fn record_token_verify(outcome: &'static str) {
    rhelma_metrics::record_auth_token_verify(outcome);
}

/// Record Redis session store op latency.
pub fn record_session_store(op: &'static str, outcome: &'static str, seconds: f64) {
    rhelma_metrics::record_auth_session_store(op, outcome, seconds);
}

/// Record OIDC verification.
pub fn record_oidc_verify(outcome: &'static str, issuer: &'static str) {
    rhelma_metrics::record_auth_oidc_verify(outcome, issuer);
}

/// Record auth event publish.
pub fn record_auth_event_publish(outcome: &'static str, topic: &'static str) {
    rhelma_metrics::record_auth_event_publish(outcome, topic);
}
