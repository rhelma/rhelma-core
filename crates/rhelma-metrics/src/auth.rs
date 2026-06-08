//! Auth/security metrics for Rhelma.

use metrics::{counter, histogram};

/// Counter: auth logins.
pub fn record_auth_login(outcome: &'static str, method: &'static str) {
    counter!("rhelma_auth_login_total", "outcome" => outcome, "method" => method).increment(1);
}

/// Counter: token verify.
pub fn record_auth_token_verify(outcome: &'static str) {
    counter!("rhelma_auth_token_verify_total", "outcome" => outcome).increment(1);
}

/// Histogram: session store latency.
pub fn record_auth_session_store(op: &'static str, outcome: &'static str, seconds: f64) {
    histogram!("rhelma_auth_session_store_duration_seconds", "op" => op, "outcome" => outcome)
        .record(seconds);
}

pub fn record_auth_oidc_verify(outcome: &'static str, issuer: &'static str) {
    counter!("rhelma_auth_oidc_verify_total", "outcome" => outcome, "issuer" => issuer)
        .increment(1);
}

pub fn record_auth_event_publish(outcome: &'static str, topic: &'static str) {
    counter!("rhelma_auth_event_publish_total", "outcome" => outcome, "topic" => topic)
        .increment(1);
}
