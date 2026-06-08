//! Governance-related shared contracts.
//!
//! These are **payload schemas** intended for event transport. They do not
//! implement governance logic; enforcement lives in services.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Topic: governance arbitration request.
pub const TOPIC_GOV_ARBITRATION_REQUEST: &str = "governance.arbitration.request";
/// Topic: governance arbitration result.
pub const TOPIC_GOV_ARBITRATION_RESULT: &str = "governance.arbitration.result";

/// Schema reference: arbitration request v1.
pub const SCHEMA_GOV_ARBITRATION_REQUEST_V1: &str =
    "rhelma://schemas/governance/arbitration_request_v1";
/// Schema reference: arbitration result v1.
pub const SCHEMA_GOV_ARBITRATION_RESULT_V1: &str =
    "rhelma://schemas/governance/arbitration_result_v1";

/// A request to open a governance arbitration case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceArbitrationRequestV1 {
    pub case_id: Uuid,
    pub created_at: DateTime<Utc>,
    /// Human-readable summary.
    pub summary: String,
    /// Optional structured evidence (links, hashes, excerpts).
    pub evidence: Value,
    /// Actor identifier (node id / operator id / council key id).
    pub requested_by: String,
    /// Optional policy bundle/version implicated.
    pub policy_version: Option<String>,
}

/// Arbitration decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceArbitrationResultV1 {
    pub case_id: Uuid,
    pub decided_at: DateTime<Utc>,
    /// Outcome label ("upheld", "rejected", "modified", ...).
    pub outcome: String,
    /// Written rationale.
    pub rationale: String,
    /// Optional structured directives (policy changes, suspensions, rollbacks).
    pub directives: Value,
    /// Signer identity for the decision (jury/council/creator representative).
    pub signed_by: String,
    /// Optional signature material, if transported inside payload.
    pub signature: Option<String>,
}
