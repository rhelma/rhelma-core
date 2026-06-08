#![forbid(unsafe_code)]

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use chrono::Utc;
use ed25519_dalek::Signer;
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use thiserror::Error;

use crate::{purpose, EventEnvelope, EventSource, PolicyMeta};

/// Errors for audit signature processing.
#[derive(Debug, Error)]
pub enum AuditSigError {
    #[error("missing signature")]
    /// Variant `MissingSignature`.
    MissingSignature,

    /// Missing payload hash for an audit event.
    #[error("missing payload hash")]
    /// Variant `MissingHash`.
    MissingHash,

    /// Provided hash did not match computed hash.
    #[error("hash mismatch (expected={expected}, computed={computed})")]
    /// Variant `HashMismatch`.
    HashMismatch { expected: String, computed: String },
    #[error("unsupported signature format")]
    /// Variant `BadFormat`.
    BadFormat,
    #[error("base64 decode failed")]
    /// Variant `BadBase64`.
    BadBase64,
    #[error("signature bytes length invalid")]
    /// Variant `BadSignatureLen`.
    BadSignatureLen,
    #[error("unknown key id '{0}'")]
    /// Variant `UnknownKeyId`.
    UnknownKeyId(String),
    #[error("verification failed")]
    /// Variant `VerifyFailed`.
    VerifyFailed,
    #[error("missing RHELMA_AUDIT_PUBKEYS env")]
    /// Variant `MissingPubKeys`.
    MissingPubKeys,
    #[error("invalid public key encoding")]
    /// Variant `BadPublicKey`.
    BadPublicKey,
    #[error("missing RHELMA_AUDIT_SIGNING_KEY env")]
    /// Variant `MissingSigningKey`.
    MissingSigningKey,
    #[error("invalid signing key encoding")]
    /// Variant `BadSigningKey`.
    BadSigningKey,
}

/// A keyring for verifying ops.audit* signatures.
#[derive(Debug, Clone)]
pub struct AuditKeyRing {
    keys: HashMap<String, VerifyingKey>,
    default_key_id: Option<String>,
}

impl AuditKeyRing {
    /// Load verifying keys from environment.
    ///
    /// Format:
    /// - RHELMA_AUDIT_PUBKEYS=BASE64PK
    /// - RHELMA_AUDIT_PUBKEYS=k1:BASE64PK,k2:BASE64PK
    ///   Optional: RHELMA_AUDIT_DEFAULT_KEY_ID=k1
    pub fn from_env() -> Result<Self, AuditSigError> {
        let raw = env::var("RHELMA_AUDIT_PUBKEYS").map_err(|_| AuditSigError::MissingPubKeys)?;
        let default_key_id = env::var("RHELMA_AUDIT_DEFAULT_KEY_ID").ok();

        let mut keys = HashMap::new();
        let items: Vec<&str> = raw
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        if items.len() == 1 && !items[0].contains(':') {
            let vk = decode_verifying_key(items[0])?;
            keys.insert("default".to_string(), vk);
            return Ok(Self {
                keys,
                default_key_id: default_key_id.or(Some("default".into())),
            });
        }

        for item in items {
            let mut it = item.splitn(2, ':');
            let id = it.next().unwrap().trim();
            let b64 = it.next().ok_or(AuditSigError::BadPublicKey)?.trim();
            if id.is_empty() {
                return Err(AuditSigError::BadPublicKey);
            }
            let vk = decode_verifying_key(b64)?;
            keys.insert(id.to_string(), vk);
        }

        Ok(Self {
            keys,
            default_key_id,
        })
    }

    fn get(&self, key_id: Option<&str>) -> Result<&VerifyingKey, AuditSigError> {
        let chosen = if let Some(id) = key_id {
            id.to_string()
        } else if let Some(id) = &self.default_key_id {
            id.clone()
        } else {
            "default".to_string()
        };

        self.keys
            .get(&chosen)
            .ok_or(AuditSigError::UnknownKeyId(chosen))
    }
}

/// Optional signer for emitting ops.audit.failure (or signing audit events).
#[derive(Clone)]
pub struct AuditSigner {
    key_id: Option<String>,
    signing_key: SigningKey,
}

impl AuditSigner {
    /// Load signing key from env:
    /// RHELMA_AUDIT_SIGNING_KEY=BASE64_32BYTE_SEED
    /// Optional: RHELMA_AUDIT_KEY_ID=k1
    pub fn from_env() -> Result<Self, AuditSigError> {
        let raw =
            env::var("RHELMA_AUDIT_SIGNING_KEY").map_err(|_| AuditSigError::MissingSigningKey)?;
        let bytes = B64
            .decode(raw.trim())
            .map_err(|_| AuditSigError::BadSigningKey)?;
        // Accept 32-byte seed or 64-byte expanded secret key (take first 32)
        let seed: [u8; 32] = match bytes.len() {
            32 => bytes
                .as_slice()
                .try_into()
                .map_err(|_| AuditSigError::BadSigningKey)?,
            64 => bytes[0..32]
                .try_into()
                .map_err(|_| AuditSigError::BadSigningKey)?,
            _ => return Err(AuditSigError::BadSigningKey),
        };

        let signing_key = SigningKey::from_bytes(&seed);
        let key_id = env::var("RHELMA_AUDIT_KEY_ID")
            .ok()
            .filter(|s| !s.trim().is_empty());
        Ok(Self {
            key_id,
            signing_key,
        })
    }

    pub fn sign_digest(&self, digest32: &[u8; 32]) -> String {
        let sig: Signature = self.signing_key.sign(digest32);
        let b64 = B64.encode(sig.to_bytes());
        match &self.key_id {
            Some(id) => format!("ed25519:{id}:{b64}"),
            None => format!("ed25519:{b64}"),
        }
    }
}

/// Verify an ops.audit* envelope signature (ed25519 over sha256(canonical_payload)).
pub fn verify_audit_envelope(
    env: &EventEnvelope,
    ring: &AuditKeyRing,
) -> Result<(), AuditSigError> {
    if !env.topic.starts_with("ops.audit") {
        return Ok(());
    }

    let sig_raw = env
        .signature
        .as_deref()
        .ok_or(AuditSigError::MissingSignature)?;
    let (key_id, sig_bytes) = parse_signature(sig_raw)?;
    let vk = ring.get(key_id.as_deref())?;

    let digest = crate::canonicalization::audit_payload_digest(&env.payload);

    // If the envelope carries a hash, validate it matches the canonical digest.
    if let Some(h) = env.hash.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let computed_hex = hex::encode(digest);
        if !h.eq_ignore_ascii_case(&computed_hex) {
            return Err(AuditSigError::VerifyFailed);
        }
    }

    let sig = Signature::from_bytes(&sig_bytes);
    vk.verify_strict(&digest, &sig)
        .map_err(|_| AuditSigError::VerifyFailed)?;
    Ok(())
}

/// Build `ops.audit.failure` envelope, optionally signed.
///
/// Note: This produces an envelope that complies with the v5.2 *shape*.
/// If `signer` is `None`, publishing via `finalize_strict()` will fail because
/// audit topics require a signature by policy.
pub fn build_audit_failure(
    original: &EventEnvelope,
    reason: &str,
    signer: Option<&AuditSigner>,
) -> EventEnvelope {
    let payload = serde_json::json!({
        "original_event_id": original.event_id,
        "original_topic": original.topic,
        "reason": reason,
        "request_id": original.request.request_id,
        "correlation_id": original.request.correlation_id,
    });

    let digest = crate::canonicalization::audit_payload_digest(&payload);
    let hash_hex = hex::encode(digest);

    let mut env = EventEnvelope {
        event_id: crate::generate_event_id(),
        event_version: original.event_version,

        topic: "ops.audit.failure".to_string(),
        key: original.key.clone(),

        timestamp: Utc::now(),
        published_at: Utc::now(),

        source: EventSource {
            service: original.source.service.clone(),
            version: original.source.version.clone(),
            region: original.source.region.clone(),
        },

        request: original.request.clone(),
        trace: original.trace.clone(),

        payload,
        payload_type: "ops.audit.failure@v1".to_string(),
        schema_ref: "ops.audit.failure@v1".to_string(),

        policy: PolicyMeta::public(purpose::OPS_AUDIT),
        residency: original.residency,
        encryption: None,
        signature: None,
        hash: Some(hash_hex),
    };

    if let Some(s) = signer {
        env.signature = Some(s.sign_digest(&digest));
    }

    env
}

fn parse_signature(raw: &str) -> Result<(Option<String>, [u8; 64]), AuditSigError> {
    let parts: Vec<&str> = raw.split(':').collect();
    if parts.is_empty() {
        return Err(AuditSigError::BadFormat);
    }
    if parts[0] != "ed25519" {
        return Err(AuditSigError::BadFormat);
    }

    let (key_id, b64sig) = match parts.len() {
        2 => (None, parts[1]),
        3 => (Some(parts[1].to_string()), parts[2]),
        _ => return Err(AuditSigError::BadFormat),
    };

    let sig = B64
        .decode(b64sig.trim())
        .map_err(|_| AuditSigError::BadBase64)?;
    if sig.len() != 64 {
        return Err(AuditSigError::BadSignatureLen);
    }
    let arr: [u8; 64] = sig
        .as_slice()
        .try_into()
        .map_err(|_| AuditSigError::BadSignatureLen)?;
    Ok((key_id, arr))
}

fn decode_verifying_key(b64: &str) -> Result<VerifyingKey, AuditSigError> {
    let bytes = B64
        .decode(b64.trim())
        .map_err(|_| AuditSigError::BadPublicKey)?;
    if bytes.len() != 32 {
        return Err(AuditSigError::BadPublicKey);
    }
    let arr: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| AuditSigError::BadPublicKey)?;
    VerifyingKey::from_bytes(&arr).map_err(|_| AuditSigError::BadPublicKey)
}

/// Compute the canonical audit digest: sha256( canonical_json(payload) ).
pub fn audit_payload_digest(payload: &Value) -> [u8; 32] {
    crate::canonicalization::audit_payload_digest(payload)
}
