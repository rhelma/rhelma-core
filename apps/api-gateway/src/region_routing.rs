#![forbid(unsafe_code)]

//! Optional multi-region routing support for API Gateway.
//!
//! This module:
//! - Parses a JSON region routing config (regions + failover metadata)
//! - Builds a `rhelma_core::multi_region::MultiRegionRouter`
//! - Starts background health checks (direct probing OR polling an external aggregator)
//! - Optionally subscribes to `obs.region_health` / `obs.region_failover` (Kafka) to update
//!   health and apply upstream-specific failover overrides.
//!
//! The whole feature is **opt-in** via `GatewayConfig.region_routing`.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rhelma_http_observability::reqwest::ReqwestRequestBuilderExt;
use serde::{Deserialize, Serialize};

use rhelma_core::multi_region::{FailoverConfig, MultiRegionRouter, RegionEndpoint, RouteDecision};
use rhelma_core::{ResidencyPolicy, RhelmaError, RhelmaResult};

use crate::config::{GatewayConfig, RegionRoutingConfig};
use crate::error::GatewayError;
use crate::eventing::GatewayEventPublisher;

// -----------------------------------------------------------------------------
// Public handle used by services/routes
// -----------------------------------------------------------------------------

/// A thin wrapper around `MultiRegionRouter` that can also apply best-effort
/// per-upstream failover overrides learned from `obs.region_failover`.
#[derive(Clone)]
pub struct RegionRoutingHandle {
    router: Arc<MultiRegionRouter>,
    overrides: Arc<RwLock<HashMap<String, FailoverOverride>>>,
    override_ttl: Duration,
    override_max_ttl: Duration,
    override_allowlist: Option<HashSet<String>>,
    event_source_allowlist: Option<HashSet<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FailoverOverride {
    pub upstream_service: String,
    pub from_region: String,
    pub to_region: String,
    pub reason: String,
    pub observed_at_ms: u64,
    pub expires_at_ms: u64,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

impl RegionRoutingHandle {
    pub fn new(
        router: Arc<MultiRegionRouter>,
        override_ttl: Duration,
        override_max_ttl: Duration,
        override_allowlist: Option<Vec<String>>,
        event_source_allowlist: Option<Vec<String>>,
    ) -> Self {
        let override_allowlist = override_allowlist.map(|v| {
            v.into_iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<HashSet<_>>()
        });
        let event_source_allowlist = event_source_allowlist.map(|v| {
            v.into_iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<HashSet<_>>()
        });

        Self {
            router,
            overrides: Arc::new(RwLock::new(HashMap::new())),
            override_ttl,
            override_max_ttl,
            override_allowlist,
            event_source_allowlist,
        }
    }

    pub fn router(&self) -> &Arc<MultiRegionRouter> {
        &self.router
    }

    pub fn snapshot(&self) -> HashMap<String, RegionEndpoint> {
        self.router.snapshot()
    }

    /// Convenience: update health/latency for a region.
    pub fn mark_health(&self, region_id: &str, is_healthy: bool, latency_ms: u32) {
        self.router.mark_health(region_id, is_healthy, latency_ms);
    }

    /// Apply an upstream-specific override learned from an external event input.
    ///
    /// This enforces an optional allowlist over the `event_source_service` value
    /// (typically `envelope.source.service`), in addition to the upstream allowlist.
    pub fn note_failover_from_event(
        &self,
        event_source_service: &str,
        upstream_service: &str,
        from_region: &str,
        to_region: &str,
        reason: &str,
    ) {
        if let Some(allow) = &self.event_source_allowlist {
            if !allow.contains(event_source_service) {
                metrics::counter!("rhelma_gateway_region_failover_override_rejected_total", "service" => upstream_service.to_string(), "why" => "untrusted_source").increment(1);
                return;
            }
        }
        self.note_failover(upstream_service, from_region, to_region, reason);
    }

    /// Apply an upstream-specific override (best-effort).
    pub fn note_failover(
        &self,
        upstream_service: &str,
        from_region: &str,
        to_region: &str,
        reason: &str,
    ) {
        // Safety guards.
        if self.override_ttl.is_zero() || self.override_max_ttl.is_zero() {
            metrics::counter!("rhelma_gateway_region_failover_override_rejected_total", "service" => upstream_service.to_string(), "why" => "ttl_zero").increment(1);
            return;
        }

        if let Some(allow) = &self.override_allowlist {
            if !allow.contains(upstream_service) {
                metrics::counter!("rhelma_gateway_region_failover_override_rejected_total", "service" => upstream_service.to_string(), "why" => "not_allowlisted").increment(1);
                return;
            }
        }

        let to_region = to_region.trim();
        if to_region.is_empty() || to_region.eq_ignore_ascii_case("fallback") {
            metrics::counter!("rhelma_gateway_region_failover_override_rejected_total", "service" => upstream_service.to_string(), "why" => "invalid_to_region").increment(1);
            return;
        }

        // Ignore unknown regions to prevent poisoning routing state.
        if !self.router.snapshot().contains_key(to_region) {
            metrics::counter!("rhelma_gateway_region_failover_override_rejected_total", "service" => upstream_service.to_string(), "why" => "unknown_region").increment(1);
            return;
        }

        let observed = now_ms();
        let ttl = self.override_ttl.min(self.override_max_ttl);
        let expires = observed.saturating_add(ttl.as_millis() as u64);

        let ov = FailoverOverride {
            upstream_service: upstream_service.to_string(),
            from_region: from_region.to_string(),
            to_region: to_region.to_string(),
            reason: reason.to_string(),
            observed_at_ms: observed,
            expires_at_ms: expires,
        };

        if let Ok(mut map) = self.overrides.write() {
            map.insert(upstream_service.to_string(), ov);
            metrics::counter!("rhelma_gateway_region_failover_override_applied_total", "service" => upstream_service.to_string()).increment(1);
        }
    }

    pub fn active_override(&self, upstream_service: &str) -> Option<FailoverOverride> {
        let now = now_ms();
        let mut prune = false;

        let ov = self
            .overrides
            .read()
            .ok()
            .and_then(|m| m.get(upstream_service).cloned());

        let ov = match ov {
            Some(ov) if ov.expires_at_ms > now => Some(ov),
            Some(_) => {
                prune = true;
                None
            }
            None => None,
        };

        if prune {
            if let Ok(mut m) = self.overrides.write() {
                m.remove(upstream_service);
            }
        }

        ov
    }

    /// Returns a best-effort snapshot of all currently active overrides.
    ///
    /// Expired entries are pruned during the snapshot.
    pub fn overrides_snapshot(&self) -> Vec<FailoverOverride> {
        let now = now_ms();
        let mut out = Vec::new();

        if let Ok(mut m) = self.overrides.write() {
            m.retain(|_, ov| {
                let alive = ov.expires_at_ms > now;
                if alive {
                    out.push(ov.clone());
                }
                alive
            });
        }

        out
    }

    /// Route with an optional upstream-specific override.
    pub fn route_for_upstream(
        &self,
        upstream_service: &str,
        residency: ResidencyPolicy,
        requested_region: Option<&str>,
    ) -> RhelmaResult<RouteDecision> {
        // Strict residency must never be overridden.
        if residency == ResidencyPolicy::RegionalRequired {
            return self.router.route(residency, requested_region);
        }

        if let Some(ov) = self.active_override(upstream_service) {
            // Prefer the override target region, but still allow fallback if it's unhealthy.
            metrics::counter!("rhelma_gateway_region_failover_override_used_total", "service" => upstream_service.to_string(), "from_region" => ov.from_region.clone(), "to_region" => ov.to_region.clone(), "reason" => ov.reason.clone()).increment(1);

            return self.router.route(
                ResidencyPolicy::RegionalPreferred,
                Some(ov.to_region.as_str()),
            );
        }

        self.router.route(residency, requested_region)
    }
}

// -----------------------------------------------------------------------------
// Config parsing
// -----------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct RegionConfigFile {
    regions: Vec<RegionEntry>,
    #[serde(default)]
    failover: Option<FailoverEntry>,
}

#[derive(Debug, Deserialize)]
struct RegionEntry {
    id: String,
    #[serde(default)]
    priority: Option<u8>,
    endpoints: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
struct FailoverEntry {
    #[serde(default, alias = "retry_before_failover")]
    retry_count: Option<u8>,
    #[serde(default)]
    failback_cooldown_sec: Option<u64>,
    #[serde(default)]
    min_healthy_endpoints: Option<u8>,
}

/// If multi-region routing is enabled, build a handle and start background tasks.
///
/// Returns `Ok(None)` when the feature is disabled.
pub fn spawn_if_enabled(
    cfg: &GatewayConfig,
    http: reqwest::Client,
    publisher: Arc<GatewayEventPublisher>,
) -> Result<Option<Arc<RegionRoutingHandle>>, GatewayError> {
    let Some(rcfg) = cfg.region_routing.clone() else {
        return Ok(None);
    };

    if !rcfg.enabled {
        return Ok(None);
    }

    let parsed: RegionConfigFile = serde_json::from_str(&rcfg.config_json).map_err(|e| {
        GatewayError::from(RhelmaError::Config(format!(
            "invalid RHELMA_GATEWAY_REGION_ROUTING_CONFIG_JSON: {e}"
        )))
    })?;

    let fo = parsed.failover.unwrap_or_default();
    let fo_cfg = FailoverConfig {
        retry_before_failover: fo.retry_count.unwrap_or(3),
        failback_cooldown_sec: fo.failback_cooldown_sec.unwrap_or(300),
        min_healthy_endpoints: fo.min_healthy_endpoints.unwrap_or(1),
    };

    // Primary region defaults to this process region.
    let router = Arc::new(MultiRegionRouter::new(fo_cfg));
    let handle = Arc::new(RegionRoutingHandle::new(
        router.clone(),
        rcfg.failover_override_ttl,
        rcfg.failover_override_max_ttl,
        rcfg.failover_override_upstream_allowlist.clone(),
        rcfg.failover_override_event_source_allowlist.clone(),
    ));

    // Seed regions.
    for r in parsed.regions {
        let endpoints = r
            .endpoints
            .into_iter()
            .map(|e| e.trim().to_string())
            .filter(|e| !e.is_empty())
            .collect::<Vec<_>>();

        if endpoints.is_empty() {
            continue;
        }

        router.upsert_region(RegionEndpoint {
            region_id: r.id.clone(),
            endpoints,
            priority: r.priority.unwrap_or(5),
            is_healthy: true,
            latency_ms: 0,
        });
    }

    // Background health source (poll aggregator OR probe endpoints).
    let router_task = router.clone();
    let publisher_task = publisher.clone();
    let rcfg_task = rcfg.clone();
    let aggregator_url = rcfg.aggregator_url.clone();
    tokio::spawn(async move {
        if let Some(url) = aggregator_url {
            aggregator_poll_loop(router_task, http, rcfg_task, url).await;
        } else {
            health_loop(router_task, http, rcfg_task, publisher_task).await;
        }
    });

    // Optional: consume region events from Kafka.
    if rcfg.event_input_enabled {
        spawn_region_event_input_if_possible(cfg, handle.clone());
    }

    Ok(Some(handle))
}

// -----------------------------------------------------------------------------
// Health polling
// -----------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct AggregatorSnapshot {
    #[serde(default)]
    regions: Vec<AggregatorRegion>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AggregatorRegion {
    region_id: String,
    is_healthy: bool,
    healthy_endpoints: u8,
    latency_ms: u32,
}

async fn aggregator_poll_loop(
    router: Arc<MultiRegionRouter>,
    http: reqwest::Client,
    rcfg: RegionRoutingConfig,
    aggregator_url: String,
) {
    let mut interval = tokio::time::interval(rcfg.health_interval.max(Duration::from_secs(1)));
    let mut last_state: HashMap<String, (bool, u32, u8)> = HashMap::new();

    let base = aggregator_url.trim().trim_end_matches('/').to_string();
    let url = format!("{base}/v1/regions/health");

    loop {
        interval.tick().await;
        let ok = aggregator_poll_once(&router, &http, &rcfg, &url, &mut last_state).await;
        if !ok {
            tracing::debug!(url=%url, "region_routing: aggregator poll failed");
        }
    }
}

async fn aggregator_poll_once(
    router: &Arc<MultiRegionRouter>,
    http: &reqwest::Client,
    rcfg: &RegionRoutingConfig,
    url: &str,
    last_state: &mut HashMap<String, (bool, u32, u8)>,
) -> bool {
    let resp = http
        .get(url)
        .timeout(rcfg.health_timeout)
        .with_rhelma_observability()
        .send()
        .await;

    let Ok(resp) = resp else {
        return false;
    };

    if !resp.status().is_success() {
        return false;
    }

    let parsed = resp.json::<AggregatorSnapshot>().await;
    let Ok(snapshot) = parsed else {
        return false;
    };

    for r in snapshot.regions {
        router.mark_health(&r.region_id, r.is_healthy, r.latency_ms);

        metrics::gauge!("rhelma_gateway_region_latency_ms", "region" => r.region_id.clone())
            .set(r.latency_ms as f64);
        metrics::gauge!("rhelma_gateway_region_healthy_endpoints", "region" => r.region_id.clone())
            .set(r.healthy_endpoints as f64);
        metrics::gauge!("rhelma_gateway_region_is_healthy", "region" => r.region_id.clone())
            .set(if r.is_healthy { 1.0 } else { 0.0 });

        let prev = last_state.get(&r.region_id).cloned();
        let cur = (r.is_healthy, r.latency_ms, r.healthy_endpoints);
        if prev != Some(cur) {
            last_state.insert(r.region_id.clone(), cur);
            tracing::info!(
                region = %r.region_id,
                is_healthy = r.is_healthy,
                healthy_endpoints = r.healthy_endpoints,
                latency_ms = r.latency_ms,
                "region_routing: updated from aggregator"
            );
        }
    }

    true
}

async fn health_loop(
    router: Arc<MultiRegionRouter>,
    http: reqwest::Client,
    rcfg: RegionRoutingConfig,
    publisher: Arc<GatewayEventPublisher>,
) {
    let mut interval = tokio::time::interval(rcfg.health_interval.max(Duration::from_secs(1)));
    let mut last_state: HashMap<String, (bool, u32, u8)> = HashMap::new();

    // Normalize health path.
    let mut path = rcfg.health_path.trim().to_string();
    if path.is_empty() {
        path = "/healthz".to_string();
    }
    if !path.starts_with('/') {
        path = format!("/{path}");
    }

    loop {
        interval.tick().await;

        let snapshot = router.snapshot();
        for (region_id, region) in snapshot {
            let mut healthy = 0u8;
            let mut best_latency_ms: Option<u32> = None;

            for base in &region.endpoints {
                let base = base.trim_end_matches('/');
                let url = format!("{base}{path}");

                let start = Instant::now();
                let resp = http
                    .get(&url)
                    .timeout(rcfg.health_timeout)
                    .with_rhelma_observability()
                    .send()
                    .await;

                let latency_ms = start.elapsed().as_millis().min(u128::from(u32::MAX)) as u32;
                let ok = matches!(resp, Ok(r) if r.status().is_success());

                if ok {
                    healthy = healthy.saturating_add(1);
                    best_latency_ms =
                        Some(best_latency_ms.map_or(latency_ms, |b| b.min(latency_ms)));
                }
            }

            let is_healthy = healthy >= router.config.min_healthy_endpoints;
            let latency_ms = best_latency_ms.unwrap_or(u32::MAX);
            router.mark_health(&region_id, is_healthy, latency_ms);

            metrics::gauge!("rhelma_gateway_region_latency_ms", "region" => region_id.clone())
                .set(latency_ms as f64);
            metrics::gauge!("rhelma_gateway_region_healthy_endpoints", "region" => region_id.clone())
                .set(healthy as f64);
            metrics::gauge!("rhelma_gateway_region_is_healthy", "region" => region_id.clone())
                .set(if is_healthy { 1.0 } else { 0.0 });

            let prev = last_state.insert(region_id.clone(), (is_healthy, latency_ms, healthy));
            let changed = match prev {
                None => true,
                Some((p_ok, p_lat, p_h)) => {
                    p_ok != is_healthy || p_h != healthy || p_lat != latency_ms
                }
            };

            if changed {
                metrics::counter!("rhelma_gateway_region_health_change_total", "region" => region_id.clone(), "healthy" => if is_healthy { "true" } else { "false" }).increment(1);

                if publisher.enabled() {
                    let health_path = Some(path.clone());
                    publisher
                        .publish_region_health(
                            &region_id,
                            is_healthy,
                            healthy,
                            latency_ms,
                            health_path,
                        )
                        .await;
                }

                tracing::info!(
                    region = %region_id,
                    healthy_endpoints = healthy,
                    is_healthy,
                    latency_ms,
                    "region health updated"
                );
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Kafka input (optional)
// -----------------------------------------------------------------------------

fn spawn_region_event_input_if_possible(cfg: &GatewayConfig, handle: Arc<RegionRoutingHandle>) {
    let brokers = cfg.kafka_brokers.trim();
    if brokers.is_empty() || brokers.eq_ignore_ascii_case("noop") {
        tracing::debug!("region_routing: event input enabled but kafka brokers are noop/empty");
        return;
    }

    #[cfg(feature = "kafka-events")]
    {
        use rhelma_event::contracts::obs::{RegionFailoverEvent, RegionHealthEvent};
        use rhelma_event_kafka::{EventHandler, KafkaConfig, KafkaSubscriber};

        #[derive(Clone)]
        struct Handler {
            handle: Arc<RegionRoutingHandle>,
        }

        #[async_trait::async_trait]
        impl EventHandler for Handler {
            async fn handle(&self, event: rhelma_event::EventEnvelope) {
                match event.topic.as_str() {
                    RegionHealthEvent::TOPIC => {
                        if let Ok(ev) = RegionHealthEvent::from_envelope(&event) {
                            self.handle.router.mark_health(
                                &ev.target_region,
                                ev.is_healthy,
                                ev.latency_ms,
                            );
                        }
                    }
                    RegionFailoverEvent::TOPIC => {
                        if let Ok(ev) = RegionFailoverEvent::from_envelope(&event) {
                            // Ignore non-region targets.
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
                    _ => {}
                }
            }
        }

        let kafka_cfg = KafkaConfig {
            brokers: brokers.to_string(),
            group_id: cfg
                .region_routing
                .as_ref()
                .map(|r| r.event_input_group_id.clone())
                .unwrap_or_else(|| format!("{}-region-routing", cfg.service_name)),
            topic_prefix: cfg.kafka_topic_prefix.trim().to_string(),
            ..Default::default()
        };

        let handler = Arc::new(Handler { handle });

        tokio::spawn(async move {
            let mut sub = match KafkaSubscriber::new(kafka_cfg, handler) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(error=%e, "region_routing: failed to init kafka subscriber");
                    return;
                }
            };

            // Subscribe to both topics.
            if let Err(e) = sub
                .subscribe_many([RegionHealthEvent::TOPIC, RegionFailoverEvent::TOPIC])
                .await
            {
                tracing::warn!(error=%e, "region_routing: failed to subscribe to region topics");
                return;
            }

            tracing::info!("region_routing: kafka event input started");
            if let Err(e) = sub.run().await {
                tracing::warn!(error=%e, "region_routing: kafka subscriber stopped");
            }
        });
    }

    #[cfg(not(feature = "kafka-events"))]
    {
        let _ = handle;
        tracing::warn!(
            "region_routing: event input enabled but binary was built without feature 'kafka-events'"
        );
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Json, Router};
    use rhelma_core::Utc;
    use rhelma_event::contracts::obs::RegionFailoverEvent;
    use tokio::sync::RwLock;

    fn make_router() -> Arc<MultiRegionRouter> {
        let router = Arc::new(MultiRegionRouter::new(FailoverConfig {
            retry_before_failover: 2,
            failback_cooldown_sec: 0,
            min_healthy_endpoints: 1,
        }));

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

        router
    }

    #[tokio::test]
    async fn aggregator_poll_updates_router_and_affects_routing() {
        // Arrange: a router with primary eu-west-1 and a secondary us-east-1.
        let router = make_router();
        let http = reqwest::Client::new();

        // A mutable snapshot we can update mid-test.
        let snapshot = Arc::new(RwLock::new(AggregatorSnapshot {
            regions: vec![
                AggregatorRegion {
                    region_id: "eu-west-1".to_string(),
                    is_healthy: false,
                    healthy_endpoints: 0,
                    latency_ms: 999,
                },
                AggregatorRegion {
                    region_id: "us-east-1".to_string(),
                    is_healthy: true,
                    healthy_endpoints: 1,
                    latency_ms: 42,
                },
            ],
        }));

        let app = {
            let snapshot = snapshot.clone();
            Router::new().route(
                "/v1/regions/health",
                get(move || {
                    let snapshot = snapshot.clone();
                    async move {
                        let s = snapshot.read().await;
                        Json(serde_json::json!({"regions": s.regions}))
                    }
                }),
            )
        };

        // Bind ephemeral.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = rx.await;
                })
                .await
                .ok();
        });

        let rcfg = RegionRoutingConfig {
            enabled: true,
            config_json: "{\"regions\":[]}".to_string(),
            health_interval: Duration::from_millis(50),
            health_timeout: Duration::from_secs(2),
            health_path: "/healthz".to_string(),
            aggregator_url: Some(format!("http://{}", addr)),
            event_input_enabled: false,
            event_input_group_id: "unit-test".to_string(),
            failover_override_ttl: Duration::from_secs(60),
            failover_override_upstream_allowlist: None,
            failover_override_event_source_allowlist: None,
            failover_override_max_ttl: Duration::from_secs(600),
        };

        let mut last_state: HashMap<String, (bool, u32, u8)> = HashMap::new();
        let url = format!("http://{}/v1/regions/health", addr);

        // Act: poll once.
        let ok = aggregator_poll_once(&router, &http, &rcfg, &url, &mut last_state).await;
        assert!(ok);

        // Assert: primary is unhealthy -> Global should failover to us-east-1.
        let decision = router
            .route(ResidencyPolicy::GlobalPreferred, None)
            .unwrap();
        match decision {
            RouteDecision::Direct(ep) => assert_eq!(ep.region_id, "us-east-1"),
        }

        // Update snapshot to recover primary.
        {
            let mut s = snapshot.write().await;
            for r in &mut s.regions {
                if r.region_id == "eu-west-1" {
                    r.is_healthy = true;
                    r.healthy_endpoints = 1;
                    r.latency_ms = 10;
                }
            }
        }

        let ok2 = aggregator_poll_once(&router, &http, &rcfg, &url, &mut last_state).await;
        assert!(ok2);

        let decision2 = router
            .route(ResidencyPolicy::GlobalPreferred, None)
            .unwrap();
        match decision2 {
            RouteDecision::Direct(ep) => assert_eq!(ep.region_id, "eu-west-1"),
        }

        let _ = tx.send(());
    }

    #[test]
    fn override_is_used_for_non_strict_residency() {
        let router = make_router();
        let handle = RegionRoutingHandle::new(
            router,
            Duration::from_secs(60),
            Duration::from_secs(600),
            None,
            None,
        );

        // Force an override to us-east-1.
        handle.note_failover("search-service", "eu-west-1", "us-east-1", "unit-test");

        let decision = handle
            .route_for_upstream(
                "search-service",
                ResidencyPolicy::GlobalPreferred,
                Some("eu-west-1"),
            )
            .expect("route should succeed");

        match decision {
            RouteDecision::Direct(r) => assert_eq!(r.region_id, "us-east-1"),
        }
    }

    #[test]
    fn strict_residency_is_never_overridden() {
        let router = make_router();
        let handle = RegionRoutingHandle::new(
            router,
            Duration::from_secs(60),
            Duration::from_secs(600),
            None,
            None,
        );
        handle.note_failover("search-service", "eu-west-1", "us-east-1", "unit-test");

        let decision = handle
            .route_for_upstream(
                "search-service",
                ResidencyPolicy::RegionalRequired,
                Some("eu-west-1"),
            )
            .expect("strict route should succeed");

        match decision {
            RouteDecision::Direct(r) => assert_eq!(r.region_id, "eu-west-1"),
        }
    }

    #[test]
    fn expired_override_is_pruned() {
        let router = make_router();
        let handle = RegionRoutingHandle::new(
            router,
            Duration::from_secs(60),
            Duration::from_secs(600),
            None,
            None,
        );

        // Insert an already-expired override directly.
        let ov = FailoverOverride {
            upstream_service: "search-service".to_string(),
            from_region: "eu-west-1".to_string(),
            to_region: "us-east-1".to_string(),
            reason: "expired".to_string(),
            observed_at_ms: 1,
            expires_at_ms: 2,
        };

        {
            let mut m = handle.overrides.write().expect("lock");
            m.insert("search-service".to_string(), ov);
        }

        assert!(handle.active_override("search-service").is_none());
        let m = handle.overrides.read().expect("lock");
        assert!(!m.contains_key("search-service"));
    }

    #[test]
    fn allowlist_blocks_nonlisted_upstream() {
        let router = make_router();
        let handle = RegionRoutingHandle::new(
            router,
            Duration::from_secs(60),
            Duration::from_secs(600),
            Some(vec!["search-service".to_string()]),
            None,
        );

        handle.note_failover("other-service", "eu-west-1", "us-east-1", "unit-test");
        assert!(handle.active_override("other-service").is_none());

        handle.note_failover("search-service", "eu-west-1", "us-east-1", "unit-test");
        assert!(handle.active_override("search-service").is_some());
    }

    #[test]
    fn event_source_allowlist_blocks_untrusted_producer() {
        let router = make_router();
        let handle = RegionRoutingHandle::new(
            router,
            Duration::from_secs(60),
            Duration::from_secs(600),
            None,
            Some(vec!["trusted-producer".to_string()]),
        );

        handle.note_failover_from_event(
            "untrusted-producer",
            "search-service",
            "eu-west-1",
            "us-east-1",
            "unit-test",
        );
        assert!(handle.active_override("search-service").is_none());

        handle.note_failover_from_event(
            "trusted-producer",
            "search-service",
            "eu-west-1",
            "us-east-1",
            "unit-test",
        );
        assert!(handle.active_override("search-service").is_some());
    }

    #[test]
    fn override_ttl_is_capped_by_max_ttl() {
        let router = make_router();
        let handle = RegionRoutingHandle::new(
            router,
            Duration::from_secs(600), // requested
            Duration::from_secs(5),   // cap
            None,
            None,
        );

        handle.note_failover("search-service", "eu-west-1", "us-east-1", "unit-test");
        let ov = handle
            .active_override("search-service")
            .expect("override exists");

        let ttl_ms = ov.expires_at_ms.saturating_sub(ov.observed_at_ms);
        assert!(
            ttl_ms <= 5000,
            "ttl should be capped by max_ttl (got {ttl_ms}ms)"
        );
    }

    #[test]
    fn failover_event_envelope_applies_override() {
        let router = make_router();
        let handle = RegionRoutingHandle::new(
            router,
            Duration::from_secs(60),
            Duration::from_secs(600),
            None,
            None,
        );

        let ev = RegionFailoverEvent {
            service: "api-gateway".to_string(),
            observed_region: "eu-west-1".to_string(),
            upstream_service: "search-service".to_string(),
            from_region: "eu-west-1".to_string(),
            to_region: "us-east-1".to_string(),
            reason: "unit-test".to_string(),
            occurred_at: Utc::now(),
            request_id: None,
            correlation_id: None,
        };

        let env = ev.clone().into_envelope().expect("envelope");
        let parsed = RegionFailoverEvent::from_envelope(&env).expect("parse");
        handle.note_failover(
            parsed.upstream_service.as_str(),
            parsed.from_region.as_str(),
            parsed.to_region.as_str(),
            parsed.reason.as_str(),
        );

        assert!(handle.active_override("search-service").is_some());
    }
}
