#![forbid(unsafe_code)]

//! Admission helpers for node registration.
//!
//! The node-registry can require a small Proof-of-Work (PoW) on registration. Challenges and
//! rate-limiting are stored either in-memory (default) or in Redis when configured.

pub mod pow;
pub mod rate_limit;
pub mod redis_store;

use serde::{Deserialize, Serialize};

/// Record stored for an admission challenge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdmissionChallengeRecord {
    /// Difficulty required for the challenge.
    pub difficulty_bits: u32,
    /// Expiration timestamp (unix seconds).
    pub expires_at_unix: i64,
    /// Optional node id that requested the challenge.
    pub node_id: Option<String>,
}

impl AdmissionChallengeRecord {
    pub fn is_expired(&self, now_unix: i64) -> bool {
        now_unix >= self.expires_at_unix
    }
}
