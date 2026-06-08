# Rhelma6 Incident Response Playbook

**Scope:** on-call response for Rhelma6 services (API Gateway, Node Registry, AI Orchestrator, Value Ledger, File Storage).

## Severity Levels

| Level | Target response | Examples |
|---|---:|---|
| **SEV1** | 15 minutes | Full outage, widespread auth failures, suspected breach |
| **SEV2** | 60 minutes | Partial outage, regional routing failure, major degradation |
| **SEV3** | 4 hours | Single service impaired, error spike limited to one feature |
| **SEV4** | 24 hours | Minor bug, non-prod issues |

## First 5 Minutes (Always)

1. **Acknowledge** alert / incident and note start time.
2. **Confirm blast radius**
   - Which region(s)? Which service(s)? Which endpoints?
3. **Check current deploys/rollouts**
   ```bash
   kubectl get deploy -n rhelma6
   kubectl get rs -n rhelma6 --sort-by=.metadata.creationTimestamp | tail -n 20
   kubectl get events -n rhelma6 --sort-by=.lastTimestamp | tail -n 50
   ```
4. **Check dashboards** (Prometheus/Grafana) for:
   - request rate (RPS), error rate, latency p95/p99
   - Kafka consumer lag
   - CPU/memory saturation
5. **Grab a correlation id / request id** from a failing request.

## Runbook: API Gateway Down (SEV1)

### Symptoms
- 5xx spike across all routes
- Liveness/readiness failing

### Quick Checks
```bash
kubectl get pods -n rhelma6 -l app=api-gateway -o wide
kubectl describe pod -n rhelma6 -l app=api-gateway | sed -n '1,200p'
kubectl logs -n rhelma6 -l app=api-gateway --tail=200
```

### Fast Recovery Actions
1. **Restart deployment**
   ```bash
   kubectl rollout restart deploy/api-gateway -n rhelma6
   kubectl rollout status deploy/api-gateway -n rhelma6 --timeout=180s
   ```
2. If the issue started after a rollout: **rollback**
   ```bash
   kubectl rollout undo deploy/api-gateway -n rhelma6
   kubectl rollout status deploy/api-gateway -n rhelma6 --timeout=180s
   ```
3. If pods are OOMKilled: increase limits or reduce concurrency temporarily.

## Runbook: Kafka Lag / Event Backlog (SEV2)

### Symptoms
- Consumers falling behind (lag increasing)
- Alerts not firing / region health not updating

### Diagnosis
```bash
kubectl get pods -n rhelma6 | grep -E 'kafka|ai-orchestrator|api-gateway|obs'
kubectl logs -n rhelma6 -l app=ai-orchestrator --tail=200
kubectl logs -n rhelma6 -l app=api-gateway --tail=200
```

### Recovery
- **Scale consumers** (if CPU-bound) and watch lag trend:
  ```bash
  kubectl scale deploy/ai-orchestrator -n rhelma6 --replicas=3
  kubectl scale deploy/api-gateway -n rhelma6 --replicas=3
  ```
- Verify broker health (disk, ISR, under-replicated partitions).

## Runbook: Region Degraded / Failover (SEV2)

Rhelma6 uses **region health events** (`obs.region_health`) and **failover events** (`obs.region_failover`) emitted by the gateway.

### Diagnosis
```bash
kubectl logs -n rhelma6 -l app=api-gateway --tail=400 | grep -Ei 'region|failover|health'
kubectl logs -n rhelma6 -l app=ai-orchestrator --tail=400 | grep -Ei 'region_health|region_failover'
```

### Actions
1. Confirm residency constraints: do **not** route requests into a region disallowed by policy.
2. If failover did not occur automatically, restart the gateway to re-evaluate health:
   ```bash
   kubectl rollout restart deploy/api-gateway -n rhelma6
   ```
3. If a region is flapping, consider a temporary **failback cooldown** increase via config (if available), or reduce traffic.

## Runbook: Node Registry Unavailable (SEV1)

### Symptoms
- Discovery failures
- Agents cannot register / heartbeats missing

### Diagnosis
```bash
kubectl get pods -n rhelma6 -l app=node-registry
kubectl logs -n rhelma6 -l app=node-registry --tail=200
kubectl get events -n rhelma6 --sort-by=.lastTimestamp | tail -n 50
```

### Recovery
```bash
kubectl rollout restart deploy/node-registry -n rhelma6
kubectl rollout status deploy/node-registry -n rhelma6 --timeout=180s
```

## Runbook: Value Ledger Inconsistency / Errors (SEV2)

### Symptoms
- credit/balance requests fail or disagree across peers
- federation snapshot lag alert

### Diagnosis
```bash
kubectl logs -n rhelma6 -l app=value-ledger --tail=200
kubectl logs -n rhelma6 -l app=value-ledger-federation --tail=200
```

### Recovery
- Prefer **freeze writes** (feature flag / config) if available, then reconcile.
- Restart federation service to trigger resync:
  ```bash
  kubectl rollout restart deploy/value-ledger-federation -n rhelma6
  ```

## Communications

### Status Update Template

- **What happened:**
- **Impact:**
- **Current status:**
- **Mitigation:**
- **Next update ETA:**

## After the Incident

- Document timeline + root cause
- Add regression test / guard if possible
- Update runbooks with anything learned

## Related service runbooks

- [API Gateway](./service_api_gateway.md)
- [Node Registry](./service_node_registry.md)
- [AI Orchestrator](./service_ai_orchestrator.md)
- [Value Ledger](./service_value_ledger.md)
- [Region Health Aggregator](./service_region_health_aggregator.md)