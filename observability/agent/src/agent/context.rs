//! system_context.rs — Rhelma v5.2 unified system RequestContext builder
//!
//! All system-generated events (heartbeat, anomaly, audit, incident, command results)
//! must use a consistent Rhelma RequestContext. This module provides that builder.

use crate::agent::config::ResidencyMode;
use rhelma_event::{EventRequestContext, EventRequestFlags};

use uuid::Uuid;

/// Build a full Rhelma v5.2 system RequestContext.
///
/// Residency MUST be specified by the caller (typically from config).
///
/// # Arguments
/// * `residency` - Residency mode (used for envelope, not request context in v5.2)
///
/// # Returns
/// System request context
pub fn system_request_context(residency: &ResidencyMode) -> EventRequestContext {
    let _ = residency; // residency is carried on envelope, not request context (v5.2)
    EventRequestContext {
        request_id: Some(Uuid::now_v7().to_string()),
        correlation_id: Some(Uuid::now_v7().to_string()),
        tenant_id: None,
        user_id: None,
        flags: EventRequestFlags {
            system: true,
            ..Default::default()
        },
    }
}

/// Minimal system context for callers who don't care about residency (fallback GLOBAL).
///
/// # Returns
/// System request context with global residency
pub fn system_request_context_global() -> EventRequestContext {
    system_request_context(&ResidencyMode::Global)
}
