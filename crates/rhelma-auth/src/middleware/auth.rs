//! Helpers for services to enforce permissions/roles using `UserPrincipal` in extensions.

use http::Request;

use crate::error::{AuthError, AuthResult};
use crate::rbac::PolicyEngine;
use crate::types::UserPrincipal;

/// Get principal from request extensions.
pub fn principal_from_req<B>(req: &Request<B>) -> AuthResult<&UserPrincipal> {
    req.extensions()
        .get::<UserPrincipal>()
        .ok_or(AuthError::Unauthorized)
}

/// Require a permission on request.
pub fn require_permission<B>(req: &Request<B>, perm: &str) -> AuthResult<()> {
    let p = principal_from_req(req)?;
    PolicyEngine::require_permission(p, perm)
}

/// Require a role on request.
pub fn require_role<B>(req: &Request<B>, role: &str) -> AuthResult<()> {
    let p = principal_from_req(req)?;
    PolicyEngine::require_role(p, role)
}
