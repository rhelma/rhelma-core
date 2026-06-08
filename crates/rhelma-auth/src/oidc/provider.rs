//! OIDC Provider trait.
//!
//! This is implemented by api-gateway / edge-worker layer which can safely do:
//! - JWKS fetching/caching
//! - token introspection
//! - provider-specific error mapping

use async_trait::async_trait;

use crate::error::AuthResult;
use crate::oidc::types::{OidcPrincipal, OidcVerifyInput};

/// OIDC verification provider.
#[async_trait]
/// trait (documented for contract compliance).
pub trait OidcProvider: Send + Sync {
    /// Verify token and return a normalized principal.
    async fn verify(&self, input: OidcVerifyInput) -> AuthResult<OidcPrincipal>;
}
