use rhelma_auth::rbac::{PolicyEngine, PolicyRule};
use rhelma_auth::types::{Permission, Role, SessionId, UserPrincipal};
use rhelma_core::prelude::{TenantId, UserId};

#[test]
fn require_permission_and_role_helpers_work() {
    let principal = UserPrincipal {
        user_id: UserId::new(),
        tenant_id: Some(TenantId::parse("tenant1").unwrap()),
        session_id: SessionId::new(),
        roles: vec![Role("admin".into())],
        permissions: vec![Permission("user.read".into())],
    };

    assert!(PolicyEngine::require_permission(&principal, "user.read").is_ok());
    assert!(PolicyEngine::require_role(&principal, "admin").is_ok());

    let rule = PolicyRule {
        any_roles: vec![Role("admin".into())],
        any_permissions: vec![Permission("user.read".into())],
    };
    assert!(PolicyEngine::require_rule(&principal, &rule).is_ok());
}

#[test]
fn require_rule_denies_when_permission_missing() {
    let principal = UserPrincipal {
        user_id: UserId::new(),
        tenant_id: Some(TenantId::parse("tenant1").unwrap()),
        session_id: SessionId::new(),
        roles: vec![Role("admin".into())],
        permissions: vec![],
    };

    let rule = PolicyRule {
        any_roles: vec![Role("admin".into())],
        any_permissions: vec![Permission("user.read".into())],
    };

    assert!(PolicyEngine::require_rule(&principal, &rule).is_err());
}
