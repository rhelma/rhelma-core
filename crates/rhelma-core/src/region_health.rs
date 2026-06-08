#![forbid(unsafe_code)]
//! Multi-region health checking helpers.
//!
//! `rhelma-core` intentionally keeps routing decisions (`MultiRegionRouter`) free of network I/O.
//! This module provides an **optional**, feature-gated health checker that can periodically probe
//! each region's health endpoint and feed results into a `MultiRegionRouter`.
//!
//! Enable with:
//! - `rhelma-core` feature: `region-health`
//!
//! The health checker uses `tokio` + `reqwest` under the hood (both are optional deps).
//! If you don't want networking in `rhelma-core`, keep the feature disabled and implement
//! probing in your application, calling `MultiRegionRouter::mark_health(...)` yourself.

use std::time::Duration;

#[cfg(feature = "region-health")]
use std::sync::Arc;
#[cfg(feature = "region-health")]
use std::time::Instant;

#[cfg(feature = "region-health")]
use crate::multi_region::MultiRegionRouter;

#[cfg(feature = "region-health")]
use reqwest::Client;
#[cfg(feature = "region-health")]
use rhelma_http_observability::reqwest::ReqwestRequestBuilderExt;

/// A single health endpoint to probe.
#[derive(Debug, Clone)]
pub struct HealthEndpoint {
    pub region_id: String,
    /// Full URL to a health endpoint, e.g. `https://eu-west-1.rhelma.example/healthz`.
    pub health_url: String,
}

/// Configuration for periodic health checks.
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    pub interval: Duration,
    pub timeout: Duration,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(10),
            timeout: Duration::from_secs(5),
        }
    }
}

/// Best-effort probe result.
#[derive(Debug, Clone, Copy)]
pub struct ProbeResult {
    pub is_healthy: bool,
    pub latency_ms: u32,
}

/// A simple async health checker that periodically probes endpoints and updates the router.
///
/// This type is only available when the `region-health` feature is enabled.
#[cfg(feature = "region-health")]
#[derive(Clone)]
pub struct HealthChecker {
    client: Client,
    cfg: HealthCheckConfig,
    endpoints: Arc<tokio::sync::RwLock<Vec<HealthEndpoint>>>,
}

#[cfg(feature = "region-health")]
impl HealthChecker {
    /// Create a health checker with a default `reqwest` client.
    pub fn new(cfg: HealthCheckConfig, endpoints: Vec<HealthEndpoint>) -> Self {
        Self {
            client: Client::new(),
            cfg,
            endpoints: Arc::new(tokio::sync::RwLock::new(endpoints)),
        }
    }

    /// Replace the list of endpoints atomically.
    pub async fn set_endpoints(&self, endpoints: Vec<HealthEndpoint>) {
        *self.endpoints.write().await = endpoints;
    }

    /// Run one probe pass and update the router.
    pub async fn probe_once(&self, router: &MultiRegionRouter) {
        let endpoints = self.endpoints.read().await.clone();
        for ep in endpoints {
            let res = self.probe(&ep.health_url).await;
            router.mark_health(&ep.region_id, res.is_healthy, res.latency_ms);
        }
    }

    /// Start background checks. Returns a join handle.
    ///
    /// This is intentionally "fire and forget"; shutdown can be handled by aborting the task.
    pub fn spawn(self, router: Arc<MultiRegionRouter>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(self.cfg.interval);
            loop {
                ticker.tick().await;
                self.probe_once(&router).await;
            }
        })
    }

    async fn probe(&self, url: &str) -> ProbeResult {
        let start = Instant::now();
        let fut = self.client.get(url).with_rhelma_observability().send();

        let resp = tokio::time::timeout(self.cfg.timeout, fut).await;
        let latency_ms = start.elapsed().as_millis().min(u128::from(u32::MAX)) as u32;

        let is_healthy = match resp {
            Ok(Ok(r)) => r.status().is_success(),
            _ => false,
        };

        ProbeResult {
            is_healthy,
            latency_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values_are_reasonable() {
        let cfg = HealthCheckConfig::default();
        assert!(cfg.interval.as_secs() >= 5);
        assert!(cfg.timeout.as_secs() >= 1);
    }

    #[test]
    fn probe_result_is_copy() {
        let _ = ProbeResult {
            is_healthy: true,
            latency_ms: 12,
        };
    }
}
