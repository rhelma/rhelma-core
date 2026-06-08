# Service Runbook: rhelma-bridge-drivers (Rhelma6)

## What this service does

rhelma-bridge-drivers is a Rhelma6 component. For intent, configuration, and API/event contracts, see:

- `apps/rhelma-bridge-drivers/README.md`
- `docs/contract/v6.0/specs/`

## Primary signals

- Process health: logs + exit status
- Metrics: if exported (sidecar or embedded), scrape at the configured endpoint
- Logs: look for `request_id`, `correlation_id`, and `trace_id` on error paths

## Common alerts

- Availability: readiness failing or the process crash-looping
- Error spike: elevated failures in logs and/or 5xx for HTTP surfaces
- Latency: p95/p99 regressions (HTTP)
- Dependency issues: Kafka/DB/Redis/downstream services unavailable

## Quick triage

```bash
kubectl get pods -n rhelma6 -l app=rhelma-bridge-drivers -o wide
kubectl logs -n rhelma6 -l app=rhelma-bridge-drivers --tail=200
kubectl get events -n rhelma6 --sort-by=.lastTimestamp | tail -n 40
```

### Check dependencies (if configured)

- Kafka / event bus
- PostgreSQL (if used)
- Redis (if used)
- Any downstream HTTP services

```bash
kubectl get pods -n rhelma6 | grep -Ei 'kafka|postgres|redis'
```

## Fast recovery actions

### Rollout restart

```bash
kubectl rollout restart deploy/rhelma-bridge-drivers -n rhelma6
kubectl rollout status deploy/rhelma-bridge-drivers -n rhelma6 --timeout=180s
```

### Roll back a bad release

```bash
kubectl rollout undo deploy/rhelma-bridge-drivers -n rhelma6
kubectl rollout status deploy/rhelma-bridge-drivers -n rhelma6 --timeout=180s
```

### Mitigate overload

- Scale horizontally:
  ```bash
  kubectl scale deploy/rhelma-bridge-drivers -n rhelma6 --replicas=4
  ```
- If the issue is downstream saturation, reduce traffic at the edge or temporarily tighten rate limits (with approval).

## Evidence collection

- Capture a failing `request_id` / `correlation_id`
- Snapshot the active ConfigMap/Secret used by the deployment
- Export recent logs around the failure window
- If relevant, export a short window of metrics (p95/p99 latency, error rate)
