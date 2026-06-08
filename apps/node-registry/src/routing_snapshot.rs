#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Minimal snapshot types. Replace/extend to match your storage.
///
/// This is a *cache/gossip readiness* artifact:
/// - registries can export a routing state snapshot
/// - federation peers can verify and merge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRoutingState {
    /// Field `node_id`.
    pub node_id: String,
    /// Field `status`.
    pub status: String,          // active|quarantined|banned
    /// Field `routing_weight`.
    pub routing_weight: f64,     // 0.0..1.0
    /// Field `risk_score`.
    pub risk_score: i32,         // 0..100
    /// Field `attested`.
    pub attested: bool,
    /// Field `reputation`.
    pub reputation: i32,         // 0..1000 (example)
    /// Field `dampened_until_unix`.
    pub dampened_until_unix: Option<i64>,
    /// Field `updated_at_unix`.
    pub updated_at_unix: i64,
    /// Field `updated_by`.
    pub updated_by: String,      // ops|guardian|judge|jury|system
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingSnapshot {
    /// Field `snapshot_id`.
    pub snapshot_id: String,
    /// Field `created_at_unix`.
    pub created_at_unix: i64,
    /// Field `merkle_root`.
    pub merkle_root: String,
    /// Field `nodes_count`.
    pub nodes_count: usize,
    /// Field `signature`.
    pub signature: Option<String>,
    /// Field `states`.
    pub states: Vec<NodeRoutingState>,
}