//! Minimal policy engine for Rhelma Auth.
//!
//! Higher layers (api-gateway) can load these rules from config/DB,
//! but rhelma-auth provides stable evaluation primitives.

use crate::error::{AuthError, AuthResult};
use crate::types::{Permission, Role, UserPrincipal};

/// One rule: requires any of roles OR any of permissions.
#[derive(Debug, Clone)]
/// struct (documented for contract compliance).
pub struct PolicyRule {
    /// Allowed roles.
    pub any_roles: Vec<Role>,
    /// Required permissions.
    pub any_permissions: Vec<Permission>,
}

impl PolicyRule {
    /// Allow if any role or any permission matches.
    pub fn evaluate(&self, p: &UserPrincipal) -> bool {
        let role_ok = self.any_roles.is_empty()
            || p.roles
                .iter()
                .any(|r| self.any_roles.iter().any(|x| x == r));

        let perm_ok = self.any_permissions.is_empty()
            || p.permissions
                .iter()
                .any(|q| self.any_permissions.iter().any(|x| x == q));

        role_ok && perm_ok
    }
}

/// Policy engine.
#[derive(Clone, Default)]
/// struct (documented for contract compliance).
pub struct PolicyEngine;

impl PolicyEngine {
    /// Require a permission.
    pub fn require_permission(p: &UserPrincipal, perm: &str) -> AuthResult<()> {
        if p.permissions.iter().any(|x| x.0 == perm) {
            Ok(())
        } else {
            Err(AuthError::Forbidden)
        }
    }

    /// Require a role.
    pub fn require_role(p: &UserPrincipal, role: &str) -> AuthResult<()> {
        if p.roles.iter().any(|x| x.0 == role) {
            Ok(())
        } else {
            Err(AuthError::Forbidden)
        }
    }

    /// Require a custom rule.
    pub fn require_rule(p: &UserPrincipal, rule: &PolicyRule) -> AuthResult<()> {
        if rule.evaluate(p) {
            Ok(())
        } else {
            Err(AuthError::Forbidden)
        }
    }
}
