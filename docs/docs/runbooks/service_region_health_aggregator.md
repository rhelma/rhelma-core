# Service Runbook: Region Health Aggregator (Rhelma6)

## What this service does

Region Health Aggregator polls each configured region endpoint, computes a region health snapshot, and optionally
publishes `obs.region_health` and synthetic `obs.region_failover` signals.

## Primary signals

- Health: `GET /healthz`
- Snapshot: `GET /v1/regions/health`
- Route debug: `GET /v1/route?residency=<...>&requested_region=<...>`

## Quick triage

```bash
kubectl get pods -n rhelma6 -l app=region-health-aggregator -o wide
kubectl logs -n rhelma6 -l app=region-health-aggregator --tail=200
```

Validate snapshot output:

```bash
kubectl exec -n rhelma6 -it deploy/region-health-aggregator -- sh -lc 'wget -qO- http://127.0.0.1:8097/v1/regions/health'
```

## Common failure modes

- Misconfigured region endpoints (bad URL, wrong health path)
- Timeouts due to network issues
- Kafka disabled or misconfigured when event publishing is expected

## Fast recovery actions

```bash
kubectl rollout restart deploy/region-health-aggregator -n rhelma6
kubectl rollout status deploy/region-health-aggregator -n rhelma6 --timeout=180s
```

## Evidence collection

- capture the snapshot JSON (before and after recovery)
- capture logs with the affected region id
- capture any corresponding gateway failover events during the same window
