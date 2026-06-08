# PostgreSQL Incidents (Rhelma6)

This runbook focuses on common PostgreSQL failure modes in Rhelma6 deployments.

## Symptoms

- connection timeouts or refused connections
- rising request latency across multiple services
- elevated 5xx error rates for DB-backed endpoints

## Quick triage

1) Identify which services are failing DB calls.
2) Inspect DB pod or managed DB health.

For in-cluster PostgreSQL:

```bash
kubectl get pods -n rhelma6 | grep -E 'postgres'
kubectl logs -n rhelma6 -l app=postgres --tail=200
```

## Common issues

### Connection exhaustion

- scale the DB connection pool down per service (if configurable)
- scale the hottest service horizontally rather than increasing connections
- verify idle connections are not leaking

### Storage pressure

- check disk usage on the DB volume
- clean up large log/audit tables per retention policy

### Replication lag (managed DB)

- pause read-after-write traffic patterns if possible
- promote or fail over using the provider mechanism

## Recovery actions

- restart DB only if you are certain the workload can tolerate it.
- prefer restoring from backups or promoting a replica for managed offerings.

## Evidence collection

- capture the error messages from clients (redact secrets)
- capture DB logs around the incident window
- record current connection counts and slow query samples
