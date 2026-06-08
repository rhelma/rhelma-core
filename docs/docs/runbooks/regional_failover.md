# Regional Failover Runbook (Rhelma6)

This runbook covers **regional degradation, failover, and failback** procedures.

> Scope: api-gateway routing, regional health signals, and traffic shift procedures.

---

## Severity guidance

- **SEV1**: complete outage of primary region, sustained > 5 minutes
- **SEV2**: partial outage / high error rate or latency, sustained > 10 minutes
- **SEV3**: intermittent issues, transient spikes

---

## Signals to watch

- `Rhelma6RegionUnhealthy{region="..."}`
- `MachGatewayRequestErrorRate` (per region)
- `MachGatewayP95LatencyMs` (per region)
- `rhelma_rha_region_is_healthy{region="..."}` (region-health-aggregator)
- `rhelma_rha_region_latency_ms{region="..."}` (region-health-aggregator)
- `rhelma_rha_global_route_change_total` (region-health-aggregator)
- Kafka consumer lag for routing / health topics

---

## Step 1 — Confirm impact

```bash
# Current gateway status (example endpoint)
curl -s http://api-gateway:8080/v1/health | jq .

# Service health in namespace
kubectl get pods -n rhelma6 -o wide
kubectl top pods -n rhelma6 --sort-by=cpu
```

If only a single service is failing, follow the service-specific runbook first.

---

## Step 2 — Identify whether this is a regional issue

```bash
# Compare error/latency across regions (example: Prometheus)
# (use your dashboard; below is a CLI hint)

kubectl port-forward -n monitoring svc/prometheus 9090:9090
# then query Prometheus UI for per-region panels
```

Cross-check:
- cloud provider status
- DNS health checks
- network connectivity between gateway and upstreams

---

## Step 3 — Automatic failover verification

If multi-region routing is enabled, confirm the system is **publishing** and **consuming**.

First, verify the region-health-aggregator view:

```bash
curl -s http://region-health-aggregator:8097/v1/regions/health | jq .
curl -s "http://region-health-aggregator:8097/v1/route?residency=global" | jq .
```

Then confirm Kafka event flow:

- `obs.region_health`
- `obs.region_failover`

```bash
# Quick sanity (replace with your Kafka tooling)
# kcat -b $KAFKA -t obs.region_health -C -o -5
# kcat -b $KAFKA -t obs.region_failover -C -o -5
```

If events are not flowing, treat as **SEV1/SEV2** and fix Kafka / producers first.

---

## Step 4 — Manual failover (when auto failover is unavailable)

### 4.1 Decide the target region

Pick a region that:
- satisfies residency rules
- has healthy upstreams
- has acceptable latency

### 4.2 Shift routing

> Implementation differs by deployment. Use the mechanism your gateway uses:

**ConfigMap / env driven routing (example):**

```bash
kubectl patch configmap rhelma6-routing -n rhelma6 \
  --patch '{"data":{"primary_region":"us-east-1"}}'

kubectl rollout restart deployment/api-gateway -n rhelma6

# Verify
after=10
sleep $after
curl -s http://api-gateway:8080/v1/routing/status | jq .
```

### 4.3 Validate traffic shift

- error rate drops
- latency stabilizes
- backlog drains

---

## Step 5 — Failback (return to primary)

Failback only when:
- primary region is healthy for **> 10 minutes**
- no active incidents
- you can observe canary safely

**Canary failback example:**

```bash
kubectl patch configmap rhelma6-routing -n rhelma6 \
  --patch '{"data":{"canary_region":"eu-west-1","canary_percent":"10"}}'

# Observe for 10-15 minutes, then raise to 50%, then 100%
```

---

## Post-incident checklist

- [ ] Root cause analysis written
- [ ] Alerts tuned (thresholds + runbook links)
- [ ] Capacity check completed
- [ ] Any manual steps automated

---

## E2E validation (Kafka event input + gateway override)

If you run Kafka locally (or in a test cluster), you can validate the gateway's
event-driven failover override end-to-end with the ignored integration test:

```bash
RHELMA_KAFKA_BROKERS=localhost:9092 \
RHELMA_KAFKA_TOPIC_PREFIX=rhelma. \
cargo test -p api-gateway --features kafka-events -- --ignored --nocapture kafka_failover_event_applies_then_expires_override

# Failover + failback (two events, then TTL expiry)
cargo test -p api-gateway --features kafka-events -- --ignored --nocapture kafka_failover_then_failback_updates_override_then_expires
```

This test publishes `obs.region_failover` and asserts the gateway's
`RegionRoutingHandle` applies an override, uses it for routing, then prunes it
after the TTL.


### Hardening: trusted event sources

In production, api-gateway should only apply failover overrides from **trusted producers**
(e.g. `region-health-aggregator`). Configure:

- `RHELMA_GATEWAY_REGION_ROUTING_FAILOVER_EVENT_SOURCE_ALLOWLIST=region-health-aggregator`

This allowlist is checked against `envelope.source.service` on `obs.region_failover` events.
