#![forbid(unsafe_code)]

use chrono::Utc;
use rhelma_core::prelude::RequestContext;
use rhelma_core::request_context::ResidencyPolicy as CtxResidencyPolicy;
use rhelma_event::{
    generate_event_id, publish_with_observability, purpose, EventBus, EventBusError, EventEnvelope,
    EventRequestContext, EventRequestFlags, EventSource, EventTraceContext, PolicyMeta, Residency,
};
use serde_json::{json, Value};
use std::sync::Arc;
use thiserror::Error;

pub struct EventSink {
    service_name: String,
    service_version: String,
    region: String,
    bus: Option<Arc<dyn EventBus + Send + Sync>>,
}

#[derive(Debug, Error)]
pub enum EventSinkError {
    #[error("event bus error: {0}")]
    /// Variant `Event`.
    Event(#[from] EventBusError),
    #[error("generic error: {0}")]
    /// Variant `Generic`.
    Generic(String),
}

impl EventSink {
    pub async fn new(service_name: String, region: String) -> Result<Self, EventSinkError> {
        Ok(Self {
            service_name,
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            region,
            bus: None,
        })
    }

    pub fn with_bus(mut self, bus: Arc<dyn EventBus + Send + Sync>) -> Self {
        self.bus = Some(bus);
        self
    }

    fn source(&self) -> EventSource {
        EventSource::new(
            self.service_name.clone(),
            self.service_version.clone(),
            self.region.clone(),
        )
    }

    fn residency_from_ctx(ctx: &RequestContext) -> Residency {
        // rhelma-core v5.2 ResidencyPolicy:
        //   Global | RegionalPreferred | RegionalStrict
        match ctx.residency().unwrap_or(CtxResidencyPolicy::Global) {
            CtxResidencyPolicy::Global => Residency::Global,
            CtxResidencyPolicy::RegionalPreferred => Residency::RegionalOnly,
            CtxResidencyPolicy::RegionalStrict => Residency::RegionStrict,
        }
    }

    fn trace_from_ctx(ctx: &RequestContext) -> EventTraceContext {
        // ctx.trace() fields are Option<String> in v5.2
        EventTraceContext {
            trace_id: ctx.trace().trace_id.clone(),
            span_id: ctx.trace().span_id.clone(),
            tracestate: rhelma_tracing::context::current_tracestate(),
            baggage: rhelma_tracing::context::current_baggage(),
            parent_span_id: None,
        }
    }

    fn request_from_ctx(
        ctx: &RequestContext,
        user_id: Option<String>,
        tenant_id: Option<String>,
    ) -> EventRequestContext {
        let request_id = ctx.request_id().to_string();
        let correlation_id = ctx
            .correlation_id()
            .map(|s| s.to_string())
            .unwrap_or_else(|| request_id.clone());

        EventRequestContext {
            request_id: Some(request_id),
            correlation_id: Some(correlation_id),
            tenant_id,
            user_id,
            flags: EventRequestFlags {
                system: false,
                ai_safe: ctx.flags().ai_safe_mode,
                read_only: ctx.flags().read_only,
            },
        }
    }

    /// Publish a WS audit-like event (not `ops.audit*` to avoid signature requirements).
    pub async fn publish_ws_audit(
        &self,
        ctx: &RequestContext,
        actor_user_id: Option<String>,
        actor_tenant_id: Option<String>,
        operation: &str,
        room: &str,
        payload_meta: Value,
    ) -> Result<(), EventSinkError> {
        let Some(bus) = self.bus.as_ref() else {
            // No event bus configured => noop
            tracing::debug!("EventSink bus not configured; skipping publish");
            return Ok(());
        };

        let now = Utc::now();

        let payload = json!({
            "actor": actor_user_id,
            "tenant_id": actor_tenant_id,
            "operation": operation,
            "resource_type": "realtime_room",
            "resource_id": room,
            "result": "success",
            "timestamp": now.to_rfc3339(),
            "meta": payload_meta
        });

        let env = EventEnvelope {
            event_id: generate_event_id(),
            event_version: 1,

            topic: "realtime.audit".to_string(),
            key: Some(room.to_string()),

            timestamp: now,
            published_at: now,

            source: self.source(),
            request: Self::request_from_ctx(ctx, actor_user_id, actor_tenant_id),
            trace: Self::trace_from_ctx(ctx),

            payload,
            payload_type: "realtime.audit".to_string(),
            schema_ref: "realtime.audit@v1".to_string(),

            policy: PolicyMeta::public(purpose::REALTIME),
            residency: Self::residency_from_ctx(ctx),
            encryption: None,

            signature: None,
            hash: None,
        };

        publish_with_observability(bus.as_ref(), env).await?;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct NoopEventBus;

#[async_trait::async_trait]
impl EventBus for NoopEventBus {
    async fn publish(&self, event: EventEnvelope) -> Result<(), EventBusError> {
        tracing::debug!(topic = %event.topic, "NOOP EventBus publish (realtime-service)");
        Ok(())
    }
}
