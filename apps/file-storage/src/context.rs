//! Request-scoped context for the file-storage service.
//
//! This mirrors the platform-wide Rhelma context model but is kept local
//! to the service for now. Once `rhelma-core` exposes a canonical
//! `RequestContext` type, this module can be replaced with a thin
//! re-export.

use rhelma_core::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Canonical per-request context shared across layers.
///
/// This type is transport-agnostic and can be attached to HTTP, gRPC,
/// or background jobs. Higher layers (Axum middleware, gateway) are
/// responsible for constructing it from headers, tokens and tracing
/// context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    /// Stable request identifier (UUIDv7).
    pub request_id: Uuid,

    /// Cross-service correlation id (can be from client or gateway).
    pub correlation_id: Option<String>,

    /// Multi-tenant identifier (validated).
    pub tenant_id: Option<TenantId>,

    /// Logical region for data residency / routing.
    pub region: Option<RegionId>,

    /// Authenticated user id (business-level), when available.
    pub user_id: Option<UserId>,

    /// Application-level session id (web/app session).
    pub session_id: Option<String>,
}

impl Default for RequestContext {
    fn default() -> Self {
        Self {
            request_id: Uuid::now_v7(),
            correlation_id: None,
            tenant_id: None,
            region: None,
            user_id: None,
            session_id: None,
        }
    }
}

impl RequestContext {
    /// Creates a new context with a fresh request id and no other data.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_tenant(mut self, tenant: TenantId) -> Self {
        self.tenant_id = Some(tenant);
        self
    }

    pub fn with_region(mut self, region: RegionId) -> Self {
        self.region = Some(region);
        self
    }

    pub fn with_user(mut self, user: UserId) -> Self {
        self.user_id = Some(user);
        self
    }

    pub fn with_correlation_id<S: Into<String>>(mut self, id: S) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    pub fn with_session_id<S: Into<String>>(mut self, id: S) -> Self {
        self.session_id = Some(id.into());
        self
    }
}
