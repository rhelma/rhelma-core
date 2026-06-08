//! Optional DB linking for rhelma-auth.
//!
//! Contract:
//! - rhelma-auth defines traits; rhelma-db or service layer implements them.
//! - No hard coupling to sqlx in this crate.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::AuthResult;
use rhelma_core::prelude::{TenantId, UserId};

/// Minimal user record for auth.
#[derive(Debug, Clone, Serialize, Deserialize)]
/// struct (documented for contract compliance).
pub struct UserRecord {
    /// Field `user_id`.
    pub user_id: UserId,
    #[serde(default)]
    /// Field `tenant_id`.
    pub tenant_id: Option<TenantId>,

    /// Stored password hash (PHC string). Optional for OIDC-only users.
    pub password_phc: Option<String>,

    /// Roles.
    #[serde(default)]
    pub roles: Vec<String>,

    /// Permissions.
    #[serde(default)]
    pub permissions: Vec<String>,

    /// Whether user is disabled.
    #[serde(default)]
    pub disabled: bool,
}

/// DB abstraction for auth flows (service implements this).
#[async_trait]
/// trait (documented for contract compliance).
pub trait AuthUserStore: Send + Sync {
    /// Find user by user_id.
    async fn find_by_user_id(&self, user_id: &UserId) -> AuthResult<Option<UserRecord>>;

    /// Find user by login identifier (email/username). Exact policy is service-defined.
    async fn find_by_login(
        &self,
        tenant_id: Option<&TenantId>,
        login: &str,
    ) -> AuthResult<Option<UserRecord>>;

    /// Find user by external identity.
    async fn find_by_external_subject(
        &self,
        tenant_id: Option<&TenantId>,
        issuer: &str,
        subject: &str,
    ) -> AuthResult<Option<UserRecord>>;

    /// Link external identity to an existing user (or create policy up to service).
    async fn link_external_subject(
        &self,
        user_id: &UserId,
        issuer: &str,
        subject: &str,
    ) -> AuthResult<()>;
}
