//! Reflex subsystem (v5.2).
//!
//! Contains the naive anomaly detector and signal contracts used by the agent runtime.

/// Naive anomaly detector utilities.
pub mod anomaly;
/// Signal contracts for the Reflex subsystem.
pub mod signals;

pub use signals::{ReflexDecision, SignalPayload};
