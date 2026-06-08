# Service Runbook: AI Orchestrator (Rhelma6)

## What this service does

AI Orchestrator consumes operational events and orchestrates analysis pipelines.
It is also a key consumer for multi-region observability topics.

## Primary signals

- Health: `GET /healthz`, readiness: `GET /readyz`
- Metrics: `GET /metrics` (when enabled)
- Kafka consumer lag (critical)

## Quick triage

```bash
kubectl get pods -n rhelma6 -l app=ai-orchestrator -o wide
kubectl logs -n rhelma6 -l app=ai-orchestrator --tail=200
```

### Kafka lag symptoms

- alerts not firing
- region health not reflected in analysis
- backlog of `obs.*` topics

Validate broker health and consumer status:

```bash
kubectl get pods -n rhelma6 | grep -E 'kafka'
kubectl logs -n rhelma6 -l app=ai-orchestrator --tail=300 | grep -Ei 'kafka|consumer|rebalance|lag'
```

## Fast recovery actions

- restart deployment:
  ```bash
  kubectl rollout restart deploy/ai-orchestrator -n rhelma6
  kubectl rollout status deploy/ai-orchestrator -n rhelma6 --timeout=180s
  ```

- scale consumers (if CPU-bound):
  ```bash
  kubectl scale deploy/ai-orchestrator -n rhelma6 --replicas=3
  ```

## Evidence collection

- capture the relevant topic name and partition information from logs
- record offsets around the time of the incident
- capture correlation ids from any downstream action triggered by the orchestrator
