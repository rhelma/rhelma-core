//! Key loading helpers for JWT Ed25519 keys.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use jsonwebtoken::{DecodingKey, EncodingKey};

use crate::error::{AuthError, AuthResult};

/// Load Ed25519 keys from base64 DER blobs (recommended for env/secret manager).
pub fn load_ed25519_keys(
    private_b64: &str,
    public_b64: &str,
) -> AuthResult<(EncodingKey, DecodingKey)> {
    let priv_bytes = STANDARD
        .decode(private_b64.as_bytes())
        .map_err(|_| AuthError::Config {
            code: "invalid_jwt_private_key_b64",
        })?;

    let pub_bytes = STANDARD
        .decode(public_b64.as_bytes())
        .map_err(|_| AuthError::Config {
            code: "invalid_jwt_public_key_b64",
        })?;

    // jsonwebtoken ≥9.3: DecodingKey::from_ed_der expects the raw 32-byte
    // Ed25519 public key, NOT a SubjectPublicKeyInfo (SPKI) DER wrapper.
    // The standard Ed25519 SPKI has a fixed 12-byte header; strip it.
    // EncodingKey::from_ed_der handles PKCS#8 (48 bytes) correctly as-is.
    let raw_pub = if pub_bytes.len() > 32 {
        &pub_bytes[pub_bytes.len() - 32..]
    } else {
        &pub_bytes
    };

    Ok((
        EncodingKey::from_ed_der(&priv_bytes),
        DecodingKey::from_ed_der(raw_pub),
    ))
}

/// Load Ed25519 public key (DecodingKey) from base64 DER blob.
///
/// Useful for services that only need to verify JWTs (no signing).
pub fn load_ed25519_public_key(public_b64: &str) -> AuthResult<DecodingKey> {
    let pub_bytes = STANDARD
        .decode(public_b64.as_bytes())
        .map_err(|_| AuthError::Config {
            code: "invalid_jwt_public_key_b64",
        })?;

    let raw_pub = if pub_bytes.len() > 32 {
        &pub_bytes[pub_bytes.len() - 32..]
    } else {
        &pub_bytes
    };
    Ok(DecodingKey::from_ed_der(raw_pub))
}
