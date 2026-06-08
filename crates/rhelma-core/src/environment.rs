//! Canonical execution environment type for Rhelma.
//!
//! IMPORTANT:
//! - This type does NOT read environment variables.
//! - This type does NOT perform parsing from process env.
//! - The source of truth for environment is rhelma-config::CentralEnv.
//! - rhelma-core only defines semantics and type safety.

use serde::{Deserialize, Serialize};

/// Execution environment for a Rhelma service.
///
/// Aligned with Rhelma Contract v5.x.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    /// Variant `Development`.
    Development,
    /// Variant `Staging`.
    Staging,
    /// Variant `Production`.
    Production,
}

impl Environment {
    /// String representation used for logs, metrics and events.
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Development => "development",
            Environment::Staging => "staging",
            Environment::Production => "production",
        }
    }
}

#[test]
fn environment_as_str_is_stable() {
    assert_eq!(Environment::Production.as_str(), "production");
}
