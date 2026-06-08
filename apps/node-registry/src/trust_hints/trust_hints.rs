/// Optional helper: provide routing/trust hints based on checkpoint freshness.
/// This is intentionally a stub to be wired into `discover` in later phases.
#[derive(Debug, Clone)]
pub struct CheckpointTrustHint {
    /// Field `domain`.
    pub domain: String,
    /// Field `min_height`.
    pub min_height: u64,
    /// Field `max_age_sec`.
    pub max_age_sec: u64,
}

pub fn should_prefer_node(_node_id: &str, _hint: &CheckpointTrustHint) -> bool {
    // Wiring point: consult gossip layer / checkpoint head cache.
    true
}
