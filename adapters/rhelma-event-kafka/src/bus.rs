#![forbid(unsafe_code)]

use async_trait::async_trait;
use rhelma_event::{EventBus, EventBusError, EventEnvelope};
use std::sync::Arc;

use crate::producer::KafkaProducerWrapper;

/// Kafka-backed EventBus implementation
pub struct KafkaEventBus {
    /// Field `producer`.
    pub producer: Arc<KafkaProducerWrapper>,
}

impl KafkaEventBus {
    pub fn new(producer: Arc<KafkaProducerWrapper>) -> Self {
        Self { producer }
    }
}

#[async_trait]
impl EventBus for KafkaEventBus {
    async fn publish(&self, event: EventEnvelope) -> Result<(), EventBusError> {
        // Contract v5.2: enforce at the publish boundary (fail-fast).
        let event = event.finalize_publish_boundary()?;

        let key = event
            .key
            .clone()
            .unwrap_or_else(|| event.source.service.clone());
        // topic is expected without prefix; producer resolves prefix
        self.producer.send(&event.topic, &event, &key).await
    }
}
