use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    /// Variant `Active`.
    Active,
    /// Variant `Quarantined`.
    Quarantined,
    /// Variant `Banned`.
    Banned,
}

impl Default for NodeStatus {
    fn default() -> Self {
        NodeStatus::Active
    }
}
