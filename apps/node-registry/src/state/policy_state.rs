use crate::policy::engine::PolicyEngine;

/// Extend your existing AppState with a `policy` field.
///
/// Example:
/// ```
/// pub struct AppState {
///   pub db: ...,
///   pub policy: PolicyEngine,
/// }
/// ```
pub fn make_policy_from_env() -> PolicyEngine {
    let max_ttl = std::env::var("RHELMA_NODE_REGISTRY__QUARANTINE__MAX_TTL_SECONDS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(604800); // 7 days

    let default_ttl = std::env::var("RHELMA_NODE_REGISTRY__QUARANTINE__DEFAULT_TTL_SECONDS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(86400); // 1 day

    PolicyEngine::new(max_ttl, default_ttl)
}
