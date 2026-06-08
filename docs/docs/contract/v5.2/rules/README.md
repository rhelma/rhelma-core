# Policies and Rules (Repository Governance)

This file is a **single index** of the most important policy/rule surfaces in this
repository. Detailed specs live in the linked documents.

Start with: **Golden rules** → `docs/contract/v5.2/rules/01_GOLDEN_RULES.md`


## Documentation layout (keep the system clear)

- Entry point: `docs/README.md`
- Rules index: `docs/contract/v5.2/rules/README.md` (this file)
- Historical gate notes: `docs/archive/phase-notes/phases/`
- Current verification guide: `docs/contributing/VERIFICATION_AND_GATES.md`
- Avoid adding new markdown files directly under `docs/`
- When adding a new gate/phase, update the relevant index so discoverability stays high
- Once a phase is fully adopted, migrate its durable invariants into the contract and keep the phase doc as rollout history

## Contract-level rules (v5.2)

- **Contract index**: `docs/contract/v5.2/00_INDEX_v5.2.md`
- **Security**: `docs/contract/v5.2/05_SECURITY_v5.2.md`
- **Observability**: `docs/contract/v5.2/02_OBSERVABILITY_v5.2.md`

## Self-improvement governance (ai-orchestrator)

- **Approval workflow** (tokens/links/Slack notifications): `docs/archive/phase-notes/phases/PHASE28_APPROVAL_WORKFLOW.md`
- **Apply policy gate** (patch + test-plan constraints): `docs/archive/phase-notes/phases/PHASE29_POLICY_GATE.md`
- **Evaluation attestation gate** (bind eval result to patch): `docs/archive/phase-notes/phases/PHASE47_EVALUATION_ATTESTATION_GATE.md`
- **Progressive rollout / canary gate**: `docs/archive/phase-notes/phases/PHASE34_ROLLOUT_CANARY_GATE.md`

## Observability alignment & conformance

- **AI ↔ Observability alignment plan**: `docs/archive/phase-notes/phases/PHASE69_AI_OBSERVABILITY_ALIGNMENT.md`
- **Shared HTTP observability rules**: `docs/archive/phase-notes/phases/PHASE70_HTTP_OBSERVABILITY_SHARED.md`
- **Event transport observability**: `docs/archive/phase-notes/phases/PHASE71_EVENT_TRANSPORT_OBSERVABILITY.md`
- **Context propagation (workers/NATS)**: `docs/archive/phase-notes/phases/PHASE72_CONTEXT_PROPAGATION_WORKERS_AND_NATS.md`
- **Outbound context injection**: `docs/archive/phase-notes/phases/PHASE76_OUTBOUND_CONTEXT_INJECTION.md`
- **HTTP outbound context enforcer**: `docs/archive/phase-notes/phases/PHASE77_HTTP_OUTBOUND_CONTEXT_ENFORCER.md`
- **Axum observability standard**: `docs/archive/phase-notes/phases/PHASE78_AXUM_OBSERVABILITY_STANDARD.md`
- **Service rollout plan**: `docs/archive/phase-notes/phases/PHASE79_HTTP_SERVICES_OBSERVABILITY_ROLLOUT.md`
- **Conformance gate**: `docs/archive/phase-notes/phases/PHASE80_OBSERVABILITY_CONFORMANCE_GATE.md`

## Metrics and data hygiene

- **PII + secrets scrubbing rules**: `docs/archive/phase-notes/phases/PHASE82_PII_SECRETS_SCRUBBING.md`
- **Metrics cardinality guardrails**: `docs/archive/phase-notes/phases/PHASE87_METRICS_CARDINALITY_AND_LOG_SCRUBBING_GATES.md`
- **Endpoint cardinality clamp**: `docs/archive/phase-notes/phases/PHASE88_METRICS_ENDPOINT_CARDINALITY_CLAMP.md`
- **Cardinality alerts**: `docs/archive/phase-notes/phases/PHASE89_METRICS_CARDINALITY_ALERTS.md`
- **Recording rules + panels**: `docs/archive/phase-notes/phases/PHASE90_METRICS_CARDINALITY_RECORDING_AND_PANELS.md`

## Pre-frontend gates and smoke tests

- **Pre-frontend alignment + smoke tests**: `docs/archive/phase-notes/phases/PHASE110_PRE_FRONTEND_ALIGNMENT.md`
- Scripts:
  - `scripts/verify_pre_frontend.(sh|ps1)`
  - `scripts/smoke_staging.(sh|ps1)`
  - `scripts/todo_guard.(sh|ps1)` + `.todo-allowlist`

## CI expectations

- **Unified verification**: `docs/archive/phase-notes/phases/PHASE85_UNIFIED_VERIFICATION.md`
- **CI required checks**: `docs/archive/phase-notes/phases/PHASE86_CI_REQUIRED_CHECKS.md`

## How to propose changes to rules

1) Prefer updating the contract docs if the rule is cross-cutting.
2) If the rule is a gate, document it under `docs/archive/phase-notes/phases/PHASE*.md` and wire it into `scripts/verify*.{sh,ps1}`.
3) Keep rule outputs **machine-parseable** (JSON where appropriate) so CI can fail deterministically.
