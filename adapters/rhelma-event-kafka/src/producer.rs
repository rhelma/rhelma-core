#![forbid(unsafe_code)]

use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};

use rhelma_event::{EventBusError, EventEnvelope};

use crate::config::KafkaConfig;
use crate::headers::kafka_headers_from_envelope_prefer_current_otel;
use crate::metrics;
use crate::topic::resolve_topic;

/// Kafka producer wrapper for rhelma-event
pub struct KafkaProducerWrapper {
    /// Field `producer`.
    pub producer: FutureProducer,
    /// Field `cfg`.
    pub cfg: KafkaConfig,
}

impl KafkaProducerWrapper {
    pub fn new(cfg: KafkaConfig) -> Result<Self, EventBusError> {
        cfg.validate_for_producer()?;
        let mut cc = ClientConfig::new();
        cc.set("bootstrap.servers", &cfg.brokers);
        cc.set("linger.ms", cfg.producer_linger_ms.to_string());
        cc.set("batch.size", cfg.producer_batch_size.to_string());
        cc.set("compression.type", &cfg.producer_compression);

        let producer: FutureProducer = cc
            .create()
            .map_err(|e| EventBusError::Transport(e.to_string()))?;

        Ok(Self { producer, cfg })
    }

    pub async fn send(
        &self,
        topic: &str,
        env: &EventEnvelope,
        key: &str,
    ) -> Result<(), EventBusError> {
        let topic = resolve_topic(self.cfg.topic_prefix.as_str(), topic)?;
        let payload =
            serde_json::to_vec(env).map_err(|e| EventBusError::Serialization(e.to_string()))?;

        metrics::inc_publish();

        let record = FutureRecord {
            topic: &topic,
            payload: Some(&payload),
            key: Some(key),
            partition: None,
            timestamp: Some(env.timestamp.timestamp_millis()),
            headers: Some(kafka_headers_from_envelope_prefer_current_otel(env)),
        };

        self.producer
            .send(record, std::time::Duration::from_secs(0))
            .await
            .map_err(|(e, _)| {
                metrics::inc_publish_error();
                EventBusError::Transport(e.to_string())
            })?;

        Ok(())
    }

    /// Send a raw JSON payload (used for DLQ/quarantine flows).
    pub async fn send_raw(
        &self,
        topic: &str,
        payload: &[u8],
        key: &str,
        ts_millis: i64,
    ) -> Result<(), EventBusError> {
        let topic = resolve_topic(self.cfg.topic_prefix.as_str(), topic)?;

        metrics::inc_publish();

        let record = FutureRecord {
            topic: &topic,
            payload: Some(payload),
            key: Some(key),
            partition: None,
            timestamp: Some(ts_millis),
            headers: None,
        };

        self.producer
            .send(record, std::time::Duration::from_secs(0))
            .await
            .map_err(|(e, _)| {
                metrics::inc_publish_error();
                EventBusError::Transport(e.to_string())
            })?;

        Ok(())
    }
}
