//! CSRF helper (baseline): double-submit token.
//!
//! Contract:
//! - For cookie-based sessions, require `x-csrf-token` header to match a cookie `csrf_token`.
//! - Services can generate csrf_token as random and store it in Redis session if they want stricter model.

use http::{header, Request};

use crate::error::{AuthError, AuthResult};

/// Validate CSRF using double-submit cookie pattern.
/// Returns Ok if:
/// - method is safe (GET/HEAD/OPTIONS), OR
/// - header token equals cookie token.
pub fn validate_csrf<B>(req: &Request<B>) -> AuthResult<()> {
    let method = req.method().as_str();
    if method == "GET" || method == "HEAD" || method == "OPTIONS" {
        return Ok(());
    }

    let hdr = req
        .headers()
        .get("x-csrf-token")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or(AuthError::Forbidden)?;

    let cookie = req
        .headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let cookie_token = parse_cookie(cookie, "csrf_token").ok_or(AuthError::Forbidden)?;

    if hdr == cookie_token {
        Ok(())
    } else {
        Err(AuthError::Forbidden)
    }
}

fn parse_cookie(cookie_header: &str, key: &str) -> Option<String> {
    cookie_header.split(';').map(|p| p.trim()).find_map(|kv| {
        let mut it = kv.splitn(2, '=');
        let k = it.next()?.trim();
        let v = it.next()?.trim();
        if k == key {
            Some(v.to_string())
        } else {
            None
        }
    })
}
