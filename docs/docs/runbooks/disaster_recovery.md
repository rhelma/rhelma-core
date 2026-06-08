# Rhelma6 Disaster Recovery

This document covers **practical DR steps** for Rhelma6. Treat it as a living doc and revise after drills and real incidents.

## Suggested RPO/RTO Targets

| Component | RPO | RTO |
|---|---:|---:|
| API Gateway | 0 | 5–10 min |
| Node Registry | 1 min | 5 min |
| Kafka | 0–1 min (depends on replication) | 15 min |
| Value Ledger | 0–1 min | 15 min |
| AI Orchestrator | 0 | 10–15 min |

## Backups & Snapshots

Minimum checklist:

- Kafka: broker storage snapshots and/or mirrored clusters (per environment policy)
- Value Ledger: durable log/snapshot strategy (and federation snapshots)
- Node Registry: backing store snapshots (if applicable)
- Config: IaC + ConfigMaps + Secrets stored securely

## Full Region Outage

### 1) Confirm the outage

- Validate from a second network / provider if possible.
- Confirm whether this is **control-plane** (K8s) or **data-plane** (app) specific.

### 2) Shift traffic to a healthy region

Rhelma6 can perform routing failover at the gateway level. If external traffic uses DNS/GSLB, move the entrypoint.

Example (generic):

1. Lower TTL ahead of time (recommended: 60s).
2. Change DNS record to point to the backup region.
3. Validate with a smoke test.

### 3) Promote dependencies

If you use a primary/replica DB pattern:

- Promote replica in backup region
- Update secrets/connection strings in `rhelma6`

### 4) Bring up critical services

Start in this order:

1. Kafka (or confirm managed Kafka is healthy)
2. Node Registry
3. API Gateway
4. Value Ledger + federation
5. AI Orchestrator

```bash
kubectl get pods -n rhelma6 -o wide
kubectl get deploy -n rhelma6
```

### 5) Validate

- Health endpoints (gateway, node registry)
- Event flow (region health events arriving)
- Auth path (JWT verification)
- Ledger reads/writes

## Partial Outage: Kafka Degraded

### Symptoms

- Consumer lag grows rapidly
- Timeouts in event publish/consume

### Recovery

1. Reduce producer rate (feature flag / backpressure)
2. Scale consumers
3. Restart unhealthy brokers
4. If necessary, temporarily disable non-critical topics

## DR Drill Checklist

- [ ] Quarterly regional failover drill
- [ ] Quarterly Kafka recovery drill
- [ ] Quarterly restore-from-backup validation
- [ ] Validate observability during drills (alerts, dashboards, logs)

## Post-DR

- Document RTO/RPO actuals
- Identify gaps and create action items