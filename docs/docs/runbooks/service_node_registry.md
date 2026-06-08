# Service Runbook: Node Registry (Rhelma6)

## What this service does

The Node Registry manages node registration, admission checks, and node lifecycle updates.

## Primary signals

- Health: `GET /healthz`, readiness: `GET /readyz`
- Metrics: `GET /metrics` (when enabled)
- Symptoms: node registration failures, missing heartbeats, discovery degradation

## Quick triage

```bash
kubectl get pods -n rhelma6 -l app=node-registry -o wide
kubectl logs -n rhelma6 -l app=node-registry --tail=200
kubectl get events -n rhelma6 --sort-by=.lastTimestamp | tail -n 40
```

## Redis-backed admission

In multi-instance deployments, PoW challenges and rate-limit counters can be stored in Redis.
If Redis is down or the URL is wrong, admission can fail or become inconsistent.

```bash
kubectl get pods -n rhelma6 | grep -E 'redis'
```

If you suspect Redis issues:

- verify the configured URL (Secret/ConfigMap)
- test connectivity from within the cluster

```bash
kubectl exec -n rhelma6 -it deploy/node-registry -- sh -lc 'echo ping | nc -w 2 ${RHELMA_NODE_REGISTRY__ADMISSION__REDIS_URL#redis://} || true'
```

## Attestation verification

If `RHELMA_NODE_REGISTRY__POLICY__REQUIRE_ATTESTATION_VERIFICATION=true`, failures in the external verifier
will block node admission.

Checklist:

- verify the verifier binary exists in the image
- verify command and timeout settings
- confirm evidence kinds allowed match the incoming node evidence

## Fast recovery actions

```bash
kubectl rollout restart deploy/node-registry -n rhelma6
kubectl rollout status deploy/node-registry -n rhelma6 --timeout=180s
```

If the incident started after a rollout:

```bash
kubectl rollout undo deploy/node-registry -n rhelma6
kubectl rollout status deploy/node-registry -n rhelma6 --timeout=180s
```

## Evidence collection

- capture node_id values from failing requests
- snapshot logs around the admission decision
- export the current governance policy bundle hash if admission is policy-driven
