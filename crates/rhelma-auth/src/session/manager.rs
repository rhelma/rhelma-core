use base64::Engine as _;
use chrono::Utc;
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::config::AuthConfig;
use crate::error::{AuthError, AuthResult};
use crate::jwt::{JwtService, JwtTokenPair};
use crate::session::store::SessionStore;
use crate::types::{RefreshRecord, Session, SessionId, UserPrincipal};

/// Coordinates JWT + Redis sessions + refresh token rotation.
#[derive(Clone)]
/// struct (documented for contract compliance).
pub struct SessionManager {
    cfg: Arc<AuthConfig>,
    jwt: JwtService,
    store: Arc<dyn SessionStore>,
}

impl SessionManager {
    /// fn (documented for contract compliance).
    pub fn new(cfg: Arc<AuthConfig>, jwt: JwtService, store: Arc<dyn SessionStore>) -> Self {
        Self { cfg, jwt, store }
    }

    /// Issue a new session and return access+refresh tokens.
    pub async fn issue_tokens(&self, principal: &UserPrincipal) -> AuthResult<JwtTokenPair> {
        let (pair, _sid) = self.issue_tokens_with_session(principal).await?;
        Ok(pair)
    }

    /// Issue a new session and return (token_pair, session_id).
    pub async fn issue_tokens_with_session(
        &self,
        principal: &UserPrincipal,
    ) -> AuthResult<(JwtTokenPair, SessionId)> {
        let mut session = Session::new(
            principal.user_id,
            principal.tenant_id.clone(),
            principal.roles.clone(),
            principal.permissions.clone(),
            self.cfg.session_absolute_timeout_secs,
        );

        // principal with new session id
        let p = UserPrincipal {
            user_id: principal.user_id,
            tenant_id: principal.tenant_id.clone(),
            session_id: session.id,
            roles: principal.roles.clone(),
            permissions: principal.permissions.clone(),
        };

        // access token
        let (access, jti, exp) = self.jwt.encode_access(&p)?;
        session.current_jti = Some(jti.clone());

        // store session
        self.store.save_session(&session).await?;

        // map jti -> session (ttl = access ttl)
        self.store
            .bind_jti(&jti, &session.id, self.cfg.access_token_ttl_secs)
            .await?;

        // refresh token (opaque) stored hashed
        let refresh = self.generate_refresh_token();
        let refresh_hash = self.hash_token(&refresh);
        let rec = RefreshRecord {
            user_id: p.user_id,
            tenant_id: p.tenant_id.clone(),
            session_id: p.session_id,
            issued_at: Utc::now(),
        };
        self.store
            .save_refresh(&refresh_hash, &rec, self.cfg.refresh_token_ttl_secs)
            .await?;

        Ok((
            JwtTokenPair {
                access_token: access,
                refresh_token: refresh,
                access_exp: exp,
            },
            session.id,
        ))
    }

    /// Verify access token and ensure an active session exists in Redis.
    pub async fn verify_access_token(&self, token: &str) -> AuthResult<UserPrincipal> {
        let claims = self.jwt.verify(token)?;
        let principal = self.jwt.claims_to_principal(claims.clone())?;

        // must have a live session bound to this jti
        let session = self.store.get_session_by_jti(&claims.jti).await?;
        let mut session = session.ok_or(AuthError::Unauthorized)?;

        if session.revoked || session.is_expired() {
            return Err(AuthError::Unauthorized);
        }

        // idle timeout enforcement
        let idle_deadline = session.last_seen_at
            + chrono::Duration::seconds(self.cfg.session_idle_timeout_secs as i64);
        if Utc::now() >= idle_deadline {
            return Err(AuthError::Unauthorized);
        }

        // Sliding idle timeout: touch `last_seen_at` (best-effort) so active sessions stay active.
        // Writes are throttled by `session_touch_interval_secs` to avoid excessive Redis traffic.
        let now = Utc::now();
        let touch_after = chrono::Duration::seconds(self.cfg.session_touch_interval_secs as i64);
        if now.signed_duration_since(session.last_seen_at) >= touch_after {
            session.last_seen_at = now;
            if let Err(e) = self.store.save_session(&session).await {
                // Fail-open for touch writes: verification should not break user traffic due to a
                // transient write hiccup. Idle enforcement will still work (may log out earlier).
                tracing::warn!(error = %e, "failed to touch auth session last_seen_at");
            }
        }

        Ok(principal)
    }

    /// Rotate refresh token and issue a new access token.
    pub async fn refresh(&self, refresh_token: &str) -> AuthResult<JwtTokenPair> {
        if refresh_token.len() < 32 || refresh_token.len() > 512 {
            return Err(AuthError::InvalidToken);
        }

        let refresh_hash = self.hash_token(refresh_token);
        let rec = self
            .store
            .get_refresh(&refresh_hash)
            .await?
            .ok_or(AuthError::Unauthorized)?;

        // Load session
        let mut session = self
            .store
            .get_session(&rec.session_id)
            .await?
            .ok_or(AuthError::Unauthorized)?;
        if session.revoked || session.is_expired() {
            return Err(AuthError::Unauthorized);
        }

        // Enforce idle timeout
        let idle_deadline = session.last_seen_at
            + chrono::Duration::seconds(self.cfg.session_idle_timeout_secs as i64);
        if Utc::now() >= idle_deadline {
            return Err(AuthError::Unauthorized);
        }

        // Delete old refresh (rotation)
        self.store.delete_refresh(&refresh_hash).await?;

        // Issue new access (keep same session_id)
        let p = UserPrincipal {
            user_id: rec.user_id,
            tenant_id: rec.tenant_id.clone(),
            session_id: rec.session_id,
            roles: session.roles.clone(),
            permissions: session.permissions.clone(),
        };

        let (access, jti, exp) = self.jwt.encode_access(&p)?;

        // Update session last seen + jti
        session.last_seen_at = Utc::now();
        session.current_jti = Some(jti.clone());
        self.store.save_session(&session).await?;
        self.store
            .bind_jti(&jti, &session.id, self.cfg.access_token_ttl_secs)
            .await?;

        // Issue new refresh
        let new_refresh = self.generate_refresh_token();
        let new_hash = self.hash_token(&new_refresh);
        let new_rec = RefreshRecord {
            user_id: rec.user_id,
            tenant_id: rec.tenant_id.clone(),
            session_id: rec.session_id,
            issued_at: Utc::now(),
        };
        self.store
            .save_refresh(&new_hash, &new_rec, self.cfg.refresh_token_ttl_secs)
            .await?;

        Ok(JwtTokenPair {
            access_token: access,
            refresh_token: new_refresh,
            access_exp: exp,
        })
    }

    /// Revoke a session (best-effort).
    pub async fn revoke_session(&self, sid: SessionId) -> AuthResult<()> {
        let mut session = self
            .store
            .get_session(&sid)
            .await?
            .ok_or(AuthError::Unauthorized)?;
        session.revoked = true;

        // best-effort cleanup: delete current jti mapping
        if let Some(jti) = session.current_jti.clone() {
            let _ = self.store.delete_jti(&jti).await;
        }

        self.store.save_session(&session).await?;
        Ok(())
    }

    /// Revoke all sessions for a user (best-effort).
    pub async fn revoke_all_user_sessions(
        &self,
        user_id: &rhelma_core::prelude::UserId,
    ) -> AuthResult<u64> {
        let sids = self.store.list_user_sessions(user_id).await?;
        let mut revoked = 0u64;
        for sid in sids {
            if self.revoke_session(sid).await.is_ok() {
                revoked += 1;
            }
        }
        let _ = self.store.delete_user_session_index(user_id).await;
        Ok(revoked)
    }

    /// Best-effort lookup of refresh record (does not rotate or validate session).
    pub async fn peek_refresh_record(
        &self,
        refresh_token: &str,
    ) -> AuthResult<Option<RefreshRecord>> {
        if refresh_token.len() < 32 || refresh_token.len() > 512 {
            return Ok(None);
        }
        let refresh_hash = self.hash_token(refresh_token);
        self.store.get_refresh(&refresh_hash).await
    }

    fn generate_refresh_token(&self) -> String {
        // 32 random bytes => base64url no-pad => ~43 chars
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    }

    fn hash_token(&self, token: &str) -> String {
        let mut h = Sha256::new();
        h.update(token.as_bytes());
        let out = h.finalize();
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(out)
    }
}
