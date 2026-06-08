//! Session helpers for services that want explicit session checks.
//!
//! Note: AuthLayer already validates session existence + revoked/expired.
//! This module is for cases where service wants manual session checks.

use crate::error::{AuthError, AuthResult};
use crate::session::store::SessionStore;
use crate::session::RedisSessionStore;
use crate::types::{Session, SessionId};

/// Load session and ensure it is active.
pub async fn require_active_session(
    store: &RedisSessionStore,
    sid: &SessionId,
) -> AuthResult<Session> {
    let s = store
        .get_session(sid)
        .await?
        .ok_or(AuthError::Unauthorized)?;
    if s.revoked || s.is_expired() {
        Err(AuthError::Unauthorized)
    } else {
        Ok(s)
    }
}
