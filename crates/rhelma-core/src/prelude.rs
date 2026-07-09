//! Rhelma Core Prelude — v5.1
//!
//! Import in applications as:
//!     use rhelma_core::prelude::*;
//!
//! Exposes ONLY the stable public API surface.
//! Internal modules (TraceContext, security internals, error internals, etc.)
//! MUST NOT be exported here.

//
// ----- Core Config + Constants ------------------------------------------------
//
pub use crate::config::AppConfig;
pub use crate::constants::*;

//
// ----- Error System (stable interface only) ----------------------------------
//
pub use crate::error::{ErrorExt, RhelmaError};
pub use crate::http_error::{HttpErrorBody, HttpErrorMapping};
pub use crate::result::RhelmaResult;

//
// ----- Request Context (Zero-Trust Public API) --------------------------------
//
pub use crate::request_context::{RequestContext, RequestFlags, RequestResidency};

//
// ----- Tenancy + Residency ----------------------------------------------------
//
pub use crate::tenancy::{ResidencyPolicy, TenancyTier, TenantProfile};

//
// ----- Strong Identity + Value Types -----------------------------------------
//
pub use crate::types::{Email, RateLimitKeyBuilder, RegionId, TenantId, UserId, WorkspaceId};

//
// ----- Observability Config (Public + Stable) --------------------------------
//
pub use crate::observability::UnifiedObservabilityConfig;

// ----- Inviroment Config (Public + Stable) --------------------------------
//
pub use crate::Environment;

//
// ----- Developer-friendly Utilities -------------------------------------------
//

// Time utilities
pub use chrono::{DateTime, Utc};

// Serialization helpers
pub use serde::{Deserialize, Serialize};
pub use serde_json::{json, Value as JsonValue};

// UUID type
pub use uuid::Uuid;

// Validation macros
pub use validator::Validate;

// Logging (developer convenience, stable)
pub use crate::security::PasswordPolicy;
pub use tracing::{debug, error, info, trace, warn};
