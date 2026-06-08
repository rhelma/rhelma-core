//! Common exports for Rhelma Auth.

pub use crate::config::AuthConfig;
/// use (documented for contract compliance).
pub use crate::crypto::password::{hash_password, validate_password_policy, verify_password};
/// use (documented for contract compliance).
pub use crate::db_link::{AuthUserStore, UserRecord};
/// use (documented for contract compliance).
pub use crate::error::{AuthError, AuthResult};
/// use (documented for contract compliance).
pub use crate::eventing::{AuthEventPolicy, AuthEventPublisher, DefaultAuthEventPolicy};
/// use (documented for contract compliance).
pub use crate::jwt::{JwtService, JwtTokenPair};
/// use (documented for contract compliance).
pub use crate::jwt_verify::{JwtVerifier, JwtVerifyConfig};
/// use (documented for contract compliance).
pub use crate::middleware::{
    principal_from_req, require_permission, require_role, AuthLayer, AuthMode, RateLimitConfig,
    RedisRateLimitLayer,
};
/// use (documented for contract compliance).
pub use crate::oidc::{OidcPrincipal, OidcProvider, OidcVerifyInput};
/// use (documented for contract compliance).
pub use crate::rbac::{PolicyEngine, PolicyRule};
/// use (documented for contract compliance).
pub use crate::session::{RedisSessionStore, SessionManager};
/// use (documented for contract compliance).
pub use crate::types::{
    AuthDecision, AuthSubject, JwtClaims, Permission, RefreshRecord, Role, Session, SessionId,
    UserPrincipal,
};

/// use (documented for contract compliance).
pub use rhelma_core::prelude::*;
