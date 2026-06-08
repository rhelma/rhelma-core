#![forbid(unsafe_code)]

//! Multi-region routing primitives.
//!
//! This module is intentionally lightweight and dependency-minimal. It provides a
//! `MultiRegionRouter` that makes **pure routing decisions** based on residency policy,
//! priority, and last-known health/latency. Health checking itself is expected to be
//! implemented by applications (api-gateway, control-plane, etc.) and fed into this router.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use dashmap::DashMap;

use crate::{tenancy::ResidencyPolicy, RhelmaError, RhelmaResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteDecision {
    /// Route directly to a chosen region.
    Direct(RegionEndpoint),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionEndpoint {
    pub region_id: String,
    pub endpoints: Vec<String>,
    /// Lower number = higher priority.
    pub priority: u8,
    pub is_healthy: bool,
    /// Last known latency in ms (best-effort).
    pub latency_ms: u32,
}

#[derive(Debug, Clone)]
pub struct FailoverConfig {
    /// Retry budget before considering failover (used by apps; router uses it as metadata).
    pub retry_before_failover: u8,
    /// Cooldown to avoid rapid failback (used by apps; router uses it as metadata).
    pub failback_cooldown_sec: u64,
    /// Minimum healthy endpoints required for a region to be considered healthy.
    pub min_healthy_endpoints: u8,
}

impl Default for FailoverConfig {
    fn default() -> Self {
        Self {
            retry_before_failover: 3,
            failback_cooldown_sec: 300,
            min_healthy_endpoints: 1,
        }
    }
}

#[derive(Debug)]
pub struct MultiRegionRouter {
    regions: DashMap<String, RegionEndpoint>,
    states: DashMap<String, RegionHealthState>,
    pub config: FailoverConfig,
}

#[derive(Debug, Clone, Copy, Default)]
struct RegionHealthState {
    /// When the region was most recently observed unhealthy (unix ms). Best-effort.
    #[allow(dead_code)]
    last_unhealthy_ms: u64,
    /// When the region recovered (transitioned from unhealthy -> healthy) (unix ms). Best-effort.
    recovered_at_ms: u64,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

impl MultiRegionRouter {
    // FIX: Removed unused `primary_region` parameter
    pub fn new(config: FailoverConfig) -> Self {
        Self {
            regions: DashMap::new(),
            states: DashMap::new(),
            config,
        }
    }

    /// Upserts a region definition.
    pub fn upsert_region(&self, endpoint: RegionEndpoint) {
        let rid = endpoint.region_id.clone();
        self.regions.insert(rid.clone(), endpoint);
        // FIX: Changed from `or_insert_with(RegionHealthState::default)` to `or_default()`
        self.states.entry(rid).or_default();
    }

    /// Bulk replace regions (keeps the router config).
    pub fn replace_regions(&self, regions: HashMap<String, RegionEndpoint>) {
        self.regions.clear();
        self.states.clear();
        for (k, v) in regions {
            self.regions.insert(k.clone(), v);
            // FIX: Changed from `or_insert_with(RegionHealthState::default)` to `or_default()`
            self.states.entry(k).or_default();
        }
    }

    /// Updates last-known health/latency for a region.
    pub fn mark_health(&self, region_id: &str, is_healthy: bool, latency_ms: u32) {
        let now = now_ms();
        let prev_healthy = self
            .regions
            .get(region_id)
            .map(|r| r.is_healthy)
            .unwrap_or(is_healthy);

        if let Some(mut r) = self.regions.get_mut(region_id) {
            r.is_healthy = is_healthy;
            r.latency_ms = latency_ms;
        }

        // FIX: Changed from `or_insert_with(RegionHealthState::default)` to `or_default()`
        let mut state = self.states.entry(region_id.to_string()).or_default();

        if !is_healthy {
            state.last_unhealthy_ms = now;
        } else if !prev_healthy {
            // Transition from unhealthy -> healthy.
            state.recovered_at_ms = now;
        }
    }

    /// Returns the current region map snapshot.
    #[must_use]
    pub fn snapshot(&self) -> HashMap<String, RegionEndpoint> {
        self.regions
            .iter()
            .map(|e| (e.key().clone(), e.value().clone()))
            .collect()
    }

    /// Choose a region based on residency policy and last-known health.
    ///
    /// `requested_region` should typically come from the tenant profile.
    pub fn route(
        &self,
        residency: ResidencyPolicy,
        requested_region: Option<&str>,
    ) -> RhelmaResult<RouteDecision> {
        let requested_region = requested_region.map(|r| r.trim()).filter(|r| !r.is_empty());

        // When a region just recovered, we may avoid immediate failback for `GlobalPreferred` / `RegionalPreferred`.
        let mut exclude_region: Option<&str> = None;

        match residency {
            // FIX: RegionalRequired is the strict policy - no failover allowed
            ResidencyPolicy::RegionalRequired => {
                // Strict: must use the requested region, no failover
                let r = requested_region.ok_or_else(|| {
                    RhelmaError::Config("RegionalRequired requires a requested region".to_string())
                })?;
                let region = self
                    .regions
                    .get(r)
                    .ok_or_else(|| RhelmaError::Config(format!("unknown region: {r}")))?
                    .clone();
                if !region.is_healthy {
                    return Err(RhelmaError::Dependency(format!("region unhealthy: {r}")));
                }
                Ok(RouteDecision::Direct(region))
            }
            // FIX: Combined GlobalPreferred and RegionalPreferred - both allow failover
            ResidencyPolicy::GlobalPreferred | ResidencyPolicy::RegionalPreferred => {
                // Prefer the requested region if healthy; otherwise, failover.
                if let Some(r) = requested_region {
                    if let Some(region) = self.regions.get(r) {
                        if region.is_healthy && self.allow_failback(r) {
                            return Ok(RouteDecision::Direct(region.clone()));
                        }
                        if region.is_healthy {
                            exclude_region = Some(r);
                        }
                    }
                }
                let best = self.best_healthy_region(Some(requested_region), exclude_region)?;
                Ok(RouteDecision::Direct(best))
            }
        }
    }

    fn allow_failback(&self, region_id: &str) -> bool {
        let cooldown_ms = self.config.failback_cooldown_sec.saturating_mul(1000);
        if cooldown_ms == 0 {
            return true;
        }

        let Some(state) = self.states.get(region_id) else {
            return true;
        };

        if state.recovered_at_ms == 0 {
            return true;
        }

        now_ms().saturating_sub(state.recovered_at_ms) >= cooldown_ms
    }

    fn best_healthy_region(
        &self,
        prefer: Option<Option<&str>>,
        exclude: Option<&str>,
    ) -> RhelmaResult<RegionEndpoint> {
        let mut candidates = self
            .regions
            .iter()
            .map(|r| r.value().clone())
            .filter(|r| r.is_healthy)
            .filter(|r| exclude.map(|x| x != r.region_id).unwrap_or(true))
            .collect::<Vec<_>>();

        if candidates.is_empty() {
            return Err(RhelmaError::Dependency("all regions unhealthy".to_string()));
        }

        // If a region just recovered, avoid routing back to it immediately when other healthy
        // options exist. This prevents rapid flapping between a primary region and its failover.
        //
        // Important: if filtering would leave us with *no* candidates, we fall back to the full
        // candidate set so we never fail routing purely due to cooldown.
        let mut cooled = candidates
            .iter()
            .filter(|r| self.allow_failback(&r.region_id))
            .cloned()
            .collect::<Vec<_>>();
        if !cooled.is_empty() {
            candidates = std::mem::take(&mut cooled);
        }

        // Prefer a region if requested and healthy.
        if let Some(Some(pref)) = prefer {
            if let Some(p) = candidates.iter().find(|r| r.region_id == pref) {
                return Ok(p.clone());
            }
        }

        candidates.sort_by(region_sort);
        Ok(candidates[0].clone())
    }
}

fn region_sort(a: &RegionEndpoint, b: &RegionEndpoint) -> Ordering {
    a.priority
        .cmp(&b.priority)
        .then(a.latency_ms.cmp(&b.latency_ms))
        .then(a.region_id.cmp(&b.region_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_router_with_config(cfg: FailoverConfig) -> MultiRegionRouter {
        // FIX: Removed primary_region argument
        let r = MultiRegionRouter::new(cfg);

        r.upsert_region(RegionEndpoint {
            region_id: "eu-west-1".to_string(),
            endpoints: vec!["https://eu-west-1.example".to_string()],
            priority: 1,
            is_healthy: true,
            latency_ms: 50,
        });

        r.upsert_region(RegionEndpoint {
            region_id: "us-east-1".to_string(),
            endpoints: vec!["https://us-east-1.example".to_string()],
            priority: 2,
            is_healthy: true,
            latency_ms: 80,
        });

        r.upsert_region(RegionEndpoint {
            region_id: "ap-south-1".to_string(),
            endpoints: vec!["https://ap-south-1.example".to_string()],
            priority: 3,
            is_healthy: true,
            latency_ms: 120,
        });

        r
    }

    fn make_router() -> MultiRegionRouter {
        make_router_with_config(FailoverConfig::default())
    }

    #[test]
    fn global_prefers_primary_when_healthy() {
        let router = make_router();
        let decision = router
            .route(ResidencyPolicy::GlobalPreferred, None)
            .unwrap();

        match decision {
            RouteDecision::Direct(region) => assert_eq!(region.region_id, "eu-west-1"),
        }
    }

    #[test]
    fn global_fails_over_when_primary_unhealthy() {
        let router = make_router();
        router.mark_health("eu-west-1", false, 999);

        let decision = router
            .route(ResidencyPolicy::GlobalPreferred, None)
            .unwrap();
        match decision {
            RouteDecision::Direct(region) => assert_eq!(region.region_id, "us-east-1"),
        }
    }

    #[test]
    fn global_avoids_immediate_failback_with_cooldown() {
        let cfg = FailoverConfig {
            // Large cooldown to ensure the primary won't be picked immediately after recovery.
            failback_cooldown_sec: 3600,
            ..Default::default()
        };

        let router = make_router_with_config(cfg);

        router.mark_health("eu-west-1", false, 999);
        let decision = router
            .route(ResidencyPolicy::GlobalPreferred, None)
            .unwrap();
        match decision {
            RouteDecision::Direct(region) => assert_eq!(region.region_id, "us-east-1"),
        }

        // Primary recovers, but cooldown is active.
        router.mark_health("eu-west-1", true, 30);
        let decision2 = router
            .route(ResidencyPolicy::GlobalPreferred, None)
            .unwrap();
        match decision2 {
            RouteDecision::Direct(region) => assert_eq!(region.region_id, "us-east-1"),
        }
    }

    #[test]
    fn regional_required_requires_requested_region() {
        let router = make_router();
        let err = router
            .route(ResidencyPolicy::RegionalRequired, None)
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("requires a requested region"), "{msg}");
    }

    #[test]
    fn regional_required_rejects_unhealthy_requested_region() {
        let router = make_router();
        router.mark_health("us-east-1", false, 999);

        let err = router
            .route(ResidencyPolicy::RegionalRequired, Some("us-east-1"))
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("region unhealthy"), "{msg}");
    }

    #[test]
    fn regional_preferred_uses_requested_when_healthy() {
        let router = make_router();

        let decision = router
            .route(ResidencyPolicy::RegionalPreferred, Some("us-east-1"))
            .unwrap();

        match decision {
            RouteDecision::Direct(region) => assert_eq!(region.region_id, "us-east-1"),
        }
    }

    #[test]
    fn regional_preferred_fails_over_when_requested_unhealthy() {
        let router = make_router();
        router.mark_health("us-east-1", false, 999);

        let decision = router
            .route(ResidencyPolicy::RegionalPreferred, Some("us-east-1"))
            .unwrap();

        // Should fail over to primary (eu-west-1) since it is still healthy.
        match decision {
            RouteDecision::Direct(region) => assert_eq!(region.region_id, "eu-west-1"),
        }
    }
}
