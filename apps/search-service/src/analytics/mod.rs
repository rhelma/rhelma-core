#![forbid(unsafe_code)]

use std::sync::Arc;

use chrono::Utc;
use rhelma_event::{
    generate_event_id, publish_with_observability, purpose, EventBus, EventBusError, EventEnvelope,
    EventRequestContext, EventRequestFlags, EventSource, EventTraceContext, PolicyMeta, Residency,
};
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

/// Analytics publishing errors.
#[derive(Debug, Error)]
pub enum AnalyticsError {
    #[error("event bus error: {0}")]
    /// Variant `Bus`.
    Bus(#[from] EventBusError),

    #[error("serialization error: {0}")]
    /// Variant `Serde`.
    Serde(#[from] serde_json::Error),
}

/// Analytics sink that emits contract-aligned events (v5.2).
///
/// The default wiring in `search-service` uses a **noop** bus so the service can
/// compile and run without Kafka/NATS. In production, pass a real `EventBus`
/// implementation.
#[derive(Clone)]
pub struct AnalyticsSink {
    service_name: String,
    service_version: String,
    region: String,
    bus: Arc<dyn EventBus + Send + Sync>,
}

/// A no-op event bus used for local/dev builds.
pub struct NoopEventBus;

#[async_trait::async_trait]
impl EventBus for NoopEventBus {
    async fn publish(&self, _event: EventEnvelope) -> Result<(), EventBusError> {
        Ok(())
    }
}

/// Helper to construct a noop bus.
///
/// Use this for builds that do not wire Kafka/NATS yet.
pub fn noop_bus() -> Arc<dyn EventBus + Send + Sync> {
    Arc::new(NoopEventBus)
}

impl AnalyticsSink {
    /// Create a new analytics sink.
    pub fn new(
        service_name: impl Into<String>,
        service_version: impl Into<String>,
        region: impl Into<String>,
        bus: Arc<dyn EventBus + Send + Sync>,
    ) -> Self {
        Self {
            service_name: service_name.into(),
            service_version: service_version.into(),
            region: region.into(),
            bus,
        }
    }

    /// Record a search query event.
    ///
    /// This is intentionally **best-effort**: the caller should never block the
    /// search request path on analytics.
    ///
    /// Contract notes (v5.2):
    /// - `request_id` MUST exist; if missing, a v7 UUID is generated.
    /// - `correlation_id` SHOULD exist; if missing, it falls back to `request_id`.
    /// - `trace_id` / `span_id` are forwarded from the inbound RequestContext when present.
    ///
    /// Record a search query analytics event.
    ///
    /// Note: This function intentionally takes many parameters because the analytics schema is flat
    /// and we keep call-sites explicit. Clippy's `too_many_arguments` lint is not actionable here.
    #[allow(clippy::too_many_arguments)]
    pub async fn record_search_query<P: Serialize>(
        &self,
        tenant_id: Option<String>,
        user_id: Option<String>,
        request_id: String,
        correlation_id: String,
        trace_id: Option<String>,
        span_id: Option<String>,
        query: String,
        payload: P,
        total_hits: u64,
    ) -> Result<(), AnalyticsError> {
        let request_id = Some(if request_id.trim().is_empty() {
            Uuid::now_v7().to_string()
        } else {
            request_id
        });

        let correlation_id = Some(if correlation_id.trim().is_empty() {
            request_id
                .clone()
                .unwrap_or_else(|| Uuid::now_v7().to_string())
        } else {
            correlation_id
        });

        let payload_value: Value = serde_json::to_value(payload)?;
        let payload = serde_json::json!({
            "tenant_id": tenant_id,
            "user_id": user_id,
            "query": query,
            "total_hits": total_hits,
            "payload": payload_value,
        });

        let envelope = EventEnvelope {
            event_id: generate_event_id(),
            event_version: 1,

            topic: "rhelma.analytics.search.query".to_string(),
            key: tenant_id.clone().or_else(|| user_id.clone()),

            timestamp: Utc::now(),
            published_at: Utc::now(),

            source: EventSource {
                service: self.service_name.clone(),
                version: self.service_version.clone(),
                region: self.region.clone(),
            },
            request: EventRequestContext {
                request_id,
                correlation_id,
                tenant_id,
                user_id,
                flags: EventRequestFlags::default(),
            },
            trace: EventTraceContext {
                trace_id,
                span_id,
                tracestate: rhelma_tracing::context::current_tracestate(),
                baggage: rhelma_tracing::context::current_baggage(),
                parent_span_id: None,
            },

            payload,
            payload_type: "rhelma.search.query.v5_2".to_string(),
            schema_ref: "rhelma://schemas/analytics/search-query@v5.2".to_string(),

            policy: PolicyMeta::public(purpose::SEARCH_ANALYTICS),
            residency: Residency::Global,
            encryption: None,

            signature: None,
            hash: None,
        };

        publish_with_observability(self.bus.as_ref(), envelope).await?;
        Ok(())
    }
}
