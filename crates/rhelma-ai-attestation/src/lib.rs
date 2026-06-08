//! rhelma-ai-attestation — lightweight cryptographic attestation helpers for Rhelma AI workflows.
//!
//! v1 design goals:
//! - Provide a small, dependency-light way to *bind* an evaluation result to a specific patch + test plan.
//! - Enable downstream workers (e.g., patch-applier) to verify results came from a trusted evaluator.
//! - Keep the primitive simple (HMAC-SHA256) so it works in constrained environments.
//!
//! NOTE: HS256 is a shared-secret scheme. For stronger non-repudiation, upgrade to Ed25519
//! with per-service keys and explicit key rotation.

#![forbid(unsafe_code)]

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use thiserror::Error;

/// Supported attestation algorithms.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttestationAlg {
    /// HMAC-SHA256 (shared secret).
    Hs256,
}

/// Attestation payload (v1).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttestationV1 {
    /// Field `alg`.
    pub alg: AttestationAlg,
    /// Key id (for future rotation). Optional but recommended.
    pub kid: Option<String>,
    /// SHA-256 hex of the canonical payload that was signed.
    pub payload_sha256_hex: String,
    /// Signature bytes encoded as base64url (no padding).
    pub signature_b64: String,
    /// Timestamp of signing.
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum AttestationError {
    #[error("attestation missing")]
    /// Variant `Missing`.
    Missing,
    #[error("unsupported alg")]
    /// Variant `UnsupportedAlg`.
    UnsupportedAlg,
    #[error("invalid signature")]
    /// Variant `InvalidSignature`.
    InvalidSignature,
    #[error("payload hash mismatch")]
    /// Variant `PayloadHashMismatch`.
    PayloadHashMismatch,
    #[error("serialization error: {0}")]
    /// Variant `Serialization`.
    Serialization(String),
    #[error("key not found")]
    /// Variant `KeyNotFound`.
    KeyNotFound,
}

/// A single HS256 key entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hs256Key {
    /// Field `kid`.
    pub kid: Option<String>,
    /// Field `secret`.
    pub secret: Vec<u8>,
}

/// A simple key ring for HS256 attestations.
///
/// This enables basic key rotation:
/// - evaluators sign using the **primary** key
/// - verifiers select key by `kid` (or try all keys if `kid` is missing)
#[derive(Debug, Clone, Default)]
pub struct Hs256KeyRing {
    /// Field `keys`.
    pub keys: Vec<Hs256Key>,
    /// Field `primary_kid`.
    pub primary_kid: Option<String>,
}

impl Hs256KeyRing {
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn from_single(secret: &[u8], kid: Option<String>) -> Self {
        Self {
            keys: vec![Hs256Key {
                kid,
                secret: secret.to_vec(),
            }],
            primary_kid: None,
        }
    }

    pub fn primary(&self) -> Option<&Hs256Key> {
        if let Some(pk) = self.primary_kid.as_ref() {
            if let Some(k) = self.keys.iter().find(|k| k.kid.as_ref() == Some(pk)) {
                return Some(k);
            }
        }
        self.keys.first()
    }
}

/// Compute SHA256 hex (lowercase) for bytes.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let out = hasher.finalize();
    hex::encode(out)
}

/// Build canonical JSON bytes for signing.
///
/// `serde_json::Map` is ordered (BTreeMap) by default, yielding stable key ordering.
pub fn canonical_json_bytes(value: &serde_json::Value) -> Result<Vec<u8>, AttestationError> {
    serde_json::to_vec(value).map_err(|e| AttestationError::Serialization(e.to_string()))
}

/// Sign canonical JSON payload using HS256.
pub fn sign_hs256(
    payload: &serde_json::Value,
    secret: &[u8],
    kid: Option<String>,
) -> Result<AttestationV1, AttestationError> {
    let bytes = canonical_json_bytes(payload)?;
    let payload_hash = sha256_hex(&bytes);

    let mut mac: Hmac<Sha256> =
        Hmac::new_from_slice(secret).map_err(|_| AttestationError::UnsupportedAlg)?;
    mac.update(payload_hash.as_bytes());
    let sig = mac.finalize().into_bytes();

    Ok(AttestationV1 {
        alg: AttestationAlg::Hs256,
        kid,
        payload_sha256_hex: payload_hash,
        signature_b64: URL_SAFE_NO_PAD.encode(sig),
        created_at: Utc::now(),
    })
}

/// Sign canonical JSON payload using the *primary* key from a key ring.
pub fn sign_hs256_with_keyring(
    payload: &serde_json::Value,
    keyring: &Hs256KeyRing,
) -> Result<AttestationV1, AttestationError> {
    let k = keyring.primary().ok_or(AttestationError::KeyNotFound)?;
    sign_hs256(payload, &k.secret, k.kid.clone())
}

/// Verify HS256 attestation against canonical JSON payload.
pub fn verify_hs256(
    payload: &serde_json::Value,
    att: &AttestationV1,
    secret: &[u8],
) -> Result<(), AttestationError> {
    if att.alg != AttestationAlg::Hs256 {
        return Err(AttestationError::UnsupportedAlg);
    }

    let bytes = canonical_json_bytes(payload)?;
    let payload_hash = sha256_hex(&bytes);

    if payload_hash
        .as_bytes()
        .ct_eq(att.payload_sha256_hex.as_bytes())
        .unwrap_u8()
        != 1
    {
        return Err(AttestationError::PayloadHashMismatch);
    }

    let mut mac: Hmac<Sha256> =
        Hmac::new_from_slice(secret).map_err(|_| AttestationError::UnsupportedAlg)?;
    mac.update(att.payload_sha256_hex.as_bytes());
    let expected = mac.finalize().into_bytes();

    let sig = URL_SAFE_NO_PAD
        .decode(att.signature_b64.as_bytes())
        .map_err(|_| AttestationError::InvalidSignature)?;

    if expected.as_slice().ct_eq(sig.as_slice()).unwrap_u8() != 1 {
        return Err(AttestationError::InvalidSignature);
    }

    Ok(())
}

/// Verify HS256 attestation using a key ring.
///
/// Selection rules:
/// - if `att.kid` is set: try the matching key
/// - otherwise: try all keys (useful for legacy attestations)
pub fn verify_hs256_with_keyring(
    payload: &serde_json::Value,
    att: &AttestationV1,
    keyring: &Hs256KeyRing,
) -> Result<(), AttestationError> {
    if keyring.keys.is_empty() {
        return Err(AttestationError::KeyNotFound);
    }

    if let Some(kid) = att.kid.as_ref() {
        let k = keyring
            .keys
            .iter()
            .find(|k| k.kid.as_ref() == Some(kid))
            .ok_or(AttestationError::KeyNotFound)?;
        return verify_hs256(payload, att, &k.secret);
    }

    for k in &keyring.keys {
        if verify_hs256(payload, att, &k.secret).is_ok() {
            return Ok(());
        }
    }

    Err(AttestationError::InvalidSignature)
}

/// Parse a comma-separated HS256 key list in the form: `kid1:secret1,kid2:secret2`.
///
/// Notes:
/// - entries without a `:` are treated as a *kidless* key (legacy).
/// - secrets are treated as UTF-8 bytes.
pub fn parse_hs256_keys(s: &str) -> Vec<Hs256Key> {
    s.split(',')
        .map(|e| e.trim())
        .filter(|e| !e.is_empty())
        .map(|e| {
            if let Some((kid, sec)) = e.split_once(':') {
                Hs256Key {
                    kid: Some(kid.trim().to_string()).filter(|x| !x.is_empty()),
                    secret: sec.as_bytes().to_vec(),
                }
            } else {
                Hs256Key {
                    kid: None,
                    secret: e.as_bytes().to_vec(),
                }
            }
        })
        .filter(|k| !k.secret.is_empty())
        .collect()
}

/// Load HS256 key ring from env.
///
/// Supported vars:
/// - `RHELMA_AI_ATTESTATION__HMAC_KEYS` (preferred): `kid1:secret1,kid2:secret2`
/// - `RHELMA_AI_ATTESTATION__PRIMARY_KID` (optional)
/// - legacy: `RHELMA_AI_ATTESTATION__HMAC_SECRET` (+ optional `RHELMA_AI_ATTESTATION__KID`)
pub fn load_hs256_keyring_from_env() -> Hs256KeyRing {
    let keys_raw = std::env::var("RHELMA_AI_ATTESTATION__HMAC_KEYS").ok();
    let primary_kid = std::env::var("RHELMA_AI_ATTESTATION__PRIMARY_KID")
        .ok()
        .and_then(|v| Some(v.trim().to_string()).filter(|s| !s.is_empty()));

    if let Some(raw) = keys_raw {
        let mut kr = Hs256KeyRing {
            keys: parse_hs256_keys(&raw),
            primary_kid,
        };
        // If no primary_kid is set but exactly one key has a kid, treat it as primary.
        if kr.primary_kid.is_none() {
            let mut kids: Vec<String> = kr.keys.iter().filter_map(|k| k.kid.clone()).collect();
            kids.sort();
            kids.dedup();
            if kids.len() == 1 {
                kr.primary_kid = Some(kids[0].clone());
            }
        }
        return kr;
    }

    // Legacy single-secret.
    let secret = std::env::var("RHELMA_AI_ATTESTATION__HMAC_SECRET")
        .ok()
        .and_then(|v| Some(v).filter(|s| !s.trim().is_empty()));
    let kid = std::env::var("RHELMA_AI_ATTESTATION__KID")
        .ok()
        .and_then(|v| Some(v.trim().to_string()).filter(|s| !s.is_empty()));

    match secret {
        Some(sec) => Hs256KeyRing {
            keys: vec![Hs256Key {
                kid,
                secret: sec.as_bytes().to_vec(),
            }],
            primary_kid: None,
        },
        None => Hs256KeyRing::default(),
    }
}

/// Convenience: require an attestation and verify it.
pub fn require_and_verify_hs256(
    payload: &serde_json::Value,
    att: Option<&AttestationV1>,
    secret: &[u8],
) -> Result<(), AttestationError> {
    let att = att.ok_or(AttestationError::Missing)?;
    verify_hs256(payload, att, secret)
}

// `hex` is used in sha256_hex.
mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        let b = bytes.as_ref();
        let mut s = String::with_capacity(b.len() * 2);
        for &v in b {
            s.push_str(&format!("{:02x}", v));
        }
        s
    }
}
