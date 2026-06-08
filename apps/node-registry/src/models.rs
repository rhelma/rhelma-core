#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEndpointsV1 {
    /// Control-plane endpoint (registration/heartbeat, optional for pure workers).
    pub control_url: Option<String>,
    /// Data-plane endpoint (job execution / inference).
    pub data_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAttestationV1 {
    /// Attestation type (e.g. "none", "software", "tpm", "sgx", "sev-snp")
    pub kind: String,
    /// Opaque evidence blob encoded as base64/hex (policy decides).
    pub evidence: Option<String>,
    /// Optional verifier hint.
    pub verifier: Option<String>,
}

/// Canonical node manifest for Rhelma 6 (Phase 1).
///
/// Notes:
/// - `node_id` is expected to be a **lower-hex Ed25519 public key** (64 chars).
/// - `signature_hex` is optional in Phase 1 (bootstrapped), but the API is shaped so
///   Phase 4 can enforce attestation/signature + challenge workflows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeManifestV1 {
    /// Field `node_id`.
    pub node_id: String,
    /// Field `public_key_hex`.
    pub public_key_hex: String,

    /// Field `display_name`.
    pub display_name: Option<String>,
    /// Field `region`.
    pub region: String,
    /// Field `allowed_residencies`.
    pub allowed_residencies: Vec<String>,
    /// Field `capabilities`.
    pub capabilities: Vec<String>,
    /// Field `endpoints`.
    pub endpoints: NodeEndpointsV1,
    /// Field `version`.
    pub version: String,

    /// Field `attestation`.
    pub attestation: Option<NodeAttestationV1>,
    /// Field `signature_hex`.
    pub signature_hex: Option<String>,

    /// Field `issued_at`.
    pub issued_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRegisterRequestV1 {
    /// Field `manifest`.
    pub manifest: NodeManifestV1,
    #[serde(default)]
    /// Field `admission`.
    pub admission: Option<AdmissionProofV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRegisterResponseV1 {
    /// Field `ok`.
    pub ok: bool,
    /// Field `node_id`.
    pub node_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHeartbeatRequestV1 {
    /// Field `node_id`.
    pub node_id: String,
    /// Field `observed_at`.
    pub observed_at: DateTime<Utc>,
    /// Field `load_avg_1m`.
    pub load_avg_1m: Option<f64>,
    /// Field `free_mem_mb`.
    pub free_mem_mb: Option<u64>,
    /// Field `notes`.
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHeartbeatResponseV1 {
    /// Field `ok`.
    pub ok: bool,
    /// Field `node_id`.
    pub node_id: String,
    /// Field `next_heartbeat_seconds`.
    pub next_heartbeat_seconds: u64,
}

/// Node attestation submission (Phase 4).
///
/// The signature is computed over the canonical JSON bytes of this object with
/// `signature_hex = None`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAttestRequestV1 {
    /// Field `node_id`.
    pub node_id: String,
    /// Field `attestation`.
    pub attestation: NodeAttestationV1,
    /// Field `issued_at`.
    pub issued_at: DateTime<Utc>,
    /// Field `signature_hex`.
    pub signature_hex: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAttestResponseV1 {
    /// Field `ok`.
    pub ok: bool,
    /// Field `node_id`.
    pub node_id: String,
    /// Field `attested`.
    pub attested: bool,
}

/// Outcome reporting (Phase 4) - intended for trusted reporters (scheduler/orchestrator).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeReportOutcomeV1 {
    /// Variant `Ok`.
    Ok,
    /// Variant `Fail`.
    Fail,
    /// Variant `Timeout`.
    Timeout,
    /// Variant `BadResult`.
    BadResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeReportRequestV1 {
    /// Field `node_id`.
    pub node_id: String,
    /// Field `outcome`.
    pub outcome: NodeReportOutcomeV1,
    /// Field `duration_ms`.
    pub duration_ms: Option<u64>,
    /// Field `issued_at`.
    pub issued_at: DateTime<Utc>,
    /// Field `notes`.
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeReportResponseV1 {
    /// Field `ok`.
    pub ok: bool,
    /// Field `node_id`.
    pub node_id: String,
    /// Field `reputation`.
    pub reputation: i32,
    /// Field `status`.
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSummaryV1 {
    /// Field `node_id`.
    pub node_id: String,
    /// Field `region`.
    pub region: String,
    /// Field `allowed_residencies`.
    pub allowed_residencies: Vec<String>,
    /// Field `capabilities`.
    pub capabilities: Vec<String>,
    /// Field `endpoints`.
    pub endpoints: NodeEndpointsV1,
    /// Field `last_seen_at`.
    pub last_seen_at: DateTime<Utc>,

    /// Reputation score (Phase 4). Starts at 0, can go negative.
    pub reputation: i32,

    /// Whether the registry considers this node attested for the current policy.
    pub attested: bool,

    /// Scheduling status: active | probation | suspended
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverResponseV1 {
    /// Field `nodes`.
    pub nodes: Vec<NodeSummaryV1>,
}

/// Proof-of-work admission proof (Phase 4). Fields are hex-encoded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdmissionProofV1 {
    /// Field `nonce_hex`.
    pub nonce_hex: String,
    /// Field `solution_hex`.
    pub solution_hex: String,
    /// Field `difficulty_bits`.
    pub difficulty_bits: u32,
}

/// Server-provided admission challenge (Phase 4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdmissionChallengeResponseV1 {
    /// Field `nonce_hex`.
    pub nonce_hex: String,
    /// Field `difficulty_bits`.
    pub difficulty_bits: u32,
    /// Field `expires_unix`.
    pub expires_unix: i64,
}

/// Durable record for a registered federation node.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg(feature = "federation-sync")]
pub struct FederationNodeRecordV1 {
    /// Field `node_id`.
    pub node_id: String,
    /// Field `manifest`.
    pub manifest: NodeManifestV1,
    /// Field `region`.
    pub region: String,
    /// Field `allowed_residencies`.
    pub allowed_residencies: Vec<String>,
    /// Field `capabilities`.
    pub capabilities: Vec<String>,
    /// Field `endpoints`.
    pub endpoints: NodeEndpointsV1,
    /// Field `registered_at`.
    pub registered_at: DateTime<Utc>,
    /// Field `last_heartbeat_at`.
    pub last_heartbeat_at: DateTime<Utc>,
    /// Field `reputation`.
    pub reputation: i32,
    /// Field `status`.
    pub status: String,
    /// Field `attested`.
    pub attested: bool,
}
