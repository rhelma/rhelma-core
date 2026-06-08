//! Auth event publisher using rhelma-event.
//!
//! If `eventing` feature is disabled, this becomes a no-op publisher.

use chrono::Utc;
use serde::Serialize;
use serde_json::Value;

use crate::error::{AuthError, AuthResult};
use crate::tracing_ext::auth_span;
use crate::types::TenantId;

use super::policy::AuthEventPolicy;
use super::types::*;

#[cfg(feature = "eventing")]
use rhelma_event::{generate_event_id, purpose, EventBus, EventEnvelope, PolicyMeta};

/// Publisher facade.
#[derive(Clone)]
/// struct (documented for contract compliance).
pub struct AuthEventPublisher {
    policy: std::sync::Arc<dyn AuthEventPolicy>,
}

impl AuthEventPublisher {
    /// fn (documented for contract compliance).
    pub fn new(policy: std::sync::Arc<dyn AuthEventPolicy>) -> Self {
        Self { policy }
    }

    #[cfg(feature = "eventing")]
    /// async fn (documented for contract compliance).
    pub async fn publish_login<B: EventBus + Send + Sync>(
        &self,
        bus: &B,
        e: AuthLoginEvent,
    ) -> AuthResult<()> {
        self.publish_generic(
            bus,
            super::topics::TOPIC_AUTH_LOGIN,
            e.tenant_id.as_ref(),
            &e,
        )
        .await
    }

    #[cfg(feature = "eventing")]
    /// async fn (documented for contract compliance).
    pub async fn publish_logout<B: EventBus + Send + Sync>(
        &self,
        bus: &B,
        e: AuthLogoutEvent,
    ) -> AuthResult<()> {
        self.publish_generic(
            bus,
            super::topics::TOPIC_AUTH_LOGOUT,
            e.tenant_id.as_ref(),
            &e,
        )
        .await
    }

    #[cfg(feature = "eventing")]
    /// async fn (documented for contract compliance).
    pub async fn publish_refresh<B: EventBus + Send + Sync>(
        &self,
        bus: &B,
        e: AuthRefreshEvent,
    ) -> AuthResult<()> {
        self.publish_generic(
            bus,
            super::topics::TOPIC_AUTH_REFRESH,
            e.tenant_id.as_ref(),
            &e,
        )
        .await
    }

    #[cfg(feature = "eventing")]
    /// async fn (documented for contract compliance).
    pub async fn publish_session_revoked<B: EventBus + Send + Sync>(
        &self,
        bus: &B,
        e: AuthSessionRevokedEvent,
    ) -> AuthResult<()> {
        self.publish_generic(
            bus,
            super::topics::TOPIC_AUTH_SESSION_REVOKED,
            e.tenant_id.as_ref(),
            &e,
        )
        .await
    }

    #[cfg(feature = "eventing")]
    /// async fn (documented for contract compliance).
    pub async fn publish_oidc_login<B: EventBus + Send + Sync>(
        &self,
        bus: &B,
        e: AuthOidcLoginEvent,
    ) -> AuthResult<()> {
        self.publish_generic(
            bus,
            super::topics::TOPIC_AUTH_OIDC_LOGIN,
            e.tenant_id.as_ref(),
            &e,
        )
        .await
    }

    #[cfg(feature = "eventing")]
    async fn publish_generic<B: EventBus + Send + Sync, T: Serialize>(
        &self,
        bus: &B,
        topic: &str,
        tenant_id: Option<&TenantId>,
        payload: &T,
    ) -> AuthResult<()> {
        let _span = auth_span("auth.publish_event");

        let value: Value = serde_json::to_value(payload).map_err(|_| AuthError::Internal)?;

        let src = self.policy.source();

        let env = EventEnvelope {
            event_id: generate_event_id(),
            event_version: 1,
            topic: topic.to_string(),
            key: None,
            timestamp: Utc::now(),
            published_at: Utc::now(),
            source: src,
            request: self.policy.request_context(tenant_id),
            trace: self.policy.trace_context(),
            payload: value,
            payload_type: "rhelma.auth.EventV1".to_string(),
            schema_ref: format!("{topic}@v1"),
            policy: PolicyMeta::public(purpose::AUTH),
            residency: self.policy.residency(tenant_id),
            encryption: None,
            signature: None,
            hash: None,
        };

        bus.publish(env.finalize_strict().map_err(|_| AuthError::Internal)?)
            .await
            .map_err(|_| AuthError::Internal)
    }

    #[cfg(not(feature = "eventing"))]
    /// async fn (documented for contract compliance).
    pub async fn publish_login<B: Send + Sync>(
        &self,
        _bus: &B,
        _e: AuthLoginEvent,
    ) -> AuthResult<()> {
        Ok(())
    }
    #[cfg(not(feature = "eventing"))]
    /// async fn (documented for contract compliance).
    pub async fn publish_logout<B: Send + Sync>(
        &self,
        _bus: &B,
        _e: AuthLogoutEvent,
    ) -> AuthResult<()> {
        Ok(())
    }
    #[cfg(not(feature = "eventing"))]
    /// async fn (documented for contract compliance).
    pub async fn publish_refresh<B: Send + Sync>(
        &self,
        _bus: &B,
        _e: AuthRefreshEvent,
    ) -> AuthResult<()> {
        Ok(())
    }
    #[cfg(not(feature = "eventing"))]
    /// async fn (documented for contract compliance).
    pub async fn publish_session_revoked<B: Send + Sync>(
        &self,
        _bus: &B,
        _e: AuthSessionRevokedEvent,
    ) -> AuthResult<()> {
        Ok(())
    }
    #[cfg(not(feature = "eventing"))]
    /// async fn (documented for contract compliance).
    pub async fn publish_oidc_login<B: Send + Sync>(
        &self,
        _bus: &B,
        _e: AuthOidcLoginEvent,
    ) -> AuthResult<()> {
        Ok(())
    }
}
