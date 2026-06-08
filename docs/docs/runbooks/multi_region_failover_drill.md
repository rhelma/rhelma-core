# Multi-Region Failover Drill

This runbook describes a repeatable way to validate **multi-region failover**
behavior end-to-end in a dev/staging environment.

## Scope

The drill covers:

1. `region-health-aggregator` probes regional endpoints and publishes a health snapshot.
2. `api-gateway` (optional) polls the aggregator and updates its `MultiRegionRouter`.
3. Route selection fails over when the primary region becomes unhealthy and fails back when it recovers.

## Prerequisites

- Two region endpoints (can be two deployed gateways or two simple HTTP mock services) exposing `/healthz`.
- `region-health-aggregator` running with a JSON routing config.
- (Optional) `api-gateway` with `RHELMA_GATEWAY_REGION_ROUTING_AGGREGATOR_URL` configured.

## Minimal config

Example routing config used by both aggregator and gateway:

```json
{
  "regions": [
    {"id": "eu-west-1", "priority": 1, "endpoints": ["http://REGION_A"]},
    {"id": "us-east-1", "priority": 2, "endpoints": ["http://REGION_B"]}
  ],
  "failover": {"retry_count": 1, "failback_cooldown_sec": 0, "min_healthy_endpoints": 1}
}
```

## Validation steps

### 1) Verify aggregator snapshot

```bash
curl -sS http://RHA_HOST:8097/v1/regions/health | jq
```

### 2) Verify routing selection

```bash
curl -sS "http://RHA_HOST:8097/v1/route?residency=global" | jq
```

Expected: `selected_region` is the primary region when healthy.

### 3) Trigger failover

Make the primary region's `/healthz` fail (simulate outage), then wait for one health interval.

Re-check the route:

```bash
curl -sS "http://RHA_HOST:8097/v1/route?residency=global" | jq
```

Expected: `selected_region` switches to the secondary region.

### 4) Trigger failback

Recover the primary region, wait for one health interval, then re-check the route.

Expected: `selected_region` switches back to the primary region.

## Load test (k6)

Run a small load profile against the aggregator API:

```bash
RHA_URL=http://RHA_HOST:8097 \
  k6 run benchmarks/k6/region_health_aggregator_load.js
```

You can also run the scenario config:

```bash
# If your tooling reads scenario YAMLs, use:
cat benchmarks/k6/scenarios/region_health_aggregator_failover.yml
```
