//! Policy Bundle schema + verification.
//!
//! The schema in this module matches `docs/governance/POLICY_BUNDLES_v1.md`.
//!
//! **Important**: this is a constitutional enforcement layer. The default
//! behavior is *fail-open* unless explicit environment variables enable
//! strict enforcement.

#![forbid(unsafe_code)]

use crate::error::RhelmaError;
use crate::result::RhelmaResult;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use super::crypto::{
    ed25519_verify_b64url, hs256_verify_b64url, GovernanceKeySets, ED25519_FPR_PREFIX,
    HS256_FPR_PREFIX,
};

/// The supported Policy Bundle class values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PolicyBundleV1Class {
    /// Standard (normal operations).
    Standard,
    /// High impact changes (stronger quorum).
    HighImpact,
    /// Emergency changes (short-lived, Security Council quorum).
    Emergency,
    /// Critical changes (requires both Policy and Security Council quorums + timelock).
    Critical,
}

/// A single signature entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicySignatureV1 {
    /// Key fingerprint (e.g. `hs256:<kid>` or `ed25519:<kid>`).
    pub key_fpr: String,
    /// Signature bytes (base64url, no padding).
    pub sig: String,
}

/// Policy Bundle schema (v1.0).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyBundleV1 {
    /// Field `bundle_id` (UUID string).
    pub bundle_id: String,
    /// Field `version`.
    pub version: String,
    /// Field `created_at`.
    pub created_at: DateTime<Utc>,
    /// Field `prev_bundle_hash` (base64url(sha256), or null for genesis).
    pub prev_bundle_hash: Option<String>,
    /// Field `class`.
    pub class: PolicyBundleV1Class,
    /// Field `summary`.
    pub summary: String,
    /// Field `policy`.
    pub policy: Value,

    /// Field `expires_at` (required for emergency bundles).
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    /// Field `rollback_plan` (required for emergency bundles).
    #[serde(default)]
    pub rollback_plan: Option<String>,

    /// Field `activate_not_before` (optional timelock).
    ///
    /// If present, nodes MUST NOT activate this bundle before the given timestamp.
    #[serde(default)]
    pub activate_not_before: Option<DateTime<Utc>>,

    /// Field `signatures`.
    pub signatures: Vec<PolicySignatureV1>,
}

/// Verification output for a v1 Policy Bundle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerifiedPolicyBundleV1 {
    /// The policy bundle.
    pub bundle: PolicyBundleV1,
    /// Canonical bundle hash (base64url(sha256(canonical_json(bundle_without_signatures)))).
    pub bundle_hash: String,
    /// The fingerprints of signers that verified successfully.
    pub verified_signers: Vec<String>,
    /// Quorum required for this bundle (for non-critical bundles).
    ///
    /// For `class=critical`, this is the **sum** of the per-council requirements.
    pub quorum_required: usize,
    /// Council keyset size used for verification (for non-critical bundles).
    ///
    /// For `class=critical`, this is the **sum** of policy + security council sizes.
    pub council_size: usize,

    /// For `class=critical`, the required policy council signatures.
    #[serde(default)]
    pub policy_quorum_required: Option<usize>,
    /// For `class=critical`, the required security council signatures.
    #[serde(default)]
    pub security_quorum_required: Option<usize>,
    /// For `class=critical`, the policy council size.
    #[serde(default)]
    pub policy_council_size: Option<usize>,
    /// For `class=critical`, the security council size.
    #[serde(default)]
    pub security_council_size: Option<usize>,
}

/// Maximum lifetime for emergency bundles (72 hours).
pub const MAX_EMERGENCY_HOURS: i64 = 72;

/// Environment variable: quorum mode.
///
/// Values:
/// - `fixed` (default): use fixed class quorum (3/4/2)
/// - `majority`: require majority of the council (but never less than fixed default)
/// - `supermajority`: require 2/3 of the council (but never less than fixed default)
pub const ENV_QUORUM_MODE: &str = "RHELMA_GOVERNANCE_QUORUM_MODE";

/// Environment variable: minimum activation delay for HighImpact bundles (seconds).
///
/// If set to a positive value, HighImpact bundles MUST include `activate_not_before`,
/// and it must be at least `created_at + delay`.
pub const ENV_HIGH_IMPACT_MIN_DELAY_SECS: &str = "RHELMA_GOVERNANCE_HIGH_IMPACT_MIN_DELAY_SECONDS";

/// Environment variable: minimum activation delay for Critical bundles (seconds).
///
/// If set to a positive value, `class=critical` bundles MUST include `activate_not_before`,
/// and it must be at least `created_at + delay`.
///
/// Default (when unset): 86400 (24h). Set to 0 to disable the minimum delay check.
pub const ENV_CRITICAL_MIN_DELAY_SECS: &str = "RHELMA_GOVERNANCE_CRITICAL_MIN_DELAY_SECONDS";

/// Environment variable: critical policy council quorum override (count).
pub const ENV_QUORUM_CRITICAL_POLICY: &str = "RHELMA_GOVERNANCE_QUORUM_CRITICAL_POLICY";
/// Environment variable: critical security council quorum override (count).
pub const ENV_QUORUM_CRITICAL_SECURITY: &str = "RHELMA_GOVERNANCE_QUORUM_CRITICAL_SECURITY";

/// Load a policy bundle from a JSON file path.
pub fn load_policy_bundle_from_path(path: &Path) -> RhelmaResult<PolicyBundleV1> {
    let raw = std::fs::read_to_string(path).map_err(|e| {
        RhelmaError::SecurityPolicy(format!(
            "governance_policy_read_error: cannot read {} ({e})",
            path.display()
        ))
    })?;

    if path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| matches!(e.to_ascii_lowercase().as_str(), "yaml" | "yml"))
        .unwrap_or(false)
    {
        return Err(RhelmaError::SecurityPolicy(
            "governance_policy_format_unsupported: YAML bundles are not supported in v1 bootstrap (use JSON)"
                .to_string(),
        ));
    }

    serde_json::from_str::<PolicyBundleV1>(&raw).map_err(|e| {
        RhelmaError::SecurityPolicy(format!(
            "governance_policy_parse_error: invalid JSON policy bundle at {} ({e})",
            path.display()
        ))
    })
}

/// Compute the canonical v1 bundle hash for an in-memory bundle.
pub fn compute_bundle_hash(bundle: &PolicyBundleV1) -> RhelmaResult<String> {
    // Canonical JSON is computed over the bundle without `signatures`.
    let mut v = serde_json::to_value(bundle).map_err(|e| {
        RhelmaError::SecurityPolicy(format!("governance_policy_serialize_error: {e}"))
    })?;

    if let Value::Object(ref mut map) = v {
        map.remove("signatures");
    }

    let bytes = serde_json::to_vec(&v).map_err(|e| {
        RhelmaError::SecurityPolicy(format!("governance_policy_canonical_error: {e}"))
    })?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    Ok(URL_SAFE_NO_PAD.encode(digest))
}

/// Verify a policy bundle against governance key sets and quorum rules.
///
/// Quorum defaults (can be overridden via env in the bootstrap layer):
/// - standard: 3-of-policy_council
/// - high_impact: 4-of-policy_council
/// - emergency: 2-of-security_council
/// - critical: 4-of-policy_council + 3-of-security_council (requires timelock)
pub fn verify_policy_bundle_v1(
    bundle: PolicyBundleV1,
    keys: &GovernanceKeySets,
    quorum_overrides: Option<&HashMap<PolicyBundleV1Class, usize>>,
    now: DateTime<Utc>,
) -> RhelmaResult<VerifiedPolicyBundleV1> {
    validate_bundle_semantics(&bundle, now)?;

    if matches!(bundle.class, PolicyBundleV1Class::Critical) {
        return verify_policy_bundle_v1_critical(bundle, keys, now);
    }

    let bundle_hash = compute_bundle_hash(&bundle)?;
    let bundle_hash_bytes = URL_SAFE_NO_PAD
        .decode(bundle_hash.as_bytes())
        .map_err(|_| {
            RhelmaError::SecurityPolicy(
                "governance_policy_hash_decode_error: internal bundle hash invalid".to_string(),
            )
        })?;

    let (keysets, quorum_required, council_size) =
        select_keysets_and_quorum(&bundle, keys, quorum_overrides)?;

    let mut verified: BTreeSet<String> = BTreeSet::new();

    for sig in &bundle.signatures {
        let fpr = sig.key_fpr.trim();

        if fpr.starts_with(HS256_FPR_PREFIX) {
            if let Some(secret) = keysets.hs256.get(fpr) {
                if hs256_verify_b64url(&bundle_hash_bytes, &sig.sig, secret) {
                    verified.insert(fpr.to_string());
                }
            }
            continue;
        }

        if fpr.starts_with(ED25519_FPR_PREFIX) {
            if let Some(vk) = keysets.ed25519.get(fpr) {
                if ed25519_verify_b64url(&bundle_hash_bytes, &sig.sig, vk) {
                    verified.insert(fpr.to_string());
                }
            }
            continue;
        }

        // Unknown scheme.
    }

    if verified.len() < quorum_required {
        return Err(RhelmaError::SecurityPolicy(format!(
            "governance_policy_quorum_not_met: class={:?} verified={} required={} council_size={}",
            bundle.class,
            verified.len(),
            quorum_required,
            council_size
        )));
    }

    Ok(VerifiedPolicyBundleV1 {
        bundle,
        bundle_hash,
        verified_signers: verified.into_iter().collect(),
        quorum_required,
        council_size,
        policy_quorum_required: None,
        security_quorum_required: None,
        policy_council_size: None,
        security_council_size: None,
    })
}

fn verify_policy_bundle_v1_critical(
    bundle: PolicyBundleV1,
    keys: &GovernanceKeySets,
    now: DateTime<Utc>,
) -> RhelmaResult<VerifiedPolicyBundleV1> {
    let bundle_hash = compute_bundle_hash(&bundle)?;
    let bundle_hash_bytes = URL_SAFE_NO_PAD
        .decode(bundle_hash.as_bytes())
        .map_err(|_| {
            RhelmaError::SecurityPolicy(
                "governance_policy_hash_decode_error: internal bundle hash invalid".to_string(),
            )
        })?;

    // Required quorums (defaults).
    let mut required_policy = env_usize(ENV_QUORUM_CRITICAL_POLICY).unwrap_or(4usize);
    let mut required_security = env_usize(ENV_QUORUM_CRITICAL_SECURITY).unwrap_or(3usize);

    // Optional dynamic quorum mode (fail-open; only increases requirements).
    match env_string(ENV_QUORUM_MODE).as_deref().unwrap_or("fixed") {
        "majority" => {
            let policy_size = keys.policy_council.len() + keys.policy_council_ed25519.len();
            let sec_size = keys.security_council.len() + keys.security_council_ed25519.len();
            required_policy = required_policy.max((policy_size / 2) + 1);
            required_security = required_security.max((sec_size / 2) + 1);
        }
        "supermajority" | "super_majority" | "super-majority" => {
            let policy_size = keys.policy_council.len() + keys.policy_council_ed25519.len();
            let sec_size = keys.security_council.len() + keys.security_council_ed25519.len();
            // FIX: Changed from `(2 * policy_size + 2) / 3` to `.div_ceil()`
            required_policy = required_policy.max((2 * policy_size).div_ceil(3));
            required_security = required_security.max((2 * sec_size).div_ceil(3));
        }
        _ => {}
    }

    let policy_size = keys.policy_council.len() + keys.policy_council_ed25519.len();
    let security_size = keys.security_council.len() + keys.security_council_ed25519.len();

    if policy_size < required_policy {
        return Err(RhelmaError::SecurityPolicy(format!(
            "governance_policy_keyset_too_small: class=critical required_policy={} policy_council_size={} ",
            required_policy, policy_size
        )));
    }
    if security_size < required_security {
        return Err(RhelmaError::SecurityPolicy(format!(
            "governance_policy_keyset_too_small: class=critical required_security={} security_council_size={} ",
            required_security, security_size
        )));
    }

    // Prevent ambiguous key assignment when councils overlap.
    let hs_overlap = keys
        .policy_council
        .keys()
        .any(|k| keys.security_council.contains_key(k));
    let ed_overlap = keys
        .policy_council_ed25519
        .keys()
        .any(|k| keys.security_council_ed25519.contains_key(k));
    if hs_overlap || ed_overlap {
        return Err(RhelmaError::SecurityPolicy(
            "governance_policy_critical_keyset_overlap: policy and security councils must not share key_fpr".to_string(),
        ));
    }

    let mut verified_policy: BTreeSet<String> = BTreeSet::new();
    let mut verified_security: BTreeSet<String> = BTreeSet::new();
    let mut verified_all: BTreeSet<String> = BTreeSet::new();

    for sig in &bundle.signatures {
        let fpr = sig.key_fpr.trim();

        if fpr.starts_with(HS256_FPR_PREFIX) {
            if let Some(secret) = keys.policy_council.get(fpr) {
                if hs256_verify_b64url(&bundle_hash_bytes, &sig.sig, secret) {
                    verified_policy.insert(fpr.to_string());
                    verified_all.insert(fpr.to_string());
                    continue;
                }
            }
            if let Some(secret) = keys.security_council.get(fpr) {
                if hs256_verify_b64url(&bundle_hash_bytes, &sig.sig, secret) {
                    verified_security.insert(fpr.to_string());
                    verified_all.insert(fpr.to_string());
                }
            }
            continue;
        }

        if fpr.starts_with(ED25519_FPR_PREFIX) {
            if let Some(vk) = keys.policy_council_ed25519.get(fpr) {
                if ed25519_verify_b64url(&bundle_hash_bytes, &sig.sig, vk) {
                    verified_policy.insert(fpr.to_string());
                    verified_all.insert(fpr.to_string());
                    continue;
                }
            }
            if let Some(vk) = keys.security_council_ed25519.get(fpr) {
                if ed25519_verify_b64url(&bundle_hash_bytes, &sig.sig, vk) {
                    verified_security.insert(fpr.to_string());
                    verified_all.insert(fpr.to_string());
                }
            }
            continue;
        }
    }

    if verified_policy.len() < required_policy || verified_security.len() < required_security {
        return Err(RhelmaError::SecurityPolicy(format!(
            "governance_policy_quorum_not_met: class=critical verified_policy={} required_policy={} verified_security={} required_security={} policy_council_size={} security_council_size={} now={}",
            verified_policy.len(),
            required_policy,
            verified_security.len(),
            required_security,
            policy_size,
            security_size,
            now.to_rfc3339(),
        )));
    }

    // FIX: Changed `verified` to `verified_all` - the correct variable in this scope
    Ok(VerifiedPolicyBundleV1 {
        bundle,
        bundle_hash,
        verified_signers: verified_all.into_iter().collect(),
        quorum_required: required_policy + required_security,
        council_size: policy_size + security_size,
        policy_quorum_required: Some(required_policy),
        security_quorum_required: Some(required_security),
        policy_council_size: Some(policy_size),
        security_council_size: Some(security_size),
    })
}

fn validate_bundle_semantics(bundle: &PolicyBundleV1, now: DateTime<Utc>) -> RhelmaResult<()> {
    if bundle.summary.len() > 4000 {
        return Err(RhelmaError::SecurityPolicy(
            "governance_policy_invalid_summary: summary exceeds 4000 characters".to_string(),
        ));
    }

    // Optional time-lock: if set, the bundle cannot be activated before this timestamp.
    if let Some(not_before) = bundle.activate_not_before {
        if not_before < bundle.created_at {
            return Err(RhelmaError::SecurityPolicy(
                "governance_policy_invalid_activate_not_before: activate_not_before precedes created_at"
                    .to_string(),
            ));
        }
        if now < not_before {
            return Err(RhelmaError::SecurityPolicy(format!(
                "governance_policy_not_yet_active: activate_not_before={} now={}",
                not_before.to_rfc3339(),
                now.to_rfc3339()
            )));
        }
    }

    // If configured, require a minimum activation delay for HighImpact bundles.
    if matches!(bundle.class, PolicyBundleV1Class::HighImpact) {
        let min_delay = env_u64(ENV_HIGH_IMPACT_MIN_DELAY_SECS).unwrap_or(0);
        if min_delay > 0 {
            let required_not_before = bundle.created_at + Duration::seconds(min_delay as i64);
            let not_before = bundle.activate_not_before.ok_or_else(|| {
                RhelmaError::SecurityPolicy(
                    "governance_policy_high_impact_requires_timelock: activate_not_before is required"
                        .to_string(),
                )
            })?;
            if not_before < required_not_before {
                return Err(RhelmaError::SecurityPolicy(format!(
                    "governance_policy_high_impact_timelock_too_short: required_not_before={} got={}",
                    required_not_before.to_rfc3339(),
                    not_before.to_rfc3339()
                )));
            }
        }
    }

    // Critical bundles: require a (configurable) minimum activation delay.
    if matches!(bundle.class, PolicyBundleV1Class::Critical) {
        let min_delay = env_u64(ENV_CRITICAL_MIN_DELAY_SECS).unwrap_or(86400);
        if min_delay > 0 {
            let required_not_before = bundle.created_at + Duration::seconds(min_delay as i64);
            let not_before = bundle.activate_not_before.ok_or_else(|| {
                RhelmaError::SecurityPolicy(
                    "governance_policy_critical_requires_timelock: activate_not_before is required"
                        .to_string(),
                )
            })?;
            if not_before < required_not_before {
                return Err(RhelmaError::SecurityPolicy(format!(
                    "governance_policy_critical_timelock_too_short: required_not_before={} got={}",
                    required_not_before.to_rfc3339(),
                    not_before.to_rfc3339()
                )));
            }
        }
    }

    if matches!(bundle.class, PolicyBundleV1Class::Emergency) {
        let expires_at = bundle.expires_at.ok_or_else(|| {
            RhelmaError::SecurityPolicy(
                "governance_policy_emergency_missing_expires_at: emergency bundles require expires_at"
                    .to_string(),
            )
        })?;
        let rollback = bundle
            .rollback_plan
            .as_ref()
            .map(|s| s.trim())
            .unwrap_or("");
        if rollback.is_empty() {
            return Err(RhelmaError::SecurityPolicy(
                "governance_policy_emergency_missing_rollback_plan: emergency bundles require rollback_plan"
                    .to_string(),
            ));
        }

        // Enforce max lifetime.
        let max = bundle.created_at + Duration::hours(MAX_EMERGENCY_HOURS);
        if expires_at > max {
            return Err(RhelmaError::SecurityPolicy(
                "governance_policy_emergency_too_long: expires_at exceeds 72 hours".to_string(),
            ));
        }

        if let Some(not_before) = bundle.activate_not_before {
            if not_before > expires_at {
                return Err(RhelmaError::SecurityPolicy(
                    "governance_policy_emergency_activate_after_expires: activate_not_before must be <= expires_at"
                        .to_string(),
                ));
            }
        }

        if now > expires_at {
            return Err(RhelmaError::SecurityPolicy(
                "governance_policy_emergency_expired: emergency bundle is past expires_at"
                    .to_string(),
            ));
        }
    }

    Ok(())
}

struct CouncilKeySetsRef<'a> {
    hs256: &'a std::collections::BTreeMap<String, Vec<u8>>,
    ed25519: &'a std::collections::BTreeMap<String, ed25519_dalek::VerifyingKey>,
}

fn select_keysets_and_quorum<'a>(
    bundle: &PolicyBundleV1,
    keys: &'a GovernanceKeySets,
    quorum_overrides: Option<&HashMap<PolicyBundleV1Class, usize>>,
) -> RhelmaResult<(CouncilKeySetsRef<'a>, usize, usize)> {
    let override_n = quorum_overrides.and_then(|m| m.get(&bundle.class)).copied();

    let (hs256, ed25519, default_quorum) = match bundle.class {
        PolicyBundleV1Class::Emergency => (
            &keys.security_council,
            &keys.security_council_ed25519,
            2usize,
        ),
        PolicyBundleV1Class::HighImpact => {
            (&keys.policy_council, &keys.policy_council_ed25519, 4usize)
        }
        // `Critical` is handled separately in verification; this arm is only to keep the selector exhaustive.
        PolicyBundleV1Class::Critical => {
            (&keys.policy_council, &keys.policy_council_ed25519, 4usize)
        }
        PolicyBundleV1Class::Standard => {
            (&keys.policy_council, &keys.policy_council_ed25519, 3usize)
        }
    };

    let council_size = hs256.len() + ed25519.len();
    let mut required = override_n.unwrap_or(default_quorum);

    // Optional dynamic quorum mode (fail-open; only increases requirements).
    if override_n.is_none() {
        match env_string(ENV_QUORUM_MODE).as_deref().unwrap_or("fixed") {
            "majority" => {
                let maj = (council_size / 2) + 1;
                required = required.max(maj);
            }
            "supermajority" | "super_majority" | "super-majority" => {
                // FIX: Changed from `(2 * council_size + 2) / 3` to `.div_ceil()`
                let supermaj = (2 * council_size).div_ceil(3);
                required = required.max(supermaj);
            }
            _ => {}
        }
    }

    if council_size < required {
        return Err(RhelmaError::SecurityPolicy(format!(
            "governance_policy_keyset_too_small: class={:?} required={} council_size={}",
            bundle.class, required, council_size
        )));
    }

    Ok((CouncilKeySetsRef { hs256, ed25519 }, required, council_size))
}

fn env_u64(key: &str) -> Option<u64> {
    std::env::var(key)
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
}

fn env_usize(key: &str) -> Option<usize> {
    std::env::var(key)
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
}

fn env_string(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}
