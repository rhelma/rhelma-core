//! Realtime identifiers and connection metadata.
//!
//! Compliant with Rhelma Contract v5.1 - Realtime Presence & Session Model.
//!
//! Strong identifiers, presence status, and connection metadata for
//! WebSocket / SSE / Realtime gateways.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::{RegionId, TenantId, UserId};

/// Strong typed realtime session identifier.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct RealtimeSessionId(pub Uuid);

impl RealtimeSessionId {
    /// Generate a new random session ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for RealtimeSessionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Logical room identifier.
///
/// NOTE: No strict format required by Rhelma v5.1,
/// but we forbid control characters for safety.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct RoomId(pub String);

impl RoomId {
    /// Safe constructor enforcing minimal hygiene.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }
        if s.chars().any(|c| c.is_control()) {
            return None;
        }
        Some(Self(s.to_string()))
    }
}

/// Realtime presence status for a user.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PresenceStatus {
    /// Actively online.
    Online,
    /// Temporarily away.
    Away,
    /// Explicitly offline.
    Offline,
}

/// Metadata for a realtime connection/session.
///
/// This structure is passed internally between gateway nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMetadata {
    /// Session identifier.
    pub session_id: RealtimeSessionId,
    /// Logical user id.
    pub user_id: UserId,
    /// Tenant (optional for shared multi-tenant environments).
    pub tenant_id: Option<TenantId>,
    /// Region (optional edge inference).
    pub region: Option<RegionId>,
    /// When the connection was opened.
    pub connected_at: DateTime<Utc>,
    /// Last observed client activity.
    pub last_seen_at: DateTime<Utc>,
    /// Optional user agent string.
    pub user_agent: Option<String>,
    /// Optional IP address.
    pub ip: Option<String>,
}

impl ConnectionMetadata {
    /// Default idle timeout for realtime connections (seconds).
    /// Rhelma v5.1 default: 300s (5 minutes).
    pub const DEFAULT_IDLE_TIMEOUT_SECS: i64 = 300;

    pub fn is_stale(&self, now: DateTime<Utc>, timeout_secs: i64) -> bool {
        let idle = (now - self.last_seen_at).num_seconds();
        idle > timeout_secs
    }

    /// Convenience wrapper using DEFAULT_IDLE_TIMEOUT_SECS.
    pub fn is_stale_default(&self, now: DateTime<Utc>) -> bool {
        self.is_stale(now, Self::DEFAULT_IDLE_TIMEOUT_SECS)
    }
}
