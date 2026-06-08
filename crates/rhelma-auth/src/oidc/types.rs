//! OIDC types.

use serde::{Deserialize, Serialize};

use rhelma_core::prelude::{TenantId, UserId};

/// Input to verify an OIDC ID token (already extracted from request).
#[derive(Debug, Clone)]
/// struct (documented for contract compliance).
pub struct OidcVerifyInput {
    /// The raw ID token / access token (depending on IdP design).
    pub token: String,

    /// Expected audience (client id).
    pub expected_aud: String,

    /// Expected issuer.
    pub expected_iss: String,

    /// Optional tenant hint (multi-tenant deployments).
    pub tenant_hint: Option<TenantId>,
}

/// Verified principal produced by OIDC verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
/// struct (documented for contract compliance).
pub struct OidcPrincipal {
    /// Issuer (IdP).
    pub issuer: String,

    /// Stable subject from IdP.
    pub subject: String,

    /// Mapped user id (optional; your integration may map via DB).
    pub user_id: Option<UserId>,

    /// Email (if available).
    pub email: Option<String>,

    /// Tenant (if known).
    pub tenant_id: Option<TenantId>,

    /// Roles from IdP (optional).
    #[serde(default)]
    pub roles: Vec<String>,
}
