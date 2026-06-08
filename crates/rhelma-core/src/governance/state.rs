//! Governance policy state.
//!
//! This module holds the currently verified policy bundle (if any) and
//! computes derived runtime flags such as **emergency mode** and **safe mode**.
//!
//! The state is stored in process memory and is initialized by
//! `governance::bootstrap::ensure_governance_ready`.

#![forbid(unsafe_code)]

use crate::error::RhelmaError;
use crate::result::RhelmaResult;

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::OnceLock;

use super::crypto::GovernanceKeySets;
use super::policy::{
    load_policy_bundle_from_path, verify_policy_bundle_v1, PolicyBundleV1Class,
    VerifiedPolicyBundleV1,
};
use super::runtime::GovernanceRuntime;

/// Environment variable: policy enforcement mode.
///
/// Values:
/// - `off` (do not load/verify)
/// - `warn` (load/verify; log errors, continue)
/// - `enforce` (load/verify; fail startup on errors)
pub const ENV_POLICY_ENFORCEMENT_MODE: &str = "RHELMA_GOVERNANCE_POLICY_ENFORCEMENT_MODE";

/// Environment variable: maximum acceptable age of a bundle before safe mode.
///
/// If set, and `now - created_at > max_age`, nodes enter Safe Mode.
pub const ENV_POLICY_MAX_AGE_SECS: &str = "RHELMA_GOVERNANCE_POLICY_MAX_AGE_SECONDS";

/// Environment variable: force safe mode.
pub const ENV_SAFE_MODE: &str = "RHELMA_GOVERNANCE_SAFE_MODE";

/// Environment variable: override quorum for standard bundles.
pub const ENV_QUORUM_STANDARD: &str = "RHELMA_GOVERNANCE_QUORUM_STANDARD";
/// Environment variable: override quorum for high-impact bundles.
pub const ENV_QUORUM_HIGH_IMPACT: &str = "RHELMA_GOVERNANCE_QUORUM_HIGH_IMPACT";
/// Environment variable: override quorum for emergency bundles.
pub const ENV_QUORUM_EMERGENCY: &str = "RHELMA_GOVERNANCE_QUORUM_EMERGENCY";

/// Derived policy state for the running process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernancePolicyState {
    /// Emergency mode flag (from env and/or policy class).
    pub emergency_mode: bool,
    /// Safe mode flag (derived from stale policy or explicit env).
    pub safe_mode: bool,
    /// Active bundle hash, if loaded.
    pub bundle_hash: Option<String>,
    /// Active bundle class, if loaded.
    pub bundle_class: Option<PolicyBundleV1Class>,
    /// Active bundle creation time, if loaded.
    pub created_at: Option<DateTime<Utc>>,
    /// Human-readable warnings.
    pub warnings: Vec<String>,
}

static VERIFIED: OnceLock<VerifiedPolicyBundleV1> = OnceLock::new();
static STATE: OnceLock<GovernancePolicyState> = OnceLock::new();

/// Get the current verified policy bundle (if any).
pub fn current_policy() -> Option<&'static VerifiedPolicyBundleV1> {
    VERIFIED.get()
}

/// Get the current governance policy state.
pub fn current_policy_state() -> Option<&'static GovernancePolicyState> {
    STATE.get()
}

/// Initialize policy state from environment + optional policy bundle.
///
/// This is called once during startup.
pub fn init_policy_state(service_name: &str) -> RhelmaResult<()> {
    let rt = GovernanceRuntime::from_env();
    let mode = read_enforcement_mode();

    let mut warnings: Vec<String> = Vec::new();

    let emergency_env = rt.emergency_mode();
    let mut emergency_mode = emergency_env;
    let mut safe_mode = env_bool(ENV_SAFE_MODE).unwrap_or(false);

    if matches!(mode, EnforcementMode::Off) {
        let st = GovernancePolicyState {
            emergency_mode,
            safe_mode,
            bundle_hash: None,
            bundle_class: None,
            created_at: None,
            warnings,
        };
        let _ = STATE.set(st);
        return Ok(());
    }

    let maybe_policy_ref = rt.validate_policy_bundle()?;
    match maybe_policy_ref {
        None => {
            // No policy bundle configured/present.
            if rt.policy_required() && matches!(mode, EnforcementMode::Enforce) {
                return Err(RhelmaError::SecurityPolicy(
                    "governance_policy_missing: policy is required but not configured".to_string(),
                ));
            }
            warnings.push("governance: no policy bundle configured".to_string());

            let st = GovernancePolicyState {
                emergency_mode,
                safe_mode,
                bundle_hash: None,
                bundle_class: None,
                created_at: None,
                warnings,
            };
            let _ = STATE.set(st);
            Ok(())
        }
        Some(pref) => {
            // Load + verify.
            let bundle = load_policy_bundle_from_path(&pref.path)?;

            let keys = GovernanceKeySets::from_env();
            let quorum_overrides = read_quorum_overrides();

            let verified =
                match verify_policy_bundle_v1(bundle, &keys, quorum_overrides.as_ref(), Utc::now())
                {
                    Ok(v) => v,
                    Err(e) => {
                        if matches!(mode, EnforcementMode::Enforce) {
                            return Err(e);
                        }
                        warnings.push(format!(
                            "governance: policy verification failed for {}: {e}",
                            pref.path.display()
                        ));
                        // Keep state, but do not set VERIFIED.
                        let st = GovernancePolicyState {
                            emergency_mode,
                            safe_mode: true,
                            bundle_hash: None,
                            bundle_class: None,
                            created_at: None,
                            warnings,
                        };
                        let _ = STATE.set(st);
                        return Ok(());
                    }
                };

            // Derived flags.
            if matches!(verified.bundle.class, PolicyBundleV1Class::Emergency) {
                emergency_mode = true;
            }

            if let Some(max_age) = env_u64(ENV_POLICY_MAX_AGE_SECS) {
                let age = Utc::now() - verified.bundle.created_at;
                if age > Duration::seconds(max_age as i64) {
                    safe_mode = true;
                    warnings.push(format!(
                        "governance: policy bundle is stale (age_sec={}); safe mode enabled",
                        age.num_seconds()
                    ));
                }
            }

            let st = GovernancePolicyState {
                emergency_mode,
                safe_mode,
                bundle_hash: Some(verified.bundle_hash.clone()),
                bundle_class: Some(verified.bundle.class.clone()),
                created_at: Some(verified.bundle.created_at),
                warnings,
            };

            let _ = VERIFIED.set(verified);
            let _ = STATE.set(st);

            // Small, auditable log hint.
            let _ = service_name;
            Ok(())
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum EnforcementMode {
    Off,
    Warn,
    Enforce,
}

fn read_enforcement_mode() -> EnforcementMode {
    match std::env::var(ENV_POLICY_ENFORCEMENT_MODE)
        .ok()
        .unwrap_or_else(|| "warn".to_string())
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "off" | "0" | "false" | "no" => EnforcementMode::Off,
        "enforce" | "strict" => EnforcementMode::Enforce,
        _ => EnforcementMode::Warn,
    }
}

fn read_quorum_overrides() -> Option<HashMap<PolicyBundleV1Class, usize>> {
    let mut m = HashMap::new();

    if let Some(v) = env_usize(ENV_QUORUM_STANDARD) {
        m.insert(PolicyBundleV1Class::Standard, v);
    }
    if let Some(v) = env_usize(ENV_QUORUM_HIGH_IMPACT) {
        m.insert(PolicyBundleV1Class::HighImpact, v);
    }
    if let Some(v) = env_usize(ENV_QUORUM_EMERGENCY) {
        m.insert(PolicyBundleV1Class::Emergency, v);
    }

    if m.is_empty() {
        None
    } else {
        Some(m)
    }
}

fn env_bool(key: &str) -> Option<bool> {
    std::env::var(key)
        .ok()
        .and_then(|v| match v.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "y" | "on" => Some(true),
            "0" | "false" | "no" | "n" | "off" => Some(false),
            _ => None,
        })
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
