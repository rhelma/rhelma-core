//! High-level auth flows.
//!
//! These are orchestration helpers so service layers don't duplicate logic.

pub mod flows;

/// use (documented for contract compliance).
pub use flows::{AuthFlows, LoginOidcInput, LoginPasswordInput};
