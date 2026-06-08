#![forbid(unsafe_code)]

// This test is intentionally feature-gated, because it requires Docker.
// To run:
//   cargo test -p rhelma-event-kafka --features integration-tests --test integration_kafka_roundtrip_test

#[cfg(not(feature = "integration-tests"))]
#[test]
fn integration_tests_disabled() {
    eprintln!("integration-tests feature is disabled; skipping Docker-based Kafka tests.");
}

#[cfg(feature = "integration-tests")]
mod enabled {
    use std::sync::Arc;
    use std::time::Duration;

    use chrono::Utc;
    use futures::StreamExt;
    use rhelma_event::{
        generate_event_id, purpose, EventEnvelope, EventRequestContext, EventRequestFlags,
        EventSource, EventTraceContext, PolicyMeta, Residency,
    };
    use rhelma_event_kafka::{
        FallibleEventHandler, KafkaConfig, KafkaEventBus, KafkaProducerWrapper, KafkaSubscriber,
    };
    use serde_json::json;
    use tokio::sync::mpsc;

    // You will need these dev-deps behind the feature in Cargo.toml:
    // testcontainers-modules, testcontainers
    use testcontainers_modules::{kafka, testcontainers::runners::AsyncRunner};

    struct TestHandler {
        tx: mpsc::Sender<EventEnvelope>,
    }

    #[async_trait::async_trait]
    impl FallibleEventHandler for TestHandler {
        async fn handle(&self, event: EventEnvelope) -> Result<(), rhelma_event::EventBusError> {
            let _ = self.tx.send(event).await;
            Ok(())
        }
    }

    fn demo_event(topic: &str) -> EventEnvelope {
        let rid = generate_event_id();
        let cid = generate_event_id();

        EventEnvelope {
            event_id: generate_event_id(),
            event_version: 1,
            topic: topic.to_string(),
            key: None,

            timestamp: Utc::now(),
            published_at: Utc::now(),

            source: EventSource {
                service: "rhelma-event-kafka-integration".into(),
                version: "0.0.0-test".into(),
                region: "eu".into(),
            },

            request: EventRequestContext {
                request_id: Some(rid),
                correlation_id: Some(cid),
                tenant_id: Some("t_it".into()),
                user_id: None,
                flags: EventRequestFlags::default(),
            },

            trace: EventTraceContext {
                trace_id: None,
                span_id: None,
                tracestate: None,
                baggage: None,
                parent_span_id: None,
            },

            payload: json!({"k": "v"}),
            payload_type: "application/json".into(),
            schema_ref: "rhelma.demo.integration.v1".into(),

            policy: PolicyMeta::public(purpose::KAFKA),
            residency: Residency::Global,
            encryption: None,

            signature: None,
            hash: None,
        }
    }

    #[tokio::test]
    async fn kafka_roundtrip_produce_consume() {
        // Start Kafka container
        let node = kafka::Kafka::default().start().await.unwrap();
        let brokers = node.brokers();

        let topic = "rhelma.demo.integration.roundtrip";

        let mut cfg = KafkaConfig::default();
        cfg.brokers = brokers;
        cfg.topic_prefix = ""; // use raw topic

        // subscriber
        let (tx, mut rx) = mpsc::channel::<EventEnvelope>(8);
        let handler = Arc::new(TestHandler { tx });
        let mut sub = KafkaSubscriber::new_fallible(cfg.clone(), handler).unwrap();
        sub.subscribe(topic).await.unwrap();

        let shutdown = tokio_util::sync::CancellationToken::new();
        let sub_task = {
            let s = shutdown.clone();
            tokio::spawn(async move {
                let _ = sub.run_with_shutdown(s).await;
            })
        };

        // producer/bus
        let prod = Arc::new(KafkaProducerWrapper::new(cfg).unwrap());
        let bus = KafkaEventBus::new(prod);

        bus.publish(demo_event(topic)).await.unwrap();

        let got = tokio::time::timeout(Duration::from_secs(15), rx.recv())
            .await
            .expect("timeout waiting for consumed event")
            .expect("channel closed");

        assert_eq!(got.topic, topic);

        shutdown.cancel();
        let _ = sub_task.await;
    }
}
