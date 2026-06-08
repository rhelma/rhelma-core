#![forbid(unsafe_code)]

use std::sync::Arc;

use chrono::Utc;
use rhelma_event::{
    generate_event_id, purpose, EventBus, EventEnvelope, EventRequestContext, EventRequestFlags,
    EventSource, EventTraceContext, PolicyMeta, Residency,
};
use rhelma_event_kafka::{KafkaConfig, KafkaEventBus, KafkaProducerWrapper};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let brokers = std::env::var("BROKERS").unwrap_or_else(|_| "localhost:9092".to_string());
    let topic = std::env::var("TOPIC").unwrap_or_else(|_| "rhelma.demo.roundtrip".to_string());
    let prefix = std::env::var("TOPIC_PREFIX").unwrap_or_else(|_| "rhelma.".to_string());

    let cfg = KafkaConfig {
        brokers,
        topic_prefix: prefix,
        ..Default::default()
    };

    let producer = Arc::new(KafkaProducerWrapper::new(cfg)?);
    let bus = KafkaEventBus::new(producer);

    let event = demo_event(&topic);
    bus.publish(event).await?;

    println!("published one event to topic: {topic}");
    Ok(())
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
            service: "rhelma-event-kafka-example".into(),
            version: "0.0.0-dev".into(),
            region: "eu".into(),
        },

        request: EventRequestContext {
            request_id: Some(rid),
            correlation_id: Some(cid),
            tenant_id: Some("t_demo".into()),
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

        payload: json!({"type": "demo", "ok": true}),
        payload_type: "application/json".into(),
        schema_ref: "rhelma.demo.roundtrip.v1".into(),

        policy: PolicyMeta::public(purpose::KAFKA),
        residency: Residency::Global,
        encryption: None,

        signature: None,
        hash: None,
    }
}
