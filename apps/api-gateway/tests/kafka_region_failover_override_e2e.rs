#![forbid(unsafe_code)]
#![cfg(feature = "kafka-events")]

//! Kafka-backed E2E test for API Gateway's multi-region failover override input.
//!
//! This test is **ignored by default** because it requires a running Kafka broker.
//!
//! Run:
//!   RHELMA_KAFKA_BROKERS=localhost:9092 RHELMA_KAFKA_TOPIC_PREFIX=rhelma. \\
//!     cargo test -p api-gateway --features kafka-events -- --ignored --nocapture

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use rhelma_core::multi_region::{FailoverConfig, MultiRegionRouter, RegionEndpoint};
use rhelma_core::ResidencyPolicy;
use rhelma_event::{
    contracts::obs::RegionFailoverEvent, generate_event_id, purpose, EventBus, EventEnvelope,
    EventRequestContext, EventRequestFlags, EventSource, EventTraceContext, PolicyMeta, Residency,
};
use rhelma_event_kafka::{
    CancellationToken, EventHandler, KafkaConfig, KafkaEventBus, KafkaProducerWrapper,
    KafkaSubscriber,
};
use serde_json::json;
use uuid::Uuid;

use api_gateway::region_routing::RegionRoutingHandle;

fn env_or(default: &str, key: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn env_u64_or(default: u64, key: &str) -> u64 {
    match std::env::var(key)
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
    {
        Some(v) if v > 0 => v,
        _ => default,
    }
}

fn kafka_cfg(brokers: &str, prefix: &str, group_id: String, offset: &str) -> KafkaConfig {
    let mut cfg = KafkaConfig::default();
    cfg.brokers = brokers.to_string();
    cfg.topic_prefix = prefix.to_string();
    cfg.group_id = group_id;
    cfg.consumer_auto_offset_reset = offset.to_string();
    // Reduce latency in tests.
    cfg.consumer_poll_timeout_ms = 100;
    cfg
}

fn mk_env(
    topic: &str,
    schema_ref: &str,
    payload: serde_json::Value,
    residency: Residency,
) -> EventEnvelope {
    let now = Utc::now();
    let request_id = Uuid::now_v7().to_string();

    EventEnvelope {
        event_id: generate_event_id(),
        event_version: 52,
        topic: topic.to_string(),
        key: None,
        timestamp: now,
        published_at: now,
        source: EventSource::new("api-gateway-e2e", "0", "local"),
        request: EventRequestContext {
            request_id: Some(request_id.clone()),
            correlation_id: Some(request_id),
            tenant_id: None,
            user_id: None,
            flags: EventRequestFlags {
                system: true,
                ai_safe: true,
                read_only: false,
            },
        },
        trace: EventTraceContext::generate(),
        payload,
        payload_type: "application/json".to_string(),
        schema_ref: schema_ref.to_string(),
        policy: PolicyMeta::public(purpose::OBS),
        residency,
        encryption: None,
        signature: None,
        hash: None,
    }
}

fn make_handle(ttl: Duration) -> Arc<RegionRoutingHandle> {
    let router = Arc::new(MultiRegionRouter::new(
        "eu-west-1",
        FailoverConfig {
            retry_before_failover: 2,
            failback_cooldown_sec: 0,
            min_healthy_endpoints: 1,
        },
    ));

    router.upsert_region(RegionEndpoint {
        region_id: "eu-west-1".to_string(),
        endpoints: vec!["http://eu.example".to_string()],
        priority: 1,
        is_healthy: true,
        latency_ms: 50,
    });
    router.upsert_region(RegionEndpoint {
        region_id: "us-east-1".to_string(),
        endpoints: vec!["http://us.example".to_string()],
        priority: 2,
        is_healthy: true,
        latency_ms: 20,
    });

    Arc::new(RegionRoutingHandle::new(
        router,
        ttl,
        Duration::from_secs(60),
        Some(vec!["search-service".to_string()]),
        Some(vec!["api-gateway-e2e".to_string()]),
    ))
}

#[derive(Clone)]
struct RegionFailoverHandler {
    handle: Arc<RegionRoutingHandle>,
}

#[async_trait]
impl EventHandler for RegionFailoverHandler {
    async fn handle(&self, event: EventEnvelope) {
        if event.topic != RegionFailoverEvent::TOPIC {
            return;
        }
        let Ok(ev) = RegionFailoverEvent::from_envelope(&event) else {
            return;
        };

        // Mirror api-gateway logic: ignore empty/fallback.
        if ev.to_region.trim().is_empty() || ev.to_region == "fallback" {
            return;
        }

        self.handle.note_failover_from_event(
            event.source.service.as_str(),
            ev.upstream_service.as_str(),
            ev.from_region.as_str(),
            ev.to_region.as_str(),
            ev.reason.as_str(),
        );
    }
}

#[tokio::test]
#[ignore]
async fn kafka_failover_event_applies_then_expires_override() {
    let brokers = env_or("localhost:9092", "RHELMA_KAFKA_BROKERS");
    let prefix = env_or("rhelma.", "RHELMA_KAFKA_TOPIC_PREFIX");
    let timeout_sec = env_u64_or(30, "RHELMA_E2E_KAFKA_TIMEOUT_SEC");
    let timeout = Duration::from_secs(timeout_sec);

    // Keep override short so the test completes quickly.
    let override_ttl = Duration::from_millis(900);
    let handle = make_handle(override_ttl);

    // Producer used to inject events.
    let prod_cfg = kafka_cfg(
        &brokers,
        &prefix,
        "api-gateway-e2e-producer".to_string(),
        "latest",
    );
    let producer = Arc::new(KafkaProducerWrapper::new(prod_cfg).expect("kafka producer"));
    let bus = Arc::new(KafkaEventBus::new(producer.clone())) as Arc<dyn EventBus>;

    // Subscriber that applies overrides.
    let mut consumer_cfg = kafka_cfg(
        &brokers,
        &prefix,
        format!("api-gateway-e2e-{}", Uuid::now_v7()),
        "latest",
    );
    consumer_cfg.handler_retry_max_attempts = 1;

    let handler = Arc::new(RegionFailoverHandler {
        handle: handle.clone(),
    });
    let mut sub = KafkaSubscriber::new_fallible(consumer_cfg, handler).expect("subscriber");
    sub.subscribe(RegionFailoverEvent::TOPIC)
        .await
        .expect("subscribe obs.region_failover");

    let shutdown = CancellationToken::new();
    let sub_task = {
        let shutdown = shutdown.clone();
        tokio::spawn(async move {
            let _ = sub.run_with_shutdown(shutdown).await;
        })
    };

    // Inject one failover event.
    let fo = RegionFailoverEvent {
        service: "api-gateway".to_string(),
        observed_region: "eu-west-1".to_string(),
        upstream_service: "search-service".to_string(),
        from_region: "eu-west-1".to_string(),
        to_region: "us-east-1".to_string(),
        reason: "e2e".to_string(),
        at: Utc::now(),
    };
    bus.publish(mk_env(
        RegionFailoverEvent::TOPIC,
        "obs.region_failover@v1",
        json!(fo),
        Residency::RegionalOnly,
    ))
    .await
    .expect("publish region_failover");

    // Wait for override to show up.
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if handle.active_override("search-service").is_some() {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("timeout waiting for override to be applied");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Ensure routing uses the override (non-strict).
    let decision = handle
        .route_for_upstream("search-service", ResidencyPolicy::Global, Some("eu-west-1"))
        .expect("route");
    match decision {
        rhelma_core::multi_region::RouteDecision::Direct(r) => assert_eq!(r.region_id, "us-east-1"),
    }

    // Wait until it expires and is pruned.
    tokio::time::sleep(override_ttl + Duration::from_millis(250)).await;
    assert!(
        handle.active_override("search-service").is_none(),
        "override should expire"
    );

    shutdown.cancel();
    let _ = sub_task.await;
}

#[tokio::test]
#[ignore]
async fn kafka_failover_then_failback_updates_override_then_expires() {
    let brokers = env_or("localhost:9092", "RHELMA_KAFKA_BROKERS");
    let prefix = env_or("rhelma.", "RHELMA_KAFKA_TOPIC_PREFIX");
    let timeout_sec = env_u64_or(30, "RHELMA_E2E_KAFKA_TIMEOUT_SEC");
    let timeout = Duration::from_secs(timeout_sec);

    // Keep override short so the test completes quickly.
    let override_ttl = Duration::from_millis(1200);
    let handle = make_handle(override_ttl);

    // Producer used to inject events.
    let prod_cfg = kafka_cfg(
        &brokers,
        &prefix,
        "api-gateway-e2e-producer".to_string(),
        "latest",
    );
    let producer = Arc::new(KafkaProducerWrapper::new(prod_cfg).expect("kafka producer"));
    let bus = Arc::new(KafkaEventBus::new(producer.clone())) as Arc<dyn EventBus>;

    // Subscriber that applies overrides.
    let mut consumer_cfg = kafka_cfg(
        &brokers,
        &prefix,
        format!("api-gateway-e2e-{}", Uuid::now_v7()),
        "latest",
    );
    consumer_cfg.handler_retry_max_attempts = 1;

    let handler = Arc::new(RegionFailoverHandler {
        handle: handle.clone(),
    });
    let mut sub = KafkaSubscriber::new_fallible(consumer_cfg, handler).expect("subscriber");
    sub.subscribe(RegionFailoverEvent::TOPIC)
        .await
        .expect("subscribe obs.region_failover");

    let shutdown = CancellationToken::new();
    let sub_task = {
        let shutdown = shutdown.clone();
        tokio::spawn(async move {
            let _ = sub.run_with_shutdown(shutdown).await;
        })
    };

    // 1) Inject a failover event to us-east-1.
    let fo = RegionFailoverEvent {
        service: "region-health-aggregator".to_string(),
        observed_region: "eu-west-1".to_string(),
        upstream_service: "search-service".to_string(),
        from_region: "eu-west-1".to_string(),
        to_region: "us-east-1".to_string(),
        reason: "e2e_failover".to_string(),
        at: Utc::now(),
    };
    bus.publish(mk_env(
        RegionFailoverEvent::TOPIC,
        "obs.region_failover@v1",
        json!(fo),
        Residency::RegionalOnly,
    ))
    .await
    .expect("publish region_failover");

    // Wait for override to show up and point to us-east-1.
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if let Some(ov) = handle.active_override("search-service") {
            if ov.to_region == "us-east-1" {
                break;
            }
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("timeout waiting for failover override to be applied");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Ensure routing uses the override (non-strict).
    let decision = handle
        .route_for_upstream("search-service", ResidencyPolicy::Global, Some("eu-west-1"))
        .expect("route");
    match decision {
        rhelma_core::multi_region::RouteDecision::Direct(r) => assert_eq!(r.region_id, "us-east-1"),
    }

    // 2) Inject a "failback" event back to eu-west-1 (updates the override).
    let fb = RegionFailoverEvent {
        service: "region-health-aggregator".to_string(),
        observed_region: "us-east-1".to_string(),
        upstream_service: "search-service".to_string(),
        from_region: "us-east-1".to_string(),
        to_region: "eu-west-1".to_string(),
        reason: "e2e_failback".to_string(),
        at: Utc::now(),
    };
    bus.publish(mk_env(
        RegionFailoverEvent::TOPIC,
        "obs.region_failover@v1",
        json!(fb),
        Residency::RegionalOnly,
    ))
    .await
    .expect("publish region_failback");

    // Wait for override to update to eu-west-1.
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if let Some(ov) = handle.active_override("search-service") {
            if ov.to_region == "eu-west-1" && ov.reason.contains("failback") {
                break;
            }
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("timeout waiting for failback override to be applied");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Wait until it expires and is pruned.
    tokio::time::sleep(override_ttl + Duration::from_millis(250)).await;
    assert!(
        handle.active_override("search-service").is_none(),
        "override should expire"
    );

    shutdown.cancel();
    let _ = sub_task.await;
}
