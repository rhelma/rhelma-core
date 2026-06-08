# Service Runbook: API Gateway (Rhelma6)

## What this service does

The API Gateway is the primary ingress for tenant requests. It handles auth flows, per-tenant rate limiting,
request/trace correlation, and multi-region routing decisions.

## Primary signals

- Health: `GET /healthz`, readiness: `GET /readyz`
- Metrics: `GET /metrics` (when enabled)
- Key logs: request_id / correlation_id present on error paths

## Common alerts

- Availability: readiness failing or no healthy pods
- Error spike: elevated 5xx or auth failures (401/403)
- Latency: p95/p99 regressions
- Rate-limit anomalies: sustained 429 spikes
- Multi-region routing: region degraded or frequent failover events

## Quick triage

```bash
kubectl get pods -n rhelma6 -l app=api-gateway -o wide
kubectl logs -n rhelma6 -l app=api-gateway --tail=200
kubectl get events -n rhelma6 --sort-by=.lastTimestamp | tail -n 40
```

### Check dependencies

- Redis (if configured): session/rate-limit state
- Downstream services: auth/search/user/ai services
- Kafka (optional): region health/failover event publishing and routing event input

```bash
kubectl get pods -n rhelma6 | grep -E 'redis|kafka|auth|search|user|ai'
```

## Fast recovery actions

### Rollout restart

```bash
kubectl rollout restart deploy/api-gateway -n rhelma6
kubectl rollout status deploy/api-gateway -n rhelma6 --timeout=180s
```

### Roll back a bad release

```bash
kubectl rollout undo deploy/api-gateway -n rhelma6
kubectl rollout status deploy/api-gateway -n rhelma6 --timeout=180s
```

### Mitigate overload

- Scale horizontally:
  ```bash
  kubectl scale deploy/api-gateway -n rhelma6 --replicas=4
  ```
- Reduce pressure by lowering traffic at the edge (ingress) or tightening rate limits if approved.

## Multi-region routing operational notes

### Configuration modes

1) Direct probing (default): gateway probes each region endpoint.
2) Aggregator polling: `RHELMA_GATEWAY_REGION_ROUTING_AGGREGATOR_URL` is set.
3) Event input: `RHELMA_GATEWAY_REGION_ROUTING_EVENT_INPUT_ENABLED=1` and the build includes the Kafka event consumer.

### Validate routing state

- Look for region health updates in logs.
- Confirm events are being produced:
  ```bash
  kubectl logs -n rhelma6 -l app=api-gateway --tail=300 | grep -Ei 'obs\.region_(health|failover)|region routing'
  ```

### Frequent flaps

If regions alternate frequently:

- verify health endpoint stability in the affected regions
- check for network saturation or packet loss
- consider increasing failback cooldown in the routing config
- if event input is enabled, verify producers are not duplicated with inconsistent values

## Evidence collection

- capture a failing request_id/correlation_id
- export recent logs with the correlation id
- snapshot current config (ConfigMap/Secret) used by the deployment
