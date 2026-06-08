//! Strongly-typed identifiers for Rhelma Platform v5.1
//!
//! Strong-ID Rules (Rhelma v5.1):
//! - MUST validate shape on construction (parse())
//! - MUST enforce lowercase-only identifiers for tenant/region
//! - MUST NOT leak raw invalid input in error messages
//! - MUST be deterministic, hashable, and stable for storage
//! - new() = unchecked (internal), parse() = safe (external)
//!
//! This module only wires submodules together and re-exports the public API.
//! All concrete identifier types live in `ids.rs`.

pub mod common;
pub mod ids;
pub mod pagination;
pub mod rate_limit;

// Re-export canonical strong ID types
pub use ids::*;

// Re-export pagination helpers
pub use pagination::*;

// Re-export the canonical rate limit key builder
pub use rate_limit::RateLimitKeyBuilder;
