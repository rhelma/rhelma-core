//! Password hashing & verification (Argon2).
//!
//! Policy goal (enterprise baseline):
//! - Hash using Argon2id
//! - Store PHC string
//! - Verify constant-time via password-hash crate

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use password_hash::{rand_core::OsRng, SaltString};

use crate::error::{AuthError, AuthResult};

/// Simple password policy checks.
/// Keep it strict but not absurd (services may add stronger policy at higher layer).
pub fn validate_password_policy(password: &str) -> AuthResult<()> {
    let p = password.trim();

    if p.len() < 10 {
        return Err(AuthError::Validation {
            code: "password_too_short",
        });
    }
    if p.len() > 256 {
        return Err(AuthError::Validation {
            code: "password_too_long",
        });
    }
    let has_upper = p.chars().any(|c| c.is_ascii_uppercase());
    let has_lower = p.chars().any(|c| c.is_ascii_lowercase());
    let has_digit = p.chars().any(|c| c.is_ascii_digit());
    let has_symbol = p.chars().any(|c| !c.is_ascii_alphanumeric());

    if !(has_upper && has_lower && has_digit && has_symbol) {
        return Err(AuthError::Validation {
            code: "password_policy_failed",
        });
    }

    Ok(())
}

/// Hash a password and return a PHC string.
pub fn hash_password(password: &str) -> AuthResult<String> {
    validate_password_policy(password)?;

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let phc = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| AuthError::Crypto)?
        .to_string();

    Ok(phc)
}

/// Verify a password against a PHC hash string.
pub fn verify_password(password: &str, phc_hash: &str) -> AuthResult<bool> {
    let parsed = PasswordHash::new(phc_hash).map_err(|_| AuthError::Crypto)?;
    let argon2 = Argon2::default();

    Ok(argon2.verify_password(password.as_bytes(), &parsed).is_ok())
}
