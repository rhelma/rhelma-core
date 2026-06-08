//! Typed payloads for auth events.
//!
//! No secrets (no tokens, no password hashes).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{SessionId, TenantId, UserId};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// struct (documented for contract compliance).
pub struct AuthLoginEvent {
    /// Field `user_id`.
    pub user_id: UserId,
    #[serde(default)]
    /// Field `tenant_id`.
    pub tenant_id: Option<TenantId>,
    /// Field `session_id`.
    pub session_id: SessionId,
    /// Field `method`.
    pub method: String, // "password" | "oidc" | ...
    /// Field `success`.
    pub success: bool,
    /// Field `at`.
    pub at: DateTime<Utc>,
    #[serde(default)]
    /// Field `meta`.
    pub meta: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// struct (documented for contract compliance).
pub struct AuthOidcLoginEvent {
    /// Field `user_id`.
    pub user_id: Option<UserId>,
    #[serde(default)]
    /// Field `tenant_id`.
    pub tenant_id: Option<TenantId>,
    /// Field `issuer`.
    pub issuer: String,
    /// Field `subject`.
    pub subject: String,
    /// Field `success`.
    pub success: bool,
    /// Field `at`.
    pub at: DateTime<Utc>,
    #[serde(default)]
    /// Field `meta`.
    pub meta: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// struct (documented for contract compliance).
pub struct AuthLogoutEvent {
    /// Field `user_id`.
    pub user_id: UserId,
    #[serde(default)]
    /// Field `tenant_id`.
    pub tenant_id: Option<TenantId>,
    /// Field `session_id`.
    pub session_id: SessionId,
    /// Field `at`.
    pub at: DateTime<Utc>,
    #[serde(default)]
    /// Field `meta`.
    pub meta: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// struct (documented for contract compliance).
pub struct AuthRefreshEvent {
    /// Field `user_id`.
    pub user_id: UserId,
    #[serde(default)]
    /// Field `tenant_id`.
    pub tenant_id: Option<TenantId>,
    /// Field `session_id`.
    pub session_id: SessionId,
    /// Field `success`.
    pub success: bool,
    /// Field `at`.
    pub at: DateTime<Utc>,
    #[serde(default)]
    /// Field `meta`.
    pub meta: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// struct (documented for contract compliance).
pub struct AuthSessionRevokedEvent {
    /// Field `user_id`.
    pub user_id: UserId,
    #[serde(default)]
    /// Field `tenant_id`.
    pub tenant_id: Option<TenantId>,
    /// Field `session_id`.
    pub session_id: SessionId,
    /// Field `reason`.
    pub reason: String,
    /// Field `at`.
    pub at: DateTime<Utc>,
    #[serde(default)]
    /// Field `meta`.
    pub meta: Value,
}
