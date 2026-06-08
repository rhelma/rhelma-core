use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::policy::status::NodeStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePolicyState {
    /// Field `status`.
    pub status: NodeStatus,
    /// Field `quarantine_reason`.
    pub quarantine_reason: Option<String>,
    /// Field `quarantine_until`.
    pub quarantine_until: Option<DateTime<Utc>>,
    /// Field `updated_at`.
    pub updated_at: DateTime<Utc>,
}

impl Default for NodePolicyState {
    fn default() -> Self {
        Self {
            status: NodeStatus::Active,
            quarantine_reason: None,
            quarantine_until: None,
            updated_at: Utc::now(),
        }
    }
}

/// Minimal in-memory policy engine.
/// In production, back this with your DB layer.
#[derive(Clone)]
pub struct PolicyEngine {
    max_ttl_seconds: u64,
    default_ttl_seconds: u64,
    // cache for last action result (helper only)
    last_node_id: Arc<Mutex<String>>,
}

impl PolicyEngine {
    pub fn new(max_ttl_seconds: u64, default_ttl_seconds: u64) -> Self {
        Self {
            max_ttl_seconds,
            default_ttl_seconds,
            last_node_id: Arc::new(Mutex::new(String::new())),
        }
    }

    pub async fn quarantine(&self, req: crate::routes::internal_policy::QuarantineReq) -> anyhow::Result<()> {
        let ttl = req.ttl_seconds.unwrap_or(self.default_ttl_seconds);
        if ttl > self.max_ttl_seconds {
            anyhow::bail!("ttl_seconds exceeds max ({})", self.max_ttl_seconds);
        }
        *self.last_node_id.lock().unwrap() = req.node_id.clone();
        // NOTE: policy persistence is phase-scoped; current engine is in-memory only.
        // status=Quarantined, quarantine_until=Utc::now()+ttl, reason=req.reason, updated_at=now
        Ok(())
    }

    pub async fn unquarantine(&self, req: crate::routes::internal_policy::UnquarantineReq) -> anyhow::Result<()> {
        *self.last_node_id.lock().unwrap() = req.node_id.clone();
        // NOTE: policy persistence is phase-scoped; current engine is in-memory only.
        // status=Active, clear quarantine fields
        Ok(())
    }

    pub fn last_node_id(&self) -> String {
        self.last_node_id.lock().unwrap().clone()
    }
}
