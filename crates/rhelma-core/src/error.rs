//! Rhelma v5.1 Error Model — Unified Error System

use crate::problem::ProblemDetails;
use anyhow::anyhow;
use thiserror::Error;

// ======================================================================
//  ENUM: RhelmaError
// ======================================================================

#[derive(Error, Debug)]
pub enum RhelmaError {
    // ---------------------
    // Config & Validation
    // ---------------------
    #[error("Configuration error: {0}")]
    /// Variant `Config`.
    Config(String),

    #[error("Validation error: {0}")]
    /// Variant `Validation`.
    Validation(String),

    // ---------------------
    // AuthN / AuthZ
    // ---------------------
    #[error("Authentication error: {0}")]
    /// Variant `Auth`.
    Auth(String),

    #[error("Authorization error: {0}")]
    /// Variant `Authz`.
    Authz(String),

    // ---------------------
    // Domain + Data Layers
    // ---------------------
    #[error("Database error: {0}")]
    /// Variant `Database`.
    Database(String),

    #[error("Cache error: {0}")]
    /// Variant `Cache`.
    Cache(String),

    #[error("Bad request: {0}")]
    /// Variant `BadRequest`.
    BadRequest(String),

    #[error("Not found: {0}")]
    /// Variant `NotFound`.
    NotFound(String),

    #[error("Conflict: {0}")]
    /// Variant `Conflict`.
    Conflict(String),

    #[error("Rate limited: {0}")]
    /// Variant `RateLimited`.
    RateLimited(String),

    #[error("Dependency failure: {0}")]
    /// Variant `Dependency`.
    Dependency(String),

    #[error("Security policy violation: {0}")]
    /// Variant `SecurityPolicy`.
    SecurityPolicy(String),

    #[error("Circuit open: {0}")]
    /// Variant `CircuitOpen`.
    CircuitOpen(String),

    #[error("Distributed transaction error: {0}")]
    /// Variant `DistributedTx`.
    DistributedTx(String),

    // ---------------------
    // INTERNAL
    // ---------------------
    #[error("Internal server error")]
    /// Variant `Internal`.
    Internal,

    // ---------------------
    // Wrapper for Anyhow
    // ---------------------
    #[error(transparent)]
    /// Variant `Other`.
    Other(#[from] anyhow::Error),
}

// ======================================================================
//  STATIC LABELS
// ======================================================================

impl RhelmaError {
    /// Rhelma Contract: residency violations MUST be treated as legal/region governance errors.
    /// We encode residency as a security policy code to avoid breaking enum matches across services.
    pub const RESIDENCY_VIOLATION_CODE: &'static str = "residency_violation";

    /// Construct a residency violation in a stable, machine-detectable format.
    ///
    /// Format: "residency_violation: <detail>"
    pub fn residency_violation(detail: impl Into<String>) -> Self {
        RhelmaError::SecurityPolicy(format!(
            "{}: {}",
            Self::RESIDENCY_VIOLATION_CODE,
            detail.into()
        ))
    }

    /// Extract policy code from a SecurityPolicy message (prefix before ':').
    /// If no code prefix exists, returns None.
    pub fn security_policy_code(&self) -> Option<&str> {
        match self {
            RhelmaError::SecurityPolicy(s) => {
                let t = s.trim();
                let (code, _) = t.split_once(':')?;
                let code = code.trim();
                if code.is_empty() {
                    None
                } else {
                    Some(code)
                }
            }
            _ => None,
        }
    }

    /// True if this error represents a residency violation (contract-mandated HTTP 451).
    pub fn is_residency_violation(&self) -> bool {
        self.security_policy_code() == Some(Self::RESIDENCY_VIOLATION_CODE)
    }

    /// Convert this error into an RFC7807 ProblemDetails structure.
    pub fn to_problem(&self) -> ProblemDetails {
        match self {
            RhelmaError::RateLimited(_) => ProblemDetails {
                type_url: "https://docs.rhelma.dev/problems/rate-limited",
                title: "Rate limited",
                status: 429,
                code: "RHELMA_429_001",
                detail: None,
            },

            RhelmaError::SecurityPolicy(_) if self.is_residency_violation() => ProblemDetails {
                type_url: "https://docs.rhelma.dev/problems/residency-violation",
                title: "Residency violation",
                status: 451,
                code: "RHELMA_451_001",
                detail: None,
            },

            RhelmaError::SecurityPolicy(_) => ProblemDetails {
                type_url: "https://docs.rhelma.dev/problems/security-policy",
                title: "Security policy violation",
                status: 403,
                code: "RHELMA_403_001",
                detail: None,
            },

            _ => ProblemDetails {
                type_url: "https://docs.rhelma.dev/problems/internal",
                title: "Internal error",
                status: 500,
                code: "RHELMA_500_000",
                detail: None,
            },
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            RhelmaError::Config(_) => "config",
            RhelmaError::Validation(_) => "validation",
            RhelmaError::Auth(_) => "auth",
            RhelmaError::Authz(_) => "authz",
            RhelmaError::Database(_) => "database",
            RhelmaError::Cache(_) => "cache",
            RhelmaError::BadRequest(_) => "bad_request",
            RhelmaError::NotFound(_) => "not_found",
            RhelmaError::Conflict(_) => "conflict",
            RhelmaError::RateLimited(_) => "rate_limited",
            RhelmaError::Dependency(_) => "dependency",
            RhelmaError::SecurityPolicy(_) => "security_policy",
            RhelmaError::CircuitOpen(_) => "circuit_open",
            RhelmaError::DistributedTx(_) => "distributed_tx",
            RhelmaError::Internal => "internal",
            RhelmaError::Other(_) => "other",
        }
    }
}

// ======================================================================
// ERROR CONTEXT EXTENSION (v5.1.1 Final)
// ======================================================================

/// Maximum depth of human-readable context chaining for errors.
///
/// Why 3?
/// - Depth > 3 produces unreadable nested messages:
///   "failed (while saving) (while updating) (while syncing) ..."
/// - Depth < 3 loses meaningful diagnostics in multi-layer failures.
/// - SaaS platforms (Stripe, AWS SDK, GCP) معمولاً ۳ تا ۴ سطح دارند.
///
/// Depth 3 → Best balance between readability + diagnostic richness.
///
/// Example with depth = 3:
/// `"not found (while saving) (while updating order)"`.
pub const MAX_CONTEXT_DEPTH: usize = 3;

pub trait ErrorExt: Sized {
    /// fn `rhelma_context`.
    fn rhelma_context<C>(self, context: C) -> Self
    where
        C: std::fmt::Display + Send + Sync + 'static;

    /// fn `context`.
    fn context<C>(self, context: C) -> Self
    where
        C: std::fmt::Display + Send + Sync + 'static,
    {
        self.rhelma_context(context)
    }
}

impl<T> ErrorExt for Result<T, RhelmaError> {
    fn rhelma_context<C>(self, context: C) -> Self
    where
        C: std::fmt::Display + Send + Sync + 'static,
    {
        let new_ctx = context.to_string();

        self.map_err(|err| {
            // internal errors باید raw بمانند
            if matches!(err, RhelmaError::Internal) {
                return RhelmaError::Internal;
            }

            // ergonimic helper
            let apply = |msg: String| merge_context(msg, &new_ctx);

            match err {
                RhelmaError::Config(msg) => RhelmaError::Config(apply(msg)),
                RhelmaError::Validation(msg) => RhelmaError::Validation(apply(msg)),
                RhelmaError::Auth(msg) => RhelmaError::Auth(apply(msg)),
                RhelmaError::Authz(msg) => RhelmaError::Authz(apply(msg)),
                RhelmaError::Database(msg) => RhelmaError::Database(apply(msg)),
                RhelmaError::Cache(msg) => RhelmaError::Cache(apply(msg)),
                RhelmaError::BadRequest(msg) => RhelmaError::BadRequest(apply(msg)),
                RhelmaError::NotFound(msg) => RhelmaError::NotFound(apply(msg)),
                RhelmaError::Conflict(msg) => RhelmaError::Conflict(apply(msg)),
                RhelmaError::RateLimited(msg) => RhelmaError::RateLimited(apply(msg)),
                RhelmaError::Dependency(msg) => RhelmaError::Dependency(apply(msg)),
                RhelmaError::SecurityPolicy(msg) => RhelmaError::SecurityPolicy(apply(msg)),
                RhelmaError::CircuitOpen(msg) => RhelmaError::CircuitOpen(apply(msg)),
                RhelmaError::DistributedTx(msg) => RhelmaError::DistributedTx(apply(msg)),
                RhelmaError::Other(e) => RhelmaError::Other(anyhow!("{} ({})", e, new_ctx)),
                RhelmaError::Internal => RhelmaError::Internal,
            }
        })
    }
}

// ======================================================================
// CONTEXT MERGE HELPERS
// ======================================================================

fn merge_context(msg: String, new_ctx: &str) -> String {
    let depth = msg.matches(" (").count();

    if depth < MAX_CONTEXT_DEPTH {
        format!("{msg} ({new_ctx})")
    } else {
        replace_last_context(&msg, new_ctx)
    }
}

fn replace_last_context(msg: &str, new_ctx: &str) -> String {
    if let Some(pos) = msg.rfind(" (") {
        let before = msg[..pos].trim_end();
        format!("{before} ({new_ctx})")
    } else {
        format!("{msg} ({new_ctx})")
    }
}

// ======================================================================
// TESTS
// ======================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_depth_is_limited() {
        let mut err: Result<(), RhelmaError> = Err(RhelmaError::NotFound("missing".into()));

        err = err.rhelma_context("ctx1");
        err = err.rhelma_context("ctx2");
        err = err.rhelma_context("ctx3");
        err = err.rhelma_context("ctx4"); // should replace last

        let msg = err.unwrap_err().to_string();
        assert_eq!(msg.matches(" (").count(), MAX_CONTEXT_DEPTH);
    }

    #[test]
    fn context_is_added_in_order() {
        let mut err: Result<(), RhelmaError> = Err(RhelmaError::Validation("failed".into()));

        err = err.rhelma_context("step1");
        err = err.rhelma_context("step2");

        let msg = err.unwrap_err().to_string();
        assert!(msg.contains("failed (step1) (step2)"));
    }
}
