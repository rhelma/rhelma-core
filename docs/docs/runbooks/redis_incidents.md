# Redis Incidents (Rhelma6)

Redis may be used for admission state, sessions, caching, or rate-limit coordination.

## Symptoms

- sudden auth/session failures
- inconsistent rate limiting
- timeouts in node admission flows

## Quick triage

```bash
kubectl get pods -n rhelma6 | grep -E 'redis'
kubectl logs -n rhelma6 -l app=redis --tail=200
```

If you have access to `redis-cli` in the cluster:

```bash
kubectl exec -n rhelma6 -it deploy/redis -- sh -lc 'redis-cli ping'
```

## Common issues

- memory pressure and evictions
- network policies blocking access
- wrong connection URL (service name or credentials)

## Recovery actions

- restart Redis only if persistence and recovery expectations are clear.
- if eviction is the issue, increase memory or reduce cache footprint.

## Evidence collection

- capture service logs that show Redis errors
- capture Redis INFO output (memory, stats) if available
