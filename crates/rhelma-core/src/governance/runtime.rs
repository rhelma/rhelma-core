//! Governance runtime configuration derived from environment variables.
//!
//! This module intentionally does **not** depend on `rhelma-config` to avoid cycles.

use crate::error::RhelmaError;
use crate::result::RhelmaResult;

use std::env;
use std::path::{Path, PathBuf};

/// Environment variable: when true, services may activate stricter safety gates.
pub const ENV_EMERGENCY_MODE: &str = "RHELMA_GOVERNANCE_EMERGENCY_MODE";

/// Environment variable: optional path to a policy bundle file (JSON/YAML).
pub const ENV_POLICY_BUNDLE_PATH: &str = "RHELMA_GOVERNANCE_POLICY_BUNDLE_PATH";

/// Environment variable: if true, policy bundle presence becomes a startup requirement.
pub const ENV_POLICY_REQUIRED: &str = "RHELMA_GOVERNANCE_POLICY_REQUIRED";

/// Security policy code used in `RhelmaError::SecurityPolicy` for missing bundles.
pub const POLICY_MISSING_CODE: &str = "governance_policy_missing";

/// A minimal reference to a policy bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyBundleRef {
    /// Filesystem path used to load the bundle.
    pub path: PathBuf,
}

/// Governance runtime derived from environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernanceRuntime {
    emergency_mode: bool,
    policy_bundle_path: Option<PathBuf>,
    policy_required: bool,
}

impl GovernanceRuntime {
    /// Read governance runtime from environment variables.
    ///
    /// Parsing is permissive; invalid booleans are treated as `false`.
    pub fn from_env() -> Self {
        let emergency_mode = env_bool(ENV_EMERGENCY_MODE).unwrap_or(false);
        let policy_required = env_bool(ENV_POLICY_REQUIRED).unwrap_or(false);

        let policy_bundle_path = env::var(ENV_POLICY_BUNDLE_PATH)
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .map(PathBuf::from);

        Self {
            emergency_mode,
            policy_bundle_path,
            policy_required,
        }
    }

    pub fn emergency_mode(&self) -> bool {
        self.emergency_mode
    }

    pub fn policy_required(&self) -> bool {
        self.policy_required
    }

    pub fn policy_bundle_path(&self) -> Option<&Path> {
        self.policy_bundle_path.as_deref()
    }

    /// Validate the configured policy bundle, returning a reference if available.
    ///
    /// - If no bundle path is configured:
    ///   - returns Ok(None) unless `policy_required=true`, in which case it fails.
    /// - If a path is configured:
    ///   - verifies the file exists and is a regular file.
    pub fn validate_policy_bundle(&self) -> RhelmaResult<Option<PolicyBundleRef>> {
        let Some(path) = self.policy_bundle_path.clone() else {
            if self.policy_required {
                return Err(RhelmaError::SecurityPolicy(format!(
                    "{}: {} is required but {} is not set",
                    POLICY_MISSING_CODE, ENV_POLICY_REQUIRED, ENV_POLICY_BUNDLE_PATH
                )));
            }
            return Ok(None);
        };

        let meta = std::fs::metadata(&path).map_err(|e| {
            RhelmaError::SecurityPolicy(format!(
                "{}: cannot read policy bundle at {} ({})",
                POLICY_MISSING_CODE,
                path.display(),
                e
            ))
        })?;

        if !meta.is_file() {
            return Err(RhelmaError::SecurityPolicy(format!(
                "{}: policy bundle path is not a file: {}",
                POLICY_MISSING_CODE,
                path.display()
            )));
        }

        Ok(Some(PolicyBundleRef { path }))
    }
}

fn env_bool(key: &str) -> Option<bool> {
    env::var(key)
        .ok()
        .and_then(|v| match v.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "y" | "on" => Some(true),
            "0" | "false" | "no" | "n" | "off" => Some(false),
            _ => None,
        })
}
