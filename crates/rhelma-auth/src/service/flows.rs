use chrono::Utc;
use serde_json::json;
use std::sync::Arc;

use crate::config::AuthConfig;
use crate::crypto::password::verify_password;
use crate::db_link::AuthUserStore;
use crate::error::{AuthError, AuthResult};
use crate::jwt::JwtTokenPair;
use crate::oidc::{OidcProvider, OidcVerifyInput};
use crate::session::{RedisSessionStore, SessionManager};
use crate::types::{Permission, Role, SessionId, TenantId, UserPrincipal};

#[cfg(feature = "eventing")]
use crate::eventing::{
    AuthEventPublisher, AuthLoginEvent, AuthLogoutEvent, AuthOidcLoginEvent, AuthRefreshEvent,
    AuthSessionRevokedEvent,
};

#[cfg(feature = "eventing")]
use rhelma_event::EventBus;

/// Password login input.
pub struct LoginPasswordInput {
    /// Field `tenant_id`.
    pub tenant_id: Option<String>,
    /// Field `login`.
    pub login: String, // email/username
    /// Field `password`.
    pub password: String,
}

/// OIDC login input.
pub struct LoginOidcInput {
    /// Field `tenant_id`.
    pub tenant_id: Option<String>,
    /// Field `raw_token`.
    pub raw_token: String,
    /// Field `audience`.
    pub audience: String,
    /// Field `issuer`.
    pub issuer: String,
}

/// High-level auth flows orchestrator.
#[derive(Clone)]
pub struct AuthFlows<U: AuthUserStore, O: OidcProvider> {
    _cfg: Arc<AuthConfig>,
    users: Arc<U>,
    oidc: Arc<O>,
    sessions: SessionManager,

    #[cfg(feature = "eventing")]
    events: AuthEventPublisher,
}

impl<U: AuthUserStore, O: OidcProvider> AuthFlows<U, O> {
    /// Build flows from config + adapters.
    pub async fn new(
        cfg: AuthConfig,
        users: Arc<U>,
        oidc: Arc<O>,
        #[cfg(feature = "eventing")] events: AuthEventPublisher,
    ) -> AuthResult<Self> {
        cfg.validate()?;
        let cfg = Arc::new(cfg);

        let jwt = crate::jwt::JwtService::new(&cfg)?;
        let store = RedisSessionStore::new(&cfg.redis_url, cfg.redis_prefix.clone()).await?;
        let sessions = SessionManager::new(cfg.clone(), jwt, Arc::new(store));

        Ok(Self {
            _cfg: cfg,
            users,
            oidc,
            sessions,
            #[cfg(feature = "eventing")]
            events,
        })
    }

    fn parse_tenant(&self, tenant_id: &Option<String>) -> AuthResult<Option<TenantId>> {
        match tenant_id {
            None => Ok(None),
            Some(s) => {
                let t = TenantId::parse(s).map_err(|_| AuthError::Validation {
                    code: "invalid_tenant_id",
                })?;
                Ok(Some(t))
            }
        }
    }

    /// Password login: find user, verify password, issue session.
    #[cfg(feature = "eventing")]
    pub async fn login_password<B: EventBus + Send + Sync>(
        &self,
        bus: &B,
        input: LoginPasswordInput,
    ) -> AuthResult<JwtTokenPair> {
        let tenant = self.parse_tenant(&input.tenant_id)?;
        let user = self
            .users
            .find_by_login(tenant.as_ref(), input.login.trim())
            .await?
            .ok_or(AuthError::Unauthorized)?;

        if user.disabled {
            return Err(AuthError::Forbidden);
        }

        let phc = user.password_phc.as_ref().ok_or(AuthError::Unauthorized)?;
        if !verify_password(&input.password, phc)? {
            self.events
                .publish_login(
                    bus,
                    AuthLoginEvent {
                        user_id: user.user_id,
                        tenant_id: user.tenant_id.clone(),
                        session_id: SessionId::new(),
                        method: "password".into(),
                        success: false,
                        at: Utc::now(),
                        meta: json!({ "reason": "bad_credentials" }),
                    },
                )
                .await?;
            return Err(AuthError::Unauthorized);
        }

        let principal = UserPrincipal {
            user_id: user.user_id,
            tenant_id: user.tenant_id.clone(),
            session_id: SessionId::new(), // will be replaced by session manager
            roles: user.roles.into_iter().map(Role).collect(),
            permissions: user.permissions.into_iter().map(Permission).collect(),
        };

        let (pair, sid) = self.sessions.issue_tokens_with_session(&principal).await?;

        self.events
            .publish_login(
                bus,
                AuthLoginEvent {
                    user_id: principal.user_id,
                    tenant_id: principal.tenant_id.clone(),
                    session_id: sid,
                    method: "password".into(),
                    success: true,
                    at: Utc::now(),
                    meta: json!({}),
                },
            )
            .await?;

        Ok(pair)
    }

    /// Password login without eventing (feature off).
    #[cfg(not(feature = "eventing"))]
    pub async fn login_password(&self, input: LoginPasswordInput) -> AuthResult<JwtTokenPair> {
        let tenant = self.parse_tenant(&input.tenant_id)?;
        let user = self
            .users
            .find_by_login(tenant.as_ref(), input.login.trim())
            .await?
            .ok_or(AuthError::Unauthorized)?;

        if user.disabled {
            return Err(AuthError::Forbidden);
        }

        let phc = user.password_phc.as_ref().ok_or(AuthError::Unauthorized)?;
        if !verify_password(&input.password, phc)? {
            return Err(AuthError::Unauthorized);
        }

        let principal = UserPrincipal {
            user_id: user.user_id,
            tenant_id: user.tenant_id.clone(),
            session_id: SessionId::new(),
            roles: user.roles.into_iter().map(Role).collect(),
            permissions: user.permissions.into_iter().map(Permission).collect(),
        };

        self.sessions.issue_tokens(&principal).await
    }

    /// OIDC login: verify with provider, map/link user, issue session.
    #[cfg(feature = "eventing")]
    pub async fn login_oidc<B: EventBus + Send + Sync>(
        &self,
        bus: &B,
        input: LoginOidcInput,
    ) -> AuthResult<JwtTokenPair> {
        let tenant = self.parse_tenant(&input.tenant_id)?;

        let oidc_principal = self
            .oidc
            .verify(OidcVerifyInput {
                token: input.raw_token,
                expected_aud: input.audience,
                expected_iss: input.issuer,
                tenant_hint: tenant.clone(),
            })
            .await?;

        // Map external subject to a local user
        let user = self
            .users
            .find_by_external_subject(
                tenant.as_ref(),
                &oidc_principal.issuer,
                &oidc_principal.subject,
            )
            .await?
            .ok_or(AuthError::Unauthorized)?;

        if user.disabled {
            return Err(AuthError::Forbidden);
        }

        let principal = UserPrincipal {
            user_id: user.user_id,
            tenant_id: user.tenant_id.clone(),
            session_id: SessionId::new(),
            roles: user.roles.into_iter().map(Role).collect(),
            permissions: user.permissions.into_iter().map(Permission).collect(),
        };

        let (pair, _sid) = self.sessions.issue_tokens_with_session(&principal).await?;

        self.events
            .publish_oidc_login(
                bus,
                AuthOidcLoginEvent {
                    user_id: Some(principal.user_id),
                    tenant_id: principal.tenant_id.clone(),
                    issuer: oidc_principal.issuer,
                    subject: oidc_principal.subject,
                    success: true,
                    at: Utc::now(),
                    meta: json!({}),
                },
            )
            .await?;

        Ok(pair)
    }

    #[cfg(not(feature = "eventing"))]
    pub async fn login_oidc(&self, input: LoginOidcInput) -> AuthResult<JwtTokenPair> {
        let tenant = self.parse_tenant(&input.tenant_id)?;

        let oidc_principal = self
            .oidc
            .verify(OidcVerifyInput {
                token: input.raw_token,
                expected_aud: input.audience,
                expected_iss: input.issuer,
                tenant_hint: tenant.clone(),
            })
            .await?;

        let user = self
            .users
            .find_by_external_subject(
                tenant.as_ref(),
                &oidc_principal.issuer,
                &oidc_principal.subject,
            )
            .await?
            .ok_or(AuthError::Unauthorized)?;

        if user.disabled {
            return Err(AuthError::Forbidden);
        }

        let principal = UserPrincipal {
            user_id: user.user_id,
            tenant_id: user.tenant_id.clone(),
            session_id: SessionId::new(),
            roles: user.roles.into_iter().map(Role).collect(),
            permissions: user.permissions.into_iter().map(Permission).collect(),
        };

        self.sessions.issue_tokens(&principal).await
    }

    /// Refresh tokens (Redis-enforced rotation).
    #[cfg(feature = "eventing")]
    pub async fn refresh<B: EventBus + Send + Sync>(
        &self,
        bus: &B,
        refresh_token: &str,
    ) -> AuthResult<JwtTokenPair> {
        let rec = self
            .sessions
            .peek_refresh_record(refresh_token)
            .await
            .ok()
            .flatten();
        let res = self.sessions.refresh(refresh_token).await;

        if let Some(rec) = rec {
            let _ = self
                .events
                .publish_refresh(
                    bus,
                    AuthRefreshEvent {
                        user_id: rec.user_id,
                        tenant_id: rec.tenant_id,
                        session_id: rec.session_id,
                        success: res.is_ok(),
                        at: Utc::now(),
                        meta: json!({}),
                    },
                )
                .await;
        }

        res
    }

    #[cfg(not(feature = "eventing"))]
    pub async fn refresh(&self, refresh_token: &str) -> AuthResult<JwtTokenPair> {
        self.sessions.refresh(refresh_token).await
    }

    /// Logout: revoke session.
    #[cfg(feature = "eventing")]
    pub async fn logout<B: EventBus + Send + Sync>(
        &self,
        bus: &B,
        user_id: rhelma_core::prelude::UserId,
        tenant_id: Option<TenantId>,
        session_id: SessionId,
    ) -> AuthResult<()> {
        self.sessions.revoke_session(session_id).await?;
        self.events
            .publish_logout(
                bus,
                AuthLogoutEvent {
                    user_id,
                    tenant_id,
                    session_id,
                    at: Utc::now(),
                    meta: json!({}),
                },
            )
            .await?;
        Ok(())
    }

    #[cfg(not(feature = "eventing"))]
    pub async fn logout(
        &self,
        _user_id: rhelma_core::prelude::UserId,
        _tenant_id: Option<TenantId>,
        session_id: SessionId,
    ) -> AuthResult<()> {
        self.sessions.revoke_session(session_id).await
    }

    /// Revoke session with a reason.
    #[cfg(feature = "eventing")]
    pub async fn revoke_session<B: EventBus + Send + Sync>(
        &self,
        bus: &B,
        user_id: rhelma_core::prelude::UserId,
        tenant_id: Option<TenantId>,
        session_id: SessionId,
        reason: &str,
    ) -> AuthResult<()> {
        self.sessions.revoke_session(session_id).await?;
        self.events
            .publish_session_revoked(
                bus,
                AuthSessionRevokedEvent {
                    user_id,
                    tenant_id,
                    session_id,
                    reason: reason.to_string(),
                    at: Utc::now(),
                    meta: json!({}),
                },
            )
            .await?;
        Ok(())
    }
}
