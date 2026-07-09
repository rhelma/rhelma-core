//! Rhelma Auth — enterprise-grade authentication and authorization for Rhelma v5.2.
//!
//! Responsibilities:
//! - JWT access tokens (EdDSA/Ed25519)
//! - Redis-backed sessions + refresh token rotation
//! - RBAC policy evaluation primitives
//! - Tower middleware for extraction/verification
//! - Optional eventing hooks (rhelma-event)

#![forbid(unsafe_code)]

/// mod (documented for contract compliance).
pub mod config;
/// mod (documented for contract compliance).
pub mod crypto;
/// mod (documented for contract compliance).
pub mod db_link;
/// mod (documented for contract compliance).
pub mod error;
/// mod (documented for contract compliance).
pub mod eventing;
/// Internal service-to-service authentication (HMAC-signed headers).
pub mod internal_service;
/// mod (documented for contract compliance).
pub mod jwt;
/// mod (documented for contract compliance).
pub mod jwt_verify;
/// mod (documented for contract compliance).
pub mod metrics;
/// mod (documented for contract compliance).
pub mod middleware;
/// mod (documented for contract compliance).
pub mod oidc;
/// mod (documented for contract compliance).
pub mod prelude;
/// mod (documented for contract compliance).
pub mod rbac;
/// mod (documented for contract compliance).
pub mod service;
/// mod (documented for contract compliance).
pub mod session;
/// mod (documented for contract compliance).
pub mod tracing_ext;
/// mod (documented for contract compliance).
pub mod types;

use std::sync::Arc;

/// use (documented for contract compliance).
pub use config::AuthConfig;
/// use (documented for contract compliance).
pub use error::{AuthError, AuthResult};
/// use (documented for contract compliance).
pub use internal_service::{
    InternalAuthError, InternalRequestSigner, InternalRequestVerifier, ServiceIdentity,
    SignedHeaders, VerifiedCaller,
};
/// use (documented for contract compliance).
pub use jwt::{JwtService, JwtTokenPair};
/// use (documented for contract compliance).
pub use jwt_verify::{
    JwtKeyEntry, JwtVerifier, JwtVerifierKeyring, JwtVerifyConfig, JwtVerifyKeyringConfig,
};
/// use (documented for contract compliance).
pub use session::{RedisSessionStore, SessionManager};
/// use (documented for contract compliance).
pub use types::{Session, SessionId, UserPrincipal};

/// High-level convenience wrapper (optional).
#[derive(Clone)]
/// struct (documented for contract compliance).
pub struct AuthService {
    cfg: Arc<AuthConfig>,
    jwt: JwtService,
    sessions: SessionManager,
}

impl AuthService {
    /// Construct AuthService (validates config and initializes Redis + JWT).
    pub async fn new(cfg: AuthConfig) -> AuthResult<Self> {
        cfg.validate()?;
        let cfg = Arc::new(cfg);

        let jwt = JwtService::new(&cfg)?;
        let store = RedisSessionStore::new(&cfg.redis_url, cfg.redis_prefix.clone()).await?;
        let sessions = SessionManager::new(cfg.clone(), jwt.clone(), Arc::new(store));

        Ok(Self { cfg, jwt, sessions })
    }

    /// Verify access token and return principal (checks session state in Redis).
    pub async fn verify_access_token(&self, token: &str) -> AuthResult<UserPrincipal> {
        self.sessions.verify_access_token(token).await
    }

    /// Issue new access+refresh token pair for a user.
    pub async fn issue_for_principal(&self, principal: &UserPrincipal) -> AuthResult<JwtTokenPair> {
        self.sessions.issue_tokens(principal).await
    }

    /// Rotate refresh token and return new pair.
    pub async fn refresh(&self, refresh_token: &str) -> AuthResult<JwtTokenPair> {
        self.sessions.refresh(refresh_token).await
    }

    /// Revoke a session.
    pub async fn revoke_session(&self, sid: SessionId) -> AuthResult<()> {
        self.sessions.revoke_session(sid).await
    }

    /// Revoke all sessions for a user.
    pub async fn revoke_all_user_sessions(
        &self,
        user_id: rhelma_core::prelude::UserId,
    ) -> AuthResult<u64> {
        self.sessions.revoke_all_user_sessions(&user_id).await
    }

    /// Expose config (read-only).
    pub fn config(&self) -> &AuthConfig {
        &self.cfg
    }

    /// Expose jwt (read-only).
    pub fn jwt(&self) -> &JwtService {
        &self.jwt
    }
}
