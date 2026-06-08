//! rhelma-core v5.1.0 — Core primitives for the Rhelma Platform (Contract v5.1)
//!
//! Contains:
//! - Zero-Trust RequestContext
//! - v5.1 unified error model
//! - DR/SLA-aware tenancy
//! - Observability configuration model
//! - Strong ID & validator types

pub mod config;
pub mod constants;
pub mod environment;
pub mod error;
pub mod error_v52;
pub mod http_error;

pub mod governance;
pub mod multi_region;
pub mod observability;
pub mod prelude;
pub mod problem;
pub mod realtime_types;
pub mod region_health;
pub mod request_context;
pub mod request_context_v52;
pub mod result;
pub mod security;
pub mod tenancy;
pub mod trace_context;
pub mod traits;
pub mod types;

// ---------- Re-exports required by Rhelma Contract ----------

pub use crate::config::AppConfig;
pub use crate::environment::Environment;
pub use crate::error::{ErrorExt, RhelmaError};
pub use crate::error_v52::{
    envelope_from_rhelma_error, envelope_from_status, ErrorEnvelopeV52, ErrorSeverity, ErrorV52,
};
pub use crate::http_error::{HttpErrorBody, HttpErrorMapping};
pub use crate::observability::UnifiedObservabilityConfig;
pub use crate::result::RhelmaResult;
pub use crate::types::RateLimitKeyBuilder;

pub use crate::tenancy::{ResidencyPolicy, TenancyTier, TenantProfile};

pub use crate::request_context::{RequestContext, RequestFlags, RequestResidency};
pub use crate::security::{PasswordPolicy, PasswordStrength};
pub use crate::trace_context::TraceContext;
pub use crate::types::*;

// ---------- Re-export safe utilities (contract-approved only) ----------
pub use chrono::{DateTime, Utc};
pub use uuid::Uuid;
