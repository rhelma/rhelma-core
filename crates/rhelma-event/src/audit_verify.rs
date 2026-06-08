//! Audit signature verification helpers (Contract v5.2)
//!
//! Goals:
//! - Verify `ops.audit*` envelopes carry a valid ed25519 signature.
//! - Verify signature over the **sha256 hash** of canonical payload, where the payload hash is
//!   computed after removing `signature` and `chain_hash` fields from the payload object (if present).
//!
//! Keyring loading (lazy, cached):
//! - `RHELMA_AUDIT_PUBKEYS` = comma/semicolon-separated entries:
//!     - `key_id:BASE64_PUBLIC_KEY`
//!     - or `BASE64_PUBLIC_KEY` (implicit key_id = "default")
//! - Optional `RHELMA_AUDIT_DEFAULT_KEY_ID` to pick default key when signature omits key_id.
//!
//! Signature formats accepted:
//! - `ed25519:<base64sig>`
//! - `ed25519:<key_id>:<base64sig>`
//!
//! The signature is verified against the 32-byte sha256 digest of canonical payload.
//! For forward/backward resilience, we also try verifying against the lowercase hex digest bytes.

use std::collections::HashMap;
use std::sync::OnceLock;

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use ed25519_dalek::{Signature, VerifyingKey};

use crate::EventEnvelope;

/// Global keyring singleton
static KEYRING: OnceLock<AuditKeyRing> = OnceLock::new();

/// Audit verification errors
#[derive(Debug, thiserror::Error)]
pub enum AuditVerifyError {
    /// Audit keyring is empty
    #[error("audit keyring is empty")]
    /// Variant `EmptyKeyring`.
    EmptyKeyring,

    /// Invalid RHELMA_AUDIT_PUBKEYS entry
    #[error("invalid RHELMA_AUDIT_PUBKEYS entry: {0}")]
    /// Variant `InvalidKeyEntry`.
    InvalidKeyEntry(String),

    /// Invalid public key for key_id
    #[error("invalid public key for key_id={0}")]
    /// Variant `InvalidPublicKey`.
    InvalidPublicKey(String),

    /// Invalid signature format
    #[error("invalid signature format")]
    /// Variant `InvalidSignatureFormat`.
    InvalidSignatureFormat,

    /// Unsupported signature algorithm
    #[error("unsupported signature algorithm: {0}")]
    /// Variant `UnsupportedAlgorithm`.
    UnsupportedAlgorithm(String),

    /// Missing envelope hash
    #[error("missing envelope.hash")]
    /// Variant `MissingHash`.
    MissingHash,

    /// Hash mismatch
    #[error("hash mismatch: expected={expected} computed={computed}")]
    /// Variant `HashMismatch`.
    HashMismatch { expected: String, computed: String },

    /// Unknown key ID
    #[error("unknown key_id={0}")]
    /// Variant `UnknownKeyId`.
    UnknownKeyId(String),

    /// Signature decode failed
    #[error("signature decode failed")]
    /// Variant `SignatureDecodeFailed`.
    SignatureDecodeFailed,

    /// Signature verification failed
    #[error("signature verification failed")]
    /// Variant `SignatureVerificationFailed`.
    SignatureVerificationFailed,
}

/// Audit keyring for signature verification
#[derive(Debug, Clone)]
pub struct AuditKeyRing {
    /// Key ID to verifying key mapping
    keys: HashMap<String, VerifyingKey>,
    /// Default key ID
    default_key_id: Option<String>,
}

impl AuditKeyRing {
    /// Creates keyring from environment variables
    ///
    /// # Returns
    /// `Result<Self, AuditVerifyError>` - Keyring or error
    pub fn from_env() -> Result<Self, AuditVerifyError> {
        let raw = std::env::var("RHELMA_AUDIT_PUBKEYS").unwrap_or_default();
        let default_key_id = std::env::var("RHELMA_AUDIT_DEFAULT_KEY_ID").ok();

        let mut keys = HashMap::new();

        let entries = raw
            .split([',', ';', '\n'])
            .map(str::trim)
            .filter(|s| !s.is_empty());

        for entry in entries {
            let (key_id, b64pk) = match entry.split_once(':') {
                Some((a, b)) if !b.contains(':') => (a.trim().to_string(), b.trim().to_string()),
                _ => ("default".to_string(), entry.trim().to_string()),
            };

            let pk_bytes = B64
                .decode(b64pk.as_bytes())
                .map_err(|_| AuditVerifyError::InvalidKeyEntry(entry.to_string()))?;

            if pk_bytes.len() != 32 {
                return Err(AuditVerifyError::InvalidPublicKey(key_id));
            }

            let vk = VerifyingKey::from_bytes(&pk_bytes.try_into().unwrap())
                .map_err(|_| AuditVerifyError::InvalidPublicKey(key_id.clone()))?;

            keys.insert(key_id, vk);
        }

        if keys.is_empty() {
            return Err(AuditVerifyError::EmptyKeyring);
        }

        Ok(Self {
            keys,
            default_key_id,
        })
    }

    /// Gets default verifying key
    ///
    /// # Returns
    /// `Result<&VerifyingKey, AuditVerifyError>` - Default key or error
    pub fn get_default(&self) -> Result<&VerifyingKey, AuditVerifyError> {
        if let Some(kid) = &self.default_key_id {
            return self
                .keys
                .get(kid)
                .ok_or_else(|| AuditVerifyError::UnknownKeyId(kid.clone()));
        }
        if self.keys.len() == 1 {
            return Ok(self.keys.values().next().unwrap());
        }
        // If multiple keys exist and no default is configured, we refuse ambiguous verification.
        Err(AuditVerifyError::EmptyKeyring)
    }

    /// Gets verifying key by ID
    ///
    /// # Arguments
    /// * `key_id` - Key identifier
    ///
    /// # Returns
    /// `Result<&VerifyingKey, AuditVerifyError>` - Key or error
    pub fn get(&self, key_id: &str) -> Result<&VerifyingKey, AuditVerifyError> {
        self.keys
            .get(key_id)
            .ok_or_else(|| AuditVerifyError::UnknownKeyId(key_id.to_string()))
    }
}

/// Verify an envelope if it is `ops.audit*`.
///
/// If topic is not `ops.audit*`, this is a no-op.
///
/// # Arguments
/// * `env` - Event envelope
///
/// # Returns
/// `Result<(), AuditVerifyError>` - Success or error
pub fn verify_if_audit(env: &EventEnvelope) -> Result<(), AuditVerifyError> {
    if !env.topic.starts_with("ops.audit") {
        return Ok(());
    }
    verify_audit(env)
}

/// Verify a required audit envelope signature.
///
/// # Arguments
/// * `env` - Event envelope
///
/// # Returns
/// `Result<(), AuditVerifyError>` - Success or error
pub fn verify_audit(env: &EventEnvelope) -> Result<(), AuditVerifyError> {
    let ring = KEYRING.get_or_init(|| AuditKeyRing::from_env().expect("valid audit keyring"));

    let expected_hash = env.hash.as_ref().ok_or(AuditVerifyError::MissingHash)?;
    let computed = crate::canonicalization::canonical_audit_payload_hash_hex(&env.payload);

    if !eq_ignore_ascii_case(expected_hash, &computed) {
        return Err(AuditVerifyError::HashMismatch {
            expected: expected_hash.clone(),
            computed,
        });
    }

    let (alg, key_id, sig_bytes) = parse_signature(
        env.signature
            .as_deref()
            .ok_or(AuditVerifyError::InvalidSignatureFormat)?,
    )?;

    if alg != "ed25519" {
        return Err(AuditVerifyError::UnsupportedAlgorithm(alg));
    }

    let vk = match key_id {
        Some(kid) => ring.get(&kid)?,
        None => ring.get_default()?,
    };

    let sig =
        Signature::from_slice(&sig_bytes).map_err(|_| AuditVerifyError::SignatureDecodeFailed)?;

    // Verify signature against raw hash bytes first
    let hash_bytes =
        hex_to_bytes(&computed).ok_or(AuditVerifyError::SignatureVerificationFailed)?;
    if vk.verify_strict(&hash_bytes, &sig).is_ok() {
        return Ok(());
    }

    // Fallback: verify against hex string bytes (compat mode).
    // Disabled by default; enable feature `audit-compat-hex-sig` only for legacy producers.
    #[cfg(feature = "audit-compat-hex-sig")]
    if vk.verify_strict(computed.as_bytes(), &sig).is_ok() {
        return Ok(());
    }

    Err(AuditVerifyError::SignatureVerificationFailed)
}

/// Parses signature string into algorithm, key ID, and signature bytes
///
/// # Arguments
/// * `sig` - Signature string
///
/// # Returns
/// `Result<(String, Option<String>, Vec<u8>), AuditVerifyError>` - Parsed components or error
fn parse_signature(sig: &str) -> Result<(String, Option<String>, Vec<u8>), AuditVerifyError> {
    let parts: Vec<&str> = sig.split(':').collect();
    match parts.as_slice() {
        [alg, b64] => {
            let sig_bytes = B64
                .decode(b64.as_bytes())
                .map_err(|_| AuditVerifyError::SignatureDecodeFailed)?;
            Ok((alg.to_string(), None, sig_bytes))
        }
        [alg, key_id, b64] => {
            let sig_bytes = B64
                .decode(b64.as_bytes())
                .map_err(|_| AuditVerifyError::SignatureDecodeFailed)?;
            Ok((alg.to_string(), Some(key_id.to_string()), sig_bytes))
        }
        _ => Err(AuditVerifyError::InvalidSignatureFormat),
    }
}

/// Converts hex string to bytes
///
/// # Arguments
/// * `hex` - Hex string
///
/// # Returns
/// `Option<Vec<u8>>` - Bytes or None if invalid
fn hex_to_bytes(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(hex.len() / 2);
    let bytes = hex.as_bytes();
    for i in (0..bytes.len()).step_by(2) {
        let hi = from_hex_digit(bytes[i])?;
        let lo = from_hex_digit(bytes[i + 1])?;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

/// Converts hex digit to u8
///
/// # Arguments
/// * `b` - ASCII byte
///
/// # Returns
/// `Option<u8>` - Digit value or None
fn from_hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(10 + (b - b'a')),
        b'A'..=b'F' => Some(10 + (b - b'A')),
        _ => None,
    }
}

/// Case-insensitive ASCII string comparison
///
/// # Arguments
/// * `a` - First string
/// * `b` - Second string
///
/// # Returns
/// `true` if strings are equal ignoring ASCII case
fn eq_ignore_ascii_case(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}
