# Service Runbook: social-service

## What this service does

`social-service` provides tenant-aware social primitives:

- posts (post/article/link)
- comments
- reactions (like/bookmark)
- latest feed (`GET /feed/latest`)

Write endpoints require `Authorization: Bearer <access-token>` validated via `rhelma-auth`.

## Primary signals

- Health: `GET /health`
- Metrics: `GET /metrics`
- Data: DB read/write latency and error rates

## Common failure modes

- **Postgres unavailable**: 5xx on feed/posts/comments
- **Redis unavailable**: auth init or token revocation checks degrade
- **Auth failures**: spikes in 401 due to expired tokens or wrong issuer/audience

## Quick triage

```bash
kubectl get pods -n rhelma6 -l app=social-service -o wide
kubectl logs -n rhelma6 -l app=social-service --tail=200
kubectl get events -n rhelma6 --sort-by=.lastTimestamp | tail -n 40
```

### Check dependencies

- Postgres connectivity
- Redis connectivity

```bash
kubectl get pods -n rhelma6 | grep -E 'postgres|redis'
```

## Fast recovery actions

### Rollout restart

```bash
kubectl rollout restart deploy/social-service -n rhelma6
kubectl rollout status deploy/social-service -n rhelma6 --timeout=180s
```

### Validate feed quickly

```bash
# Replace headers as needed
curl -s "http://social-service:8085/feed/latest" \
  -H "x-tenant-id: central" | jq .
```

## Evidence collection

- capture request_id/correlation_id from logs
- record the failing tenant id and post id (if applicable)
