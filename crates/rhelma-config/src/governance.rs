//! Governance-related runtime configuration.
//!
//! This module provides a small, additive config surface for the constitutional
//! governance layer without requiring every service to model it.
//!
//! All fields are **optional** and default to safe, decentralized operation.

use crate::{ConfigError, ConfigResult};
use std::env;

/// Governance runtime config (from environment variables).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernanceRuntimeConfig {
    /// If true, services may activate stricter safety gates.
    pub emergency_mode: bool,
    /// Optional path to a signed Policy Bundle artifact.
    pub policy_bundle_path: Option<String>,
    /// If true, services must refuse to start if policy_bundle_path is missing.
    pub policy_required: bool,
}

impl GovernanceRuntimeConfig {
    /// Non-strict parsing from environment (invalid bools become false).
    pub fn from_env() -> Self {
        Self {
            emergency_mode: env_bool("RHELMA_GOVERNANCE_EMERGENCY_MODE").unwrap_or(false),
            policy_bundle_path: env::var("RHELMA_GOVERNANCE_POLICY_BUNDLE_PATH")
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty()),
            policy_required: env_bool("RHELMA_GOVERNANCE_POLICY_REQUIRED").unwrap_or(false),
        }
    }

    /// Strict parsing from environment.
    pub fn from_env_strict() -> ConfigResult<Self> {
        let emergency_mode = env_bool_strict("RHELMA_GOVERNANCE_EMERGENCY_MODE")?.unwrap_or(false);
        let policy_required =
            env_bool_strict("RHELMA_GOVERNANCE_POLICY_REQUIRED")?.unwrap_or(false);

        Ok(Self {
            emergency_mode,
            policy_bundle_path: env::var("RHELMA_GOVERNANCE_POLICY_BUNDLE_PATH")
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty()),
            policy_required,
        })
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

fn env_bool_strict(key: &'static str) -> ConfigResult<Option<bool>> {
    let Ok(v) = env::var(key) else {
        return Ok(None);
    };
    let t = v.trim().to_ascii_lowercase();
    let parsed = match t.as_str() {
        "1" | "true" | "yes" | "y" | "on" => Some(true),
        "0" | "false" | "no" | "n" | "off" => Some(false),
        _ => None,
    };
    parsed
        .ok_or_else(|| ConfigError::InvalidValue {
            field: key,
            message: format!("expected boolean, got `{}`", v),
        })
        .map(Some)
}
