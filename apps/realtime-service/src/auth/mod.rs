#![forbid(unsafe_code)]

use rhelma_auth::prelude::{AuthConfig, AuthError, UserPrincipal};
use rhelma_auth::AuthService;
use rhelma_core::prelude::{TenantId, UserId};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct WsAuthContext {
    /// Field `user_id`.
    pub user_id: UserId,
    /// Field `tenant_id`.
    pub tenant_id: Option<TenantId>,
    /// Field `session_id`.
    pub session_id: Option<String>,
    /// Field `roles`.
    pub roles: Vec<String>,
    /// Field `permissions`.
    pub permissions: HashSet<String>,
    /// Field `is_anonymous`.
    pub is_anonymous: bool,
}

impl WsAuthContext {
    pub fn has_any_permission(&self, perms: &[&str]) -> bool {
        perms.iter().any(|p| self.permissions.contains(*p))
    }
}

pub async fn build_auth_service(
    service_name: &str,
    environment: &str,
    redis_override: Option<String>,
) -> Result<AuthService, AuthError> {
    let cfg: AuthConfig = AuthConfig::from_env(service_name, environment, redis_override)?;
    AuthService::new(cfg).await
}

pub fn from_principal(p: UserPrincipal) -> WsAuthContext {
    WsAuthContext {
        user_id: p.user_id,
        tenant_id: p.tenant_id,
        session_id: Some(p.session_id.to_string()),
        roles: p.roles.into_iter().map(|r| r.0).collect(),
        permissions: p.permissions.into_iter().map(|perm| perm.0).collect(),
        is_anonymous: false,
    }
}

pub fn anonymous() -> WsAuthContext {
    WsAuthContext {
        user_id: UserId::new(),
        tenant_id: None,
        session_id: None,
        roles: vec![],
        permissions: HashSet::new(),
        is_anonymous: true,
    }
}
