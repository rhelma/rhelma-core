//! Error model for rhelma-auth (sanitized, Rhelma-aligned).
//!
//! Rules:
//! - Never include secrets / raw backend error messages in public errors.
//! - Convert backend errors to stable, low-cardinality variants.
//! - Provide conversion to `rhelma_core::error::RhelmaError`.

use thiserror::Error;

/// Result type for auth operations.
pub type AuthResult<T> = Result<T, AuthError>;

/// Sanitized auth errors (no secrets, no raw backend error text).
#[derive(Debug, Error, Clone, PartialEq, Eq)]
/// enum (documented for contract compliance).
pub enum AuthError {
    /// Configuration is invalid or missing required fields.
    #[error("auth configuration error: {code}")]
    /// Variant `Config`.
    Config { code: &'static str },

    /// Input validation failed.
    #[error("auth validation error: {code}")]
    /// Variant `Validation`.
    Validation { code: &'static str },

    /// Invalid or malformed token.
    #[error("invalid token")]
    /// Variant `InvalidToken`.
    InvalidToken,

    /// Token is expired.
    #[error("token expired")]
    /// Variant `TokenExpired`.
    TokenExpired,

    /// Permission denied.
    #[error("forbidden")]
    /// Variant `Forbidden`.
    Forbidden,

    /// Authentication required.
    #[error("unauthorized")]
    /// Variant `Unauthorized`.
    Unauthorized,

    /// Redis/session store failed (sanitized).
    #[error("session store unavailable")]
    /// Variant `SessionStore`.
    SessionStore,

    /// Crypto error (hashing/signing/verifying).
    #[error("crypto error")]
    /// Variant `Crypto`.
    Crypto,

    /// Rate limited.
    #[error("rate limited")]
    /// Variant `RateLimited`.
    RateLimited,

    /// Internal error (sanitized).
    #[error("internal error")]
    /// Variant `Internal`.
    Internal,
}

impl From<redis::RedisError> for AuthError {
    fn from(_: redis::RedisError) -> Self {
        // DO NOT leak redis URL, command, etc.
        AuthError::SessionStore
    }
}

impl From<serde_json::Error> for AuthError {
    fn from(_: serde_json::Error) -> Self {
        AuthError::SessionStore
    }
}

impl From<jsonwebtoken::errors::Error> for AuthError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        use jsonwebtoken::errors::ErrorKind;
        match err.kind() {
            ErrorKind::ExpiredSignature => AuthError::TokenExpired,
            _ => AuthError::InvalidToken,
        }
    }
}

impl From<AuthError> for rhelma_core::error::RhelmaError {
    fn from(e: AuthError) -> Self {
        use rhelma_core::error::RhelmaError;
        match e {
            AuthError::Config { code } => RhelmaError::Config(code.to_string()),
            AuthError::Validation { code } => RhelmaError::Validation(code.to_string()),
            AuthError::Unauthorized | AuthError::InvalidToken | AuthError::TokenExpired => {
                RhelmaError::Auth("unauthorized".into())
            }
            AuthError::Forbidden => RhelmaError::Authz("forbidden".into()),
            AuthError::RateLimited => RhelmaError::RateLimited("rate_limited".into()),
            AuthError::SessionStore => RhelmaError::Dependency("session_store_unavailable".into()),
            AuthError::Crypto => RhelmaError::SecurityPolicy("crypto_error".into()),
            AuthError::Internal => RhelmaError::Internal,
        }
    }
}
