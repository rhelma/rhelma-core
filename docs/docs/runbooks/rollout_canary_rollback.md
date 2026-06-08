# Rhelma6 Rollout, Canary, and Rollback

This runbook describes a safe, repeatable rollout pattern for Rhelma6 services.

**Goals:**
- Minimize blast radius
- Detect regressions early (SLOs + smoke)
- Make rollback fast and deterministic

> Assumptions: Kubernetes namespace is `rhelma6`. Adjust for your cluster.

---

## 1) Preconditions

- [ ] You have a known-good image tag to roll back to.
- [ ] Launch readiness checks are green (see `launch_readiness_checklist.md`).
- [ ] Observability is working (metrics, logs, alerts).
- [ ] On-call owners are present for the duration of the canary window.

---

## 2) Define Success and Rollback Conditions

Pick concrete thresholds before you touch prod:

**Suggested success checks (per service):**
- Availability: no sustained 5xx or timeouts
- Latency: p95 not regressed by more than 20%
- Error rate: < 1% sustained
- Kafka lag (if consuming): no growing consumer lag
- Domain checks: key endpoints behave (smoke)

**Rollback triggers (examples):**
- Error rate > 2% for 5 minutes
- p95 latency regresses > 30% for 10 minutes
- CrashLoopBackOff or OOMKill
- Kafka lag grows continuously for 10 minutes

---

## 3) Canary Strategy (Recommended)

### Phase A — Deploy 1 pod (0–5% traffic)

1) Update image for the deployment:

```bash
kubectl -n rhelma6 set image deployment/<svc> <svc>=<image>:<tag>
```

2) Wait for rollout:

```bash
kubectl -n rhelma6 rollout status deployment/<svc>
```

3) Watch pods + events:

```bash
kubectl -n rhelma6 get pods -l app=<svc> -w
kubectl -n rhelma6 describe deploy/<svc> | sed -n '1,200p'
```

4) Run smoke against the canary path (if you have a canary ingress) or against the service directly:

```bash
RHELMA_SMOKE_TIMEOUT_SEC=5 \
RHELMA_SMOKE_API_GATEWAY_URL="https://<gateway>" \
RHELMA_SMOKE_AI_ORCH_URL="https://<ai-orchestrator>" \
RHELMA_SMOKE_NODE_REGISTRY_URL="https://<node-registry>" \
RHELMA_SMOKE_KAFKA_BROKERS="kafka-0:9092,kafka-1:9092" \
  ./scripts/rhelma6/smoke_core.sh
```

Hold at least **10–15 minutes** (longer for stateful changes) while watching dashboards.

### Phase B — Increase to 25–50%

How you do this depends on your traffic mechanism:

- **Ingress/controller canary**: adjust canary weight (preferred)
- **Service mesh**: adjust virtual service routing weights
- **Simple approach**: increase replicas and rely on load balancing

Example (simple approach):

```bash
kubectl -n rhelma6 scale deployment/<svc> --replicas=3
```

Repeat smoke and watch SLOs.

### Phase C — Full rollout

Scale to normal replica count, keep monitoring for the agreed window.

---

## 4) Rollback (Fast Path)

### Option A — Kubernetes rollout undo

```bash
kubectl -n rhelma6 rollout undo deployment/<svc>
kubectl -n rhelma6 rollout status deployment/<svc>
```

### Option B — Pin previous image tag

```bash
kubectl -n rhelma6 set image deployment/<svc> <svc>=<image>:<previous_tag>
kubectl -n rhelma6 rollout status deployment/<svc>
```

### Verify rollback

```bash
./scripts/rhelma6/smoke_core.sh
```

---

## 5) Post-Rollout Checklist

- [ ] Record the deployed image tags (per service).
- [ ] Record canary duration and any anomalies.
- [ ] If rollback happened: create incident report + follow-up tickets.
- [ ] Update baselines if this is a planned performance shift.

---

## 6) Notes for Multi-Region

- Roll out one region at a time unless the change is proven safe.
- Keep failover override TTL short during rollout.
- Confirm `obs.region_health` / `obs.region_failover` signals and dashboards before ramping traffic.
