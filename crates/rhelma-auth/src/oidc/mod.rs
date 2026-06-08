//! OIDC integration.
//!
//! Contract choice:
//! - rhelma-auth SHOULD NOT fetch JWKS / do HTTP by itself.
//! - Higher layers (api-gateway/edge-worker) implement `OidcProvider`
//!   (or call external IdP introspection) and pass verified identity here.

pub mod provider;
/// mod (documented for contract compliance).
pub mod types;

/// use (documented for contract compliance).
pub use provider::OidcProvider;
/// use (documented for contract compliance).
pub use types::{OidcPrincipal, OidcVerifyInput};
