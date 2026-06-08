// src/agent/mod.rs

pub mod config;
pub mod context;
pub mod state;

pub use context::system_request_context_global;
pub use state::EffectiveSeverity;
pub use state::ObservabilityAgent;
