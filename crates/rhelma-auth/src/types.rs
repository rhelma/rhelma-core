//! Core types for Rhelma Auth.
//!
//! Notes:
//! - We reuse strong IDs from rhelma-core where possible.
//! - SessionId is local to rhelma-auth (UUID).
//! - No secrets are stored in these structs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// use (documented for contract compliance).
pub use rhelma_core::prelude::{TenantId, UserId};

use crate::error::{AuthError, AuthResult};

/// Session id (UUID).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
/// struct (documented for contract compliance).
pub struct SessionId(pub Uuid);

impl SessionId {
    /// Create a new random session id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parse from external string.
    pub fn parse(s: &str) -> AuthResult<Self> {
        let uuid = Uuid::parse_str(s).map_err(|_| AuthError::Validation {
            code: "invalid_session_id",
        })?;
        Ok(Self(uuid))
    }

    /// fn (documented for contract compliance).
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Role name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
/// struct (documented for contract compliance).
pub struct Role(pub String);

/// Permission name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
/// struct (documented for contract compliance).
pub struct Permission(pub String);

/// Who is the subject of auth.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// enum (documented for contract compliance).
pub enum AuthSubject {
    /// A human user.
    User,
    /// A system/agent component.
    System,
}

/// Policy decision outcome.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// enum (documented for contract compliance).
pub enum AuthDecision {
    /// Allow.
    Allow,
    /// Deny.
    Deny,
}

/// JWT claims for Rhelma access tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
/// struct (documented for contract compliance).
pub struct JwtClaims {
    /// Subject (user id).
    pub sub: UserId,

    /// Tenant (optional).
    #[serde(default)]
    pub tenant_id: Option<TenantId>,

    /// Session id.
    pub session_id: SessionId,

    /// Issued-at (unix).
    pub iat: i64,

    /// Expiration (unix).
    pub exp: i64,

    /// JWT id (for revocation/mapping).
    pub jti: String,

    /// Subject type.
    pub subject: AuthSubject,

    /// Issuer.
    pub iss: String,

    /// Audience.
    pub aud: String,

    /// Roles (low-cardinality).
    #[serde(default)]
    pub roles: Vec<Role>,

    /// Permissions (low-cardinality).
    #[serde(default)]
    pub permissions: Vec<Permission>,
}

/// A verified principal (inserted into request extensions by middleware).
#[derive(Debug, Clone, Serialize, Deserialize)]
/// struct (documented for contract compliance).
pub struct UserPrincipal {
    /// Field `user_id`.
    pub user_id: UserId,
    #[serde(default)]
    /// Field `tenant_id`.
    pub tenant_id: Option<TenantId>,
    /// Field `session_id`.
    pub session_id: SessionId,
    #[serde(default)]
    /// Field `roles`.
    pub roles: Vec<Role>,
    #[serde(default)]
    /// Field `permissions`.
    pub permissions: Vec<Permission>,
}

/// Session model stored in Redis.
///
/// We store roles/permissions so refresh rotation can re-issue equivalent access tokens
/// without a DB round-trip (higher layers may still override this).
#[derive(Debug, Clone, Serialize, Deserialize)]
/// struct (documented for contract compliance).
pub struct Session {
    /// Session id.
    pub id: SessionId,
    /// User id.
    pub user_id: UserId,
    /// Tenant id.
    #[serde(default)]
    pub tenant_id: Option<TenantId>,

    /// Roles snapshot.
    #[serde(default)]
    pub roles: Vec<Role>,
    /// Permissions snapshot.
    #[serde(default)]
    pub permissions: Vec<Permission>,

    /// Created time.
    pub created_at: DateTime<Utc>,
    /// Last seen (idle timeout is based on this).
    pub last_seen_at: DateTime<Utc>,
    /// Absolute expiration time.
    pub expires_at: DateTime<Utc>,

    /// Last issued access token jti (best-effort for cleanup).
    #[serde(default)]
    pub current_jti: Option<String>,

    /// Whether session is revoked.
    #[serde(default)]
    pub revoked: bool,
}

impl Session {
    /// Create a new session with absolute timeout.
    pub fn new(
        user_id: UserId,
        tenant_id: Option<TenantId>,
        roles: Vec<Role>,
        permissions: Vec<Permission>,
        absolute_ttl_secs: u64,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::new(),
            user_id,
            tenant_id,
            roles,
            permissions,
            created_at: now,
            last_seen_at: now,
            expires_at: now + chrono::Duration::seconds(absolute_ttl_secs as i64),
            current_jti: None,
            revoked: false,
        }
    }

    /// fn (documented for contract compliance).
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }
}

/// Refresh token record stored in Redis (hashed token is the key).
#[derive(Debug, Clone, Serialize, Deserialize)]
/// struct (documented for contract compliance).
pub struct RefreshRecord {
    /// Field `user_id`.
    pub user_id: UserId,
    #[serde(default)]
    /// Field `tenant_id`.
    pub tenant_id: Option<TenantId>,
    /// Field `session_id`.
    pub session_id: SessionId,
    /// Field `issued_at`.
    pub issued_at: DateTime<Utc>,
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}
