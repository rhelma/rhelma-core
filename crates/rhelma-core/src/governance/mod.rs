//! Governance runtime helpers (Constitutional layer).
//!
//! Design goals:
//! - **Fail-open by default** (decentralization & operability), but **fail-closed when configured**.
//! - Keep this module **dependency-light** and **additive**.
//! - Provide a single place for apps to enforce:
//!   - policy bundle presence (optional/required)
//!   - emergency mode gates
//!   - audit-friendly structured logs

#![forbid(unsafe_code)]

pub mod bootstrap;
pub mod crypto;
pub mod policy;
pub mod runtime;
pub mod state;

pub use runtime::{GovernanceRuntime, PolicyBundleRef};

pub use policy::{PolicyBundleV1, PolicyBundleV1Class, PolicySignatureV1, VerifiedPolicyBundleV1};
pub use state::{current_policy, current_policy_state, GovernancePolicyState};
