# Service Runbook: control-service (Rhelma control-plane)

## What this service does

`control-service` is the control-plane for Rhelma.
It stores and serves:

- realms
- nodes
- realm → service routes

`api-gateway` may call `GET /v1/discovery?...` to resolve upstream base URLs.

## Primary signals

- Health: `GET /health` / `GET /healthz`
- Readiness: `GET /readyz` (checks DB)
- Metrics: `GET /metrics`

## Common failure modes

- **DB unavailable**: readiness fails, admin/discovery endpoints return 5xx
- **Bad admin token**: 403s on admin endpoints
- **No heartbeats**: discovery returns empty services because nodes are offline

## Quick triage

```bash
kubectl get pods -n rhelma6 -l app=control-service -o wide
kubectl logs -n rhelma6 -l app=control-service --tail=200
kubectl get events -n rhelma6 --sort-by=.lastTimestamp | tail -n 40
```

### Check dependencies

- Postgres connectivity and migrations
- (Optional) Redis if you integrate shared auth/infra

```bash
kubectl get pods -n rhelma6 | grep -E 'postgres|redis'
```

## Fast recovery actions

### Rollout restart

```bash
kubectl rollout restart deploy/control-service -n rhelma6
kubectl rollout status deploy/control-service -n rhelma6 --timeout=180s
```

### Validate discovery quickly

```bash
# Replace <realm>
curl -s "http://control-service:8086/v1/discovery?realm=<realm>" | jq .
```

## Evidence collection

- capture a failing request_id/correlation_id from logs
- snapshot realm/node/route rows for the affected realm (if safe)
