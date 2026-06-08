#![forbid(unsafe_code)]

//! Event publishing helpers for API Gateway.
//!
//! This module is intentionally **best-effort**:
//! - If Kafka is disabled (brokers="noop") we fall back to a no-op bus.
//! - Kafka support is behind the `kafka-events` feature so default builds stay slim.

use std::sync::Arc;

use chrono::Utc;
use rhelma_event::contracts::obs::{RegionFailoverEvent, RegionHealthEvent};
use rhelma_event::{publish_with_observability, EventBus, EventBusError, EventEnvelope};

/// A no-op event bus for local/dev runs.
pub struct NoopEventBus;

#[async_trait::async_trait]
impl EventBus for NoopEventBus {
    async fn publish(&self, _event: EventEnvelope) -> Result<(), EventBusError> {
        Ok(())
    }
}

/// Construct an event bus for the gateway.
///
/// - When brokers are `noop`, returns a NoopEventBus.
/// - With feature `kafka-events`, returns a KafkaEventBus.
pub fn build_event_bus(
    service_name: &str,
    kafka_brokers: &str,
    kafka_topic_prefix: &str,
) -> Arc<dyn EventBus> {
    let brokers = kafka_brokers.trim();
    if brokers.is_empty() || brokers.eq_ignore_ascii_case("noop") {
        return Arc::new(NoopEventBus);
    }

    #[cfg(feature = "kafka-events")]
    {
        use rhelma_event_kafka::{KafkaConfig, KafkaEventBus, KafkaProducerWrapper};

        let kafka_cfg = KafkaConfig {
            brokers: brokers.to_string(),
            group_id: format!("{service_name}-publisher"),
            topic_prefix: kafka_topic_prefix.trim().to_string(),
            ..Default::default()
        };

        match KafkaProducerWrapper::new(kafka_cfg) {
            Ok(producer) => {
                let bus = KafkaEventBus::new(Arc::new(producer));
                return Arc::new(bus);
            }
            Err(e) => {
                tracing::warn!(error=%e, "api-gateway: failed to initialize Kafka producer; falling back to NoopEventBus");
                return Arc::new(NoopEventBus);
            }
        }
    }

    #[cfg(not(feature = "kafka-events"))]
    {
        tracing::warn!(
            service_name = %service_name,
            kafka_topic_prefix = %kafka_topic_prefix,
            "api-gateway: RHELMA_GATEWAY_KAFKA_BROKERS is set but the binary was built without feature 'kafka-events'; using NoopEventBus"
        );
        Arc::new(NoopEventBus)
    }
}

/// Best-effort publisher for gateway routing/health events.
#[derive(Clone)]
pub struct GatewayEventPublisher {
    enabled: bool,
    service: String,
    observed_region: String,
    bus: Arc<dyn EventBus>,
}

impl GatewayEventPublisher {
    pub fn new(
        enabled: bool,
        service: String,
        observed_region: String,
        bus: Arc<dyn EventBus>,
    ) -> Self {
        Self {
            enabled,
            service,
            observed_region,
            bus,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub async fn publish_region_health(
        &self,
        target_region: &str,
        is_healthy: bool,
        healthy_endpoints: u8,
        latency_ms: u32,
        health_path: Option<String>,
    ) {
        if !self.enabled {
            return;
        }

        let ev = RegionHealthEvent {
            service: self.service.clone(),
            observed_region: self.observed_region.clone(),
            target_region: target_region.to_string(),
            checked_at: Utc::now(),
            is_healthy,
            healthy_endpoints,
            latency_ms,
            health_path,
        };

        match ev.into_envelope() {
            Ok(env) => {
                if let Err(e) = publish_with_observability(self.bus.as_ref(), env).await {
                    tracing::debug!(error=%e, "api-gateway: publish_region_health failed");
                }
            }
            Err(e) => tracing::debug!(error=%e, "api-gateway: region_health envelope build failed"),
        }
    }

    pub async fn publish_failover(
        &self,
        upstream_service: &str,
        request_id: &str,
        correlation_id: Option<&str>,
        from_region: &str,
        to_region: &str,
        reason: &str,
    ) {
        if !self.enabled {
            return;
        }

        let ev = RegionFailoverEvent {
            service: self.service.clone(),
            observed_region: self.observed_region.clone(),
            upstream_service: upstream_service.to_string(),
            request_id: Some(request_id.to_string()),
            correlation_id: correlation_id.map(|s| s.to_string()),
            from_region: from_region.to_string(),
            to_region: to_region.to_string(),
            reason: reason.to_string(),
            occurred_at: Utc::now(),
        };

        match ev.into_envelope() {
            Ok(env) => {
                if let Err(e) = publish_with_observability(self.bus.as_ref(), env).await {
                    tracing::debug!(error=%e, "api-gateway: publish_failover failed");
                }
            }
            Err(e) => {
                tracing::debug!(error=%e, "api-gateway: region_failover envelope build failed")
            }
        }
    }
}
