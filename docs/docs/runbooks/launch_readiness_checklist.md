# Rhelma6 Launch Readiness Checklist

Use this checklist before a production launch, major version cut, or a large config rollout.

> Treat this as a “go/no-go” gate. If an item is “N/A”, write why.

## 1) Build, Quality, and Contract Gates

### One-command gate (recommended)

If you want a single, shareable artifact for the launch ticket, run the release gate:

```bash
./scripts/rhelma6/release_gate.sh
```

This generates `benchmarks/out/release_gate_report.md`.

Run on the exact commit you will deploy:

```bash
# Full verification (preferred)
./scripts/verify.sh
./scripts/verify_all.sh

# Guard rails (explicit)
./scripts/check-structure.sh
./scripts/todo_guard.sh
./scripts/contract_guard.sh
./scripts/event_contract_guard.sh
./scripts/env_contract_guard.sh
./scripts/header_contract_guard.sh
./scripts/outbound_http_context_guard.sh
./scripts/scrapeability_guard.sh
./scripts/metrics_cardinality_guard.sh
./scripts/uuidv7_guard.sh

# Observability verification
./scripts/verify_observability.sh
```

## 2) Config, Secrets, and Governance Readiness

- [ ] No plaintext secrets in repo (double-check `.env.example` is non-sensitive).
- [ ] All required prod secrets exist in the secret store (DB URLs, Kafka, signing keys, tokens).
- [ ] Governance keys present for the councils you run:
  - HS256 keys for bootstrap (if enabled)
  - Ed25519 public keys for verification (recommended)
- [ ] Quorum/timelock settings are intentional:
  - `RHELMA_GOVERNANCE_QUORUM_MODE`
  - `RHELMA_GOVERNANCE_HIGH_IMPACT_MIN_DELAY_SECONDS`
  - `RHELMA_GOVERNANCE_CRITICAL_MIN_DELAY_SECONDS`
  - `RHELMA_GOVERNANCE_QUORUM_CRITICAL_POLICY` (optional)
  - `RHELMA_GOVERNANCE_QUORUM_CRITICAL_SECURITY` (optional)
- [ ] “Break glass” process exists for emergency council (humans, comms, logging).

## 3) Data & Dependency Readiness

- [ ] Postgres migrations applied (if your environment uses them).
- [ ] Redis reachable and persistence mode is appropriate.
- [ ] Kafka brokers reachable from all namespaces/regions.
- [ ] Backups verified:
  - [ ] Most recent snapshot exists
  - [ ] Restore procedure tested at least once (see `disaster_recovery.md`)

## 4) Smoke & Functional Checks

### Quick critical smoke (recommended)

```bash
# Critical endpoints + optional Kafka TCP check
RHELMA_SMOKE_KAFKA_BROKERS="kafka-0:9092,kafka-1:9092" \
  ./scripts/rhelma6/smoke.sh
```

### Full HTTP smoke (staging/prod style)

```bash
RHELMA_SMOKE_API_GATEWAY_URL="https://<gateway>" \
RHELMA_SMOKE_AI_ORCH_URL="https://<ai-orchestrator>" \
RHELMA_SMOKE_NODE_REGISTRY_URL="https://<node-registry>" \
RHELMA_SMOKE_KAFKA_BROKERS="kafka-0:9092,kafka-1:9092" \
  ./scripts/smoke_staging.sh
```

Optional flows:

```bash
# Auth flow (requires DB + migrations)
RHELMA_SMOKE_AUTH_FLOW=1 RHELMA_SMOKE_TENANT_ID=prod \
  ./scripts/smoke_staging.sh

# Node registry admission/register/heartbeat flow (best-effort)
RHELMA_SMOKE_NODE_REGISTRY_FLOW=1 \
  ./scripts/smoke_staging.sh
```

## 5) Load & Chaos Readiness

- [ ] k6 smoke profile green:

```bash
./scripts/rhelma6/load/run_k6_profiles.sh quick both
```

- [ ] Chaos E2E green (at least once on the deploy commit):

```bash
./scripts/rhelma6/chaos/run_chaos_tests.sh
```

- [ ] Baselines exist for the target surfaces (see `benchmarks/baselines/`).

## 6) Observability & Alerting

- [ ] Prometheus scrapes all services (no target drop).
- [ ] Dashboards exist for availability, latency (p95/p99), error rate, Kafka lag.
- [ ] Log correlation confirmed (request_id/correlation_id present).
- [ ] Alerts exist (and are routed):
  - Availability down
  - Error spike
  - Kafka lag critical
  - Region degraded/unhealthy

## 7) Rollout Plan

- [ ] Canary plan written (traffic %, duration, success metrics, rollback triggers).
- [ ] Previous known-good image tags are recorded.
- [ ] Rollback plan is rehearsed.
- [ ] See: `rollout_canary_rollback.md`.

## 8) Post-Launch

- [ ] 24–48h enhanced monitoring window
- [ ] Capture top errors and open patch tickets
- [ ] Record incident notes even if no SEV declared
