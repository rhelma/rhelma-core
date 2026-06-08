//! Lightweight JWT verifier (Ed25519 / EdDSA) for Rhelma services that only need verification.
//!
//! Unlike [`crate::jwt::JwtService`], this verifier does not require private keys.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use jsonwebtoken::{decode_header, Algorithm, DecodingKey, Validation};

use crate::crypto::keys::load_ed25519_public_key;
use crate::error::{AuthError, AuthResult};
use crate::metrics;
use crate::tracing_ext::auth_span;
use crate::types::JwtClaims;

/// Public-only JWT verification config.
#[derive(Debug, Clone)]
/// struct (documented for contract compliance).
pub struct JwtVerifyConfig {
    /// Expected issuer.
    pub issuer: String,
    /// Expected audience.
    pub audience: String,
    /// Ed25519 public key (base64 DER).
    pub public_key_b64: String,
}

impl JwtVerifyConfig {
    /// fn (documented for contract compliance).
    pub fn validate(&self) -> AuthResult<()> {
        if self.issuer.trim().is_empty() {
            return Err(AuthError::Config {
                code: "jwt_issuer_empty",
            });
        }
        if self.audience.trim().is_empty() {
            return Err(AuthError::Config {
                code: "jwt_audience_empty",
            });
        }
        if self.public_key_b64.trim().is_empty() {
            return Err(AuthError::Config {
                code: "jwt_public_key_b64_empty",
            });
        }
        Ok(())
    }
}

/// Verify-only JWT service (EdDSA).
#[derive(Clone)]
/// struct (documented for contract compliance).
pub struct JwtVerifier {
    decoding: DecodingKey,
    issuer: String,
    audience: String,
}

impl JwtVerifier {
    /// Build verifier from a public key and expected issuer/audience.
    pub fn new(cfg: JwtVerifyConfig) -> AuthResult<Self> {
        cfg.validate()?;
        let decoding = load_ed25519_public_key(cfg.public_key_b64.trim())?;

        Ok(Self {
            decoding,
            issuer: cfg.issuer.trim().to_string(),
            audience: cfg.audience.trim().to_string(),
        })
    }

    /// Verify JWT and return claims.
    pub fn verify(&self, token: &str) -> AuthResult<JwtClaims> {
        let _span = auth_span("jwt.verify_only");

        let mut v = Validation::new(Algorithm::EdDSA);
        v.set_audience(std::slice::from_ref(&self.audience));
        v.set_issuer(std::slice::from_ref(&self.issuer));
        v.validate_exp = true;

        let data = jsonwebtoken::decode::<JwtClaims>(token, &self.decoding, &v)?;
        metrics::record_token_verify("ok");
        Ok(data.claims)
    }
}

/// Public key entry for a JWT keyring.
#[derive(Debug, Clone)]
/// struct (documented for contract compliance).
pub struct JwtKeyEntry {
    /// Key id (kid) used in JWT headers.
    pub kid: String,
    /// Ed25519 public key (base64 DER / SPKI).
    pub public_key_b64: String,
}

/// Public-only JWT verification config for a keyring.
///
/// This supports key rotation by selecting the decoding key based on the JWT header `kid`.
#[derive(Debug, Clone)]
/// struct (documented for contract compliance).
pub struct JwtVerifyKeyringConfig {
    /// Expected issuer.
    pub issuer: String,
    /// Expected audience.
    pub audience: String,
    /// Key set (must not be empty).
    pub keys: Vec<JwtKeyEntry>,
    /// If true, accept JWTs without a `kid` header and verify with the fallback key.
    pub allow_legacy_no_kid: bool,
    /// Fallback key id (used when `kid` is missing).
    pub fallback_kid: Option<String>,
}

impl JwtVerifyKeyringConfig {
    /// Validate config consistency.
    pub fn validate(&self) -> AuthResult<()> {
        if self.issuer.trim().is_empty() {
            return Err(AuthError::Config {
                code: "jwt_issuer_empty",
            });
        }
        if self.audience.trim().is_empty() {
            return Err(AuthError::Config {
                code: "jwt_audience_empty",
            });
        }
        if self.keys.is_empty() {
            return Err(AuthError::Config {
                code: "jwt_keyring_empty",
            });
        }

        for k in &self.keys {
            if k.kid.trim().is_empty() {
                return Err(AuthError::Config {
                    code: "jwt_kid_empty",
                });
            }
            if k.public_key_b64.trim().is_empty() {
                return Err(AuthError::Config {
                    code: "jwt_public_key_b64_empty",
                });
            }
        }
        Ok(())
    }
}

/// Verify-only JWT verifier that supports multiple rotating public keys (EdDSA / Ed25519).
///
/// Intended for Rhelma services that want JWT verification without private keys and with
/// rotation support (e.g. when keys are served via JWKS).
#[derive(Clone)]
/// struct (documented for contract compliance).
pub struct JwtVerifierKeyring {
    issuer: String,
    audience: String,
    allow_legacy_no_kid: bool,
    keys: Arc<RwLock<HashMap<String, Arc<DecodingKey>>>>,
    fallback_kid: Arc<RwLock<Option<String>>>,
}

impl JwtVerifierKeyring {
    /// Build verifier from a keyring config.
    pub fn new(cfg: JwtVerifyKeyringConfig) -> AuthResult<Self> {
        cfg.validate()?;

        let map = build_key_map(&cfg.keys)?;
        let fallback = cfg.fallback_kid.clone().and_then(|k| {
            let kk = k.trim().to_string();
            if kk.is_empty() {
                None
            } else {
                Some(kk)
            }
        });

        Ok(Self {
            issuer: cfg.issuer.trim().to_string(),
            audience: cfg.audience.trim().to_string(),
            allow_legacy_no_kid: cfg.allow_legacy_no_kid,
            keys: Arc::new(RwLock::new(map)),
            fallback_kid: Arc::new(RwLock::new(fallback)),
        })
    }

    /// Replace the key set at runtime (best-effort key rotation).
    ///
    /// This is safe to call concurrently with `verify()`; requests will either use the old
    /// or the new key set.
    pub fn replace_keys(
        &self,
        keys: Vec<JwtKeyEntry>,
        fallback_kid: Option<String>,
    ) -> AuthResult<()> {
        if keys.is_empty() {
            return Err(AuthError::Config {
                code: "jwt_keyring_empty",
            });
        }
        let map = build_key_map(&keys)?;
        let mut g = self.keys.write().map_err(|_| AuthError::Internal)?;
        *g = map;

        let mut fb = self.fallback_kid.write().map_err(|_| AuthError::Internal)?;
        *fb = fallback_kid.and_then(|k| {
            let kk = k.trim().to_string();
            if kk.is_empty() {
                None
            } else {
                Some(kk)
            }
        });
        Ok(())
    }

    /// Verify JWT and return claims.
    pub fn verify(&self, token: &str) -> AuthResult<JwtClaims> {
        let _span = auth_span("jwt.verify_keyring");

        let header = decode_header(token)?;
        let kid = header.kid;

        let key = {
            let g = self.keys.read().map_err(|_| AuthError::Internal)?;
            let fb = self.fallback_kid.read().map_err(|_| AuthError::Internal)?;
            select_key(&g, kid.as_deref(), self.allow_legacy_no_kid, fb.as_deref())
                .ok_or(AuthError::InvalidToken)?
                .clone()
        };

        let mut v = Validation::new(Algorithm::EdDSA);
        v.set_audience(std::slice::from_ref(&self.audience));
        v.set_issuer(std::slice::from_ref(&self.issuer));
        v.validate_exp = true;

        let data = jsonwebtoken::decode::<JwtClaims>(token, &key, &v)?;
        metrics::record_token_verify("ok");
        Ok(data.claims)
    }
}

fn build_key_map(keys: &[JwtKeyEntry]) -> AuthResult<HashMap<String, Arc<DecodingKey>>> {
    let mut out: HashMap<String, Arc<DecodingKey>> = HashMap::new();
    for k in keys {
        let kid = k.kid.trim().to_string();
        let pk = k.public_key_b64.trim().to_string();

        if kid.is_empty() || pk.is_empty() {
            return Err(AuthError::Config {
                code: "jwt_keyring_entry_invalid",
            });
        }
        let decoding = load_ed25519_public_key(&pk)?;
        out.insert(kid, Arc::new(decoding));
    }
    Ok(out)
}

fn select_key<'a>(
    keys: &'a HashMap<String, Arc<DecodingKey>>,
    kid: Option<&str>,
    allow_legacy_no_kid: bool,
    fallback_kid: Option<&str>,
) -> Option<&'a Arc<DecodingKey>> {
    if let Some(kid) = kid.map(|s| s.trim()).filter(|s| !s.is_empty()) {
        return keys.get(kid);
    }

    if allow_legacy_no_kid {
        if let Some(fk) = fallback_kid.map(|s| s.trim()).filter(|s| !s.is_empty()) {
            if let Some(k) = keys.get(fk) {
                return Some(k);
            }
        }
        if keys.len() == 1 {
            return keys.values().next();
        }
    }

    None
}
