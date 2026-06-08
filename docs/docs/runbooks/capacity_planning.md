# Capacity Planning Guide (Rhelma6)

This guide provides **practical defaults** for sizing Rhelma6 deployments and a repeatable process for adjusting resources safely.

---

## What to measure (weekly)

- **p95 / p99 latency** per critical endpoint (api-gateway)
- **error rate** (4xx/5xx) per service
- **Kafka consumer lag** per consumer group
- **CPU / memory headroom** (target: ≥ 30% headroom)
- **database saturation** (connections, IOPS, cache hit rate)

---

## Baseline resource targets (starter)

> These are initial baselines for a small-to-medium cluster; tune using load tests.

| Component | CPU | Memory | Notes |
|---|---:|---:|---|
| api-gateway | 500m–2 | 512Mi–2Gi | scale by RPS and p99 latency |
| ai-orchestrator | 1–4 | 1Gi–4Gi | scale by event volume + model usage |
| node-registry | 250m–1 | 256Mi–1Gi | watch DB connections |
| vlf (value ledger) | 1–4 | 1Gi–8Gi | depends heavily on federation settings |

---

## Load testing → capacity decisions

1) Pick a k6 profile (smoke, baseline, peak)
2) Run for 30–60 minutes
3) Record:
   - max sustainable RPS before p95 breaches SLO
   - error rate under steady load
   - Kafka lag under peak
4) Adjust **one lever at a time**:
   - replicas
   - resource limits
   - DB pool size
   - Kafka partitions

---

## Kafka sizing (rule of thumb)

- Partitions per high-volume topic: **(max consumers) × (2–3)**
- Keep average partition throughput below **5–10 MB/s** unless broker tuned
- Prefer scaling by **partitions** and **consumer replicas** over giant single consumers

---

## Safe scaling checklist

- [ ] Can we roll back quickly? (chart version / image tag)
- [ ] Is HPA/PDB configured?
- [ ] Are dashboards/alerts green before change?
- [ ] Do we have a canary window?

---

## When to add a region

Consider multi-region when:
- SLO requires regional redundancy
- single-region blast radius is unacceptable
- residency constraints require local processing

For failover steps, see: `docs/runbooks/regional_failover.md`.
