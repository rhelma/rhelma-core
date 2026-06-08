//! Governance cryptography helpers.
//!
//! v1 bootstrap used **HS256 (HMAC-SHA256)** shared secrets.
//! That is simple, but it is a shared-secret scheme.
//!
//! This module supports an additive upgrade to **Ed25519** public keys for
//! non-repudiation, while keeping the on-wire Policy Bundle schema stable:
//! signatures are computed over the canonical `bundle_hash` bytes.

#![forbid(unsafe_code)]

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ed25519_dalek::Signature;
use ed25519_dalek::VerifyingKey;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

use std::collections::BTreeMap;

/// Prefix used for HS256 governance key fingerprints.
pub const HS256_FPR_PREFIX: &str = "hs256:";
/// Prefix used for Ed25519 governance key fingerprints.
pub const ED25519_FPR_PREFIX: &str = "ed25519:";

/// Environment variable: policy council HS256 keys (`kid:secret,kid:secret`).
pub const ENV_POLICY_COUNCIL_KEYS: &str = "RHELMA_GOVERNANCE__POLICY_COUNCIL_HMAC_KEYS";
/// Environment variable: security council HS256 keys (`kid:secret,kid:secret`).
pub const ENV_SECURITY_COUNCIL_KEYS: &str = "RHELMA_GOVERNANCE__SECURITY_COUNCIL_HMAC_KEYS";
/// Environment variable: fallback governance HS256 key list.
pub const ENV_GOVERNANCE_KEYS_FALLBACK: &str = "RHELMA_GOVERNANCE__HMAC_KEYS";

/// Environment variable: policy council Ed25519 public keys (`kid:pubkey_b64url,kid2:pubkey_b64url`).
pub const ENV_POLICY_COUNCIL_ED25519_PUBKEYS: &str =
    "RHELMA_GOVERNANCE__POLICY_COUNCIL_ED25519_PUBKEYS";
/// Environment variable: security council Ed25519 public keys (`kid:pubkey_b64url,kid2:pubkey_b64url`).
pub const ENV_SECURITY_COUNCIL_ED25519_PUBKEYS: &str =
    "RHELMA_GOVERNANCE__SECURITY_COUNCIL_ED25519_PUBKEYS";
/// Environment variable: fallback Ed25519 public key list.
pub const ENV_GOVERNANCE_ED25519_PUBKEYS_FALLBACK: &str = "RHELMA_GOVERNANCE__ED25519_PUBKEYS";

/// Parse a comma-separated HS256 key list in the form `kid:secret,kid2:secret2`.
///
/// Secrets are treated as UTF-8 bytes.
pub fn parse_hs256_key_map(raw: &str) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();

    for entry in raw.split(',').map(|e| e.trim()).filter(|e| !e.is_empty()) {
        let (kid, sec) = match entry.split_once(':') {
            Some((k, s)) => (k.trim(), s),
            None => ("", entry),
        };

        let kid = if kid.is_empty() { "legacy" } else { kid };
        let fpr = format!("{}{}", HS256_FPR_PREFIX, kid);
        if !sec.trim().is_empty() {
            out.insert(fpr, sec.as_bytes().to_vec());
        }
    }

    out
}

/// Parse a comma-separated Ed25519 public key list in the form `kid:pubkey_b64url,kid2:pubkey_b64url`.
///
/// - `pubkey_b64url` must decode to 32 bytes.
/// - keys are stored under fingerprint `ed25519:<kid>`.
pub fn parse_ed25519_pubkey_map(raw: &str) -> BTreeMap<String, VerifyingKey> {
    let mut out = BTreeMap::new();

    for entry in raw.split(',').map(|e| e.trim()).filter(|e| !e.is_empty()) {
        let (kid, pk) = match entry.split_once(':') {
            Some((k, p)) => (k.trim(), p.trim()),
            None => ("", entry.trim()),
        };

        let kid = if kid.is_empty() { "legacy" } else { kid };
        let fpr = format!("{}{}", ED25519_FPR_PREFIX, kid);

        let Ok(bytes) = URL_SAFE_NO_PAD.decode(pk.as_bytes()) else {
            continue;
        };
        let Ok(b32) = <[u8; 32]>::try_from(bytes.as_slice()) else {
            continue;
        };
        let Ok(vk) = VerifyingKey::from_bytes(&b32) else {
            continue;
        };
        out.insert(fpr, vk);
    }

    out
}

/// Governance key sets loaded from environment.
///
/// Both HS256 and Ed25519 can be enabled simultaneously.
#[derive(Debug, Clone, Default)]
pub struct GovernanceKeySets {
    /// HS256 key fingerprint -> secret.
    pub policy_council: BTreeMap<String, Vec<u8>>,
    /// HS256 key fingerprint -> secret.
    pub security_council: BTreeMap<String, Vec<u8>>,

    /// Ed25519 key fingerprint -> verifying key.
    pub policy_council_ed25519: BTreeMap<String, VerifyingKey>,
    /// Ed25519 key fingerprint -> verifying key.
    pub security_council_ed25519: BTreeMap<String, VerifyingKey>,
}

impl GovernanceKeySets {
    /// Load governance key sets from environment.
    ///
    /// HS256 precedence:
    /// 1) specific vars (`RHELMA_GOVERNANCE__POLICY_COUNCIL_HMAC_KEYS`, `RHELMA_GOVERNANCE__SECURITY_COUNCIL_HMAC_KEYS`)
    /// 2) `RHELMA_GOVERNANCE__HMAC_KEYS`
    /// 3) fallback to `RHELMA_AI_ATTESTATION__HMAC_KEYS`
    ///
    /// Ed25519 precedence:
    /// 1) specific vars (`RHELMA_GOVERNANCE__POLICY_COUNCIL_ED25519_PUBKEYS`, `RHELMA_GOVERNANCE__SECURITY_COUNCIL_ED25519_PUBKEYS`)
    /// 2) `RHELMA_GOVERNANCE__ED25519_PUBKEYS`
    /// 3) fallback to `RHELMA_AI_ATTESTATION__ED25519_PUBKEYS`
    pub fn from_env() -> Self {
        let policy_hs256 = std::env::var(ENV_POLICY_COUNCIL_KEYS)
            .ok()
            .and_then(|v| Some(v).filter(|s| !s.trim().is_empty()))
            .map(|v| parse_hs256_key_map(&v));

        let security_hs256 = std::env::var(ENV_SECURITY_COUNCIL_KEYS)
            .ok()
            .and_then(|v| Some(v).filter(|s| !s.trim().is_empty()))
            .map(|v| parse_hs256_key_map(&v));

        let fallback_hs256 = std::env::var(ENV_GOVERNANCE_KEYS_FALLBACK)
            .ok()
            .and_then(|v| Some(v).filter(|s| !s.trim().is_empty()))
            .or_else(|| {
                std::env::var("RHELMA_AI_ATTESTATION__HMAC_KEYS")
                    .ok()
                    .and_then(|v| Some(v).filter(|s| !s.trim().is_empty()))
            })
            .map(|v| parse_hs256_key_map(&v))
            .unwrap_or_default();

        let policy_ed = std::env::var(ENV_POLICY_COUNCIL_ED25519_PUBKEYS)
            .ok()
            .and_then(|v| Some(v).filter(|s| !s.trim().is_empty()))
            .map(|v| parse_ed25519_pubkey_map(&v));

        let security_ed = std::env::var(ENV_SECURITY_COUNCIL_ED25519_PUBKEYS)
            .ok()
            .and_then(|v| Some(v).filter(|s| !s.trim().is_empty()))
            .map(|v| parse_ed25519_pubkey_map(&v));

        let fallback_ed = std::env::var(ENV_GOVERNANCE_ED25519_PUBKEYS_FALLBACK)
            .ok()
            .and_then(|v| Some(v).filter(|s| !s.trim().is_empty()))
            .or_else(|| {
                std::env::var("RHELMA_AI_ATTESTATION__ED25519_PUBKEYS")
                    .ok()
                    .and_then(|v| Some(v).filter(|s| !s.trim().is_empty()))
            })
            .map(|v| parse_ed25519_pubkey_map(&v))
            .unwrap_or_default();

        Self {
            policy_council: policy_hs256.unwrap_or_else(|| fallback_hs256.clone()),
            security_council: security_hs256.unwrap_or(fallback_hs256),
            policy_council_ed25519: policy_ed.unwrap_or_else(|| fallback_ed.clone()),
            security_council_ed25519: security_ed.unwrap_or(fallback_ed),
        }
    }

    /// Returns `true` if no keys are configured at all.
    pub fn is_empty(&self) -> bool {
        self.policy_council.is_empty()
            && self.security_council.is_empty()
            && self.policy_council_ed25519.is_empty()
            && self.security_council_ed25519.is_empty()
    }
}

/// Compute an HS256 signature over the given message bytes.
pub fn hs256_sign(message: &[u8], secret: &[u8]) -> Option<Vec<u8>> {
    let mut mac: Hmac<Sha256> = Hmac::new_from_slice(secret).ok()?;
    mac.update(message);
    Some(mac.finalize().into_bytes().to_vec())
}

/// Verify an HS256 signature (base64url, no padding) over the given message bytes.
pub fn hs256_verify_b64url(message: &[u8], sig_b64url: &str, secret: &[u8]) -> bool {
    let Some(expected) = hs256_sign(message, secret) else {
        return false;
    };
    let Ok(sig) = URL_SAFE_NO_PAD.decode(sig_b64url.as_bytes()) else {
        return false;
    };

    expected.as_slice().ct_eq(sig.as_slice()).unwrap_u8() == 1
}

/// Verify an Ed25519 signature (base64url, no padding) over the given message bytes.
pub fn ed25519_verify_b64url(message: &[u8], sig_b64url: &str, vk: &VerifyingKey) -> bool {
    let Ok(sig_bytes) = URL_SAFE_NO_PAD.decode(sig_b64url.as_bytes()) else {
        return false;
    };
    let Ok(sig) = Signature::from_slice(&sig_bytes) else {
        return false;
    };

    vk.verify_strict(message, &sig).is_ok()
}
