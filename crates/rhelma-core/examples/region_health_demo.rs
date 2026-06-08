#![forbid(unsafe_code)]

//! Demo: multi-region routing + optional health checker.
//!
//! Run:
//! ```bash
//! cargo run -p rhelma-core --example region_health_demo --features region-health
//! ```
//!
//! Then set:
//! - `RHELMA_REGION_CONFIG_JSON` (optional; falls back to a tiny built-in config)

// This example is intended to demonstrate the optional `region-health` feature.
// The workspace CI runs clippy without that feature enabled, so we keep a small
// stub `main` to ensure the example still compiles.

#[cfg(not(feature = "region-health"))]
fn main() {
    eprintln!(
        "region_health_demo requires the 'region-health' feature.\n\
Enable it with: cargo run -p rhelma-core --example region_health_demo --features region-health"
    );
}

#[cfg(feature = "region-health")]
mod demo {
    use std::collections::HashMap;
    use std::env;
    use std::sync::Arc;
    use std::time::Duration;

    use rhelma_core::multi_region::{
        FailoverConfig, MultiRegionRouter, RegionEndpoint, RouteDecision,
    };
    use rhelma_core::region_health::{HealthCheckConfig, HealthChecker, HealthEndpoint};
    use rhelma_core::tenancy::ResidencyPolicy;

    #[derive(Debug, serde::Deserialize)]
    struct RegionsConfig {
        regions: Vec<RegionCfg>,
        #[serde(default)]
        failover: Option<FailoverCfg>,
        #[serde(default)]
        health: Option<HealthCfg>,
    }

    #[derive(Debug, serde::Deserialize)]
    struct RegionCfg {
        id: String,
        priority: u8,
        endpoints: Vec<String>,
    }

    #[derive(Debug, serde::Deserialize)]
    struct FailoverCfg {
        #[serde(default = "d_retry")]
        retry_before_failover: u8,
        #[serde(default = "d_cooldown")]
        failback_cooldown_sec: u64,
        #[serde(default = "d_min")]
        min_healthy_endpoints: u8,
    }

    #[derive(Debug, serde::Deserialize)]
    struct HealthCfg {
        #[serde(default = "d_path")]
        path: String,
        #[serde(default = "d_interval")]
        interval_ms: u64,
        #[serde(default = "d_timeout")]
        timeout_ms: u64,
    }

    fn d_retry() -> u8 {
        3
    }
    fn d_cooldown() -> u64 {
        300
    }
    fn d_min() -> u8 {
        1
    }
    fn d_path() -> String {
        "/healthz".to_string()
    }
    fn d_interval() -> u64 {
        10_000
    }
    fn d_timeout() -> u64 {
        5_000
    }

    fn default_json() -> String {
        r#"{
  "regions": [
    {"id":"eu-west-1","priority":1,"endpoints":["http://localhost:8080"]},
    {"id":"us-east-1","priority":2,"endpoints":["http://localhost:8081"]}
  ],
  "failover": { "retry_before_failover": 3, "failback_cooldown_sec": 10, "min_healthy_endpoints": 1 },
  "health": { "path": "/healthz", "interval_ms": 2000, "timeout_ms": 1000 }
}"#
        .to_string()
    }

    pub async fn run() -> anyhow::Result<()> {
        let json = env::var("RHELMA_REGION_CONFIG_JSON").unwrap_or_else(|_| default_json());
        let cfg: RegionsConfig = serde_json::from_str(&json)?;

        let fo = cfg.failover.unwrap_or(FailoverCfg {
            retry_before_failover: d_retry(),
            failback_cooldown_sec: d_cooldown(),
            min_healthy_endpoints: d_min(),
        });

        // MultiRegionRouter::new takes only the config.
        let router = Arc::new(MultiRegionRouter::new(FailoverConfig {
            retry_before_failover: fo.retry_before_failover,
            failback_cooldown_sec: fo.failback_cooldown_sec,
            min_healthy_endpoints: fo.min_healthy_endpoints,
        }));

        // Seed regions.
        let mut map: HashMap<String, RegionEndpoint> = HashMap::new();
        for r in cfg.regions {
            map.insert(
                r.id.clone(),
                RegionEndpoint {
                    region_id: r.id,
                    endpoints: r.endpoints,
                    priority: r.priority,
                    is_healthy: true,
                    latency_ms: 0,
                },
            );
        }
        router.replace_regions(map);

        let health_cfg = cfg.health.unwrap_or(HealthCfg {
            path: d_path(),
            interval_ms: d_interval(),
            timeout_ms: d_timeout(),
        });

        // Build health endpoints from region endpoints.
        let mut eps = Vec::new();
        for (rid, r) in router.snapshot() {
            if let Some(base) = r.endpoints.first() {
                let url = format!("{}{}", base.trim_end_matches('/'), health_cfg.path.as_str());
                eps.push(HealthEndpoint {
                    region_id: rid,
                    health_url: url,
                });
            }
        }

        let checker = HealthChecker::new(
            HealthCheckConfig {
                interval: Duration::from_millis(health_cfg.interval_ms),
                timeout: Duration::from_millis(health_cfg.timeout_ms),
            },
            eps,
        );
        let _jh = checker.clone().spawn(router.clone());

        // Print routing decisions periodically.
        let mut t = tokio::time::interval(Duration::from_secs(2));
        loop {
            t.tick().await;

            let d = router.route(ResidencyPolicy::GlobalPreferred, None)?;
            match d {
                RouteDecision::Direct(r) => {
                    println!(
                        "route=GlobalPreferred -> region={} healthy={} latency_ms={} endpoints={:?}",
                        r.region_id, r.is_healthy, r.latency_ms, r.endpoints
                    );
                }
            }
        }
    }
}

#[cfg(feature = "region-health")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    demo::run().await
}
