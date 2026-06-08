//! Event policy for auth event envelopes.
//!
//! Contract:
//! - Residency: derived from tenant+region and configured policy.
//! - Service name/region must be injected from higher layer.

use serde_json::Value;

use rhelma_core::{RegionId, TenantId};

#[cfg(feature = "eventing")]
use rhelma_event::{
    EventRequestContext, EventRequestFlags, EventResidency, EventSource, EventTraceContext,
};

#[cfg(feature = "eventing")]
use uuid::Uuid;

/// Decide how to build event envelope fields (residency/source/request/trace).
pub trait AuthEventPolicy: Send + Sync {
    /// fn `source`.
    fn source(&self) -> EventSource;
    /// fn `residency`.
    fn residency(&self, tenant_id: Option<&TenantId>) -> EventResidency;

    /// fn `request_context`.
    fn request_context(&self, tenant_id: Option<&TenantId>) -> EventRequestContext {
        EventRequestContext {
            // Rhelma v5.2 contract: uuidv7 strings
            request_id: Some(Uuid::now_v7().to_string()),
            correlation_id: Some(Uuid::now_v7().to_string()),
            tenant_id: tenant_id.map(|t| t.0.clone()),
            user_id: None,
            // Default posture: these are internal auth control-plane events.
            flags: EventRequestFlags {
                system: true,
                ai_safe: true,
                read_only: true,
            },
        }
    }

    /// fn `trace_context`.
    fn trace_context(&self) -> EventTraceContext {
        // Prefer the current tracing context, but always ensure a valid pair.
        // rhelma-tracing guarantees canonical lower-hex IDs.
        let trace_id = rhelma_tracing::context::current_trace_id();
        let span_id = rhelma_tracing::context::current_span_id();
        let tracestate = rhelma_tracing::context::current_tracestate();
        let baggage = rhelma_tracing::context::current_baggage();
        EventTraceContext {
            trace_id,
            span_id,
            tracestate,
            baggage,
            parent_span_id: None,
        }
    }

    /// fn `default_meta`.
    fn default_meta(&self) -> Value {
        Value::Null
    }
}

/// Default policy: conservative residency = RegionStrict for tenant-scoped events.
#[derive(Clone)]
/// struct (documented for contract compliance).
pub struct DefaultAuthEventPolicy {
    /// Field `service`.
    pub service: String,
    /// Field `version`.
    pub version: String,
    /// Field `region`.
    pub region: RegionId,
}

impl DefaultAuthEventPolicy {
    /// fn (documented for contract compliance).
    pub fn new(service: impl Into<String>, region: RegionId) -> Self {
        Self {
            service: service.into(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            region,
        }
    }

    /// Explicit version injection (recommended for services that are not the auth crate itself).
    pub fn new_with_version(
        service: impl Into<String>,
        version: impl Into<String>,
        region: RegionId,
    ) -> Self {
        Self {
            service: service.into(),
            version: version.into(),
            region,
        }
    }
}

#[cfg(feature = "eventing")]
impl AuthEventPolicy for DefaultAuthEventPolicy {
    fn source(&self) -> EventSource {
        EventSource {
            service: self.service.clone(),
            version: self.version.clone(),
            region: self.region.0.clone(),
        }
    }

    fn residency(&self, tenant_id: Option<&TenantId>) -> EventResidency {
        // Default rule:
        // - tenant-scoped auth events: RegionStrict (safe)
        // - system/global events: RegionalOnly
        if tenant_id.is_some() {
            EventResidency::RegionStrict
        } else {
            EventResidency::RegionalOnly
        }
    }
}
