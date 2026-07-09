# Rhelma6 roadmap

This roadmap uses **milestones** (not phase numbers). It exists to help developers understand **dependencies** and **delivery order**.

Normative requirements live in the contract:
- `docs/contract/v6.0/` (rules + specs)

## Milestone A — Foundation

Goal: a minimal, verifiable base that can evolve without re-architecture.

- **Node identity & registration** (bootstrap)
- **Node API** baseline (health, capabilities, simple work endpoint)
- **Event taxonomy** for the swarm (topics, envelopes)
- **Request context propagation** across HTTP and event boundaries
- **Repository verification**: deterministic checks that enforce contract invariants

Relevant docs:
- `docs/architecture/NODE_LIFECYCLE_AND_REGISTRY.md`
- `docs/contract/v6.0/specs/A3_NODE_API_SPEC.md`
- `docs/contract/v6.0/specs/A2_EVENT_TOPICS_RHELMA6.md`
- `docs/contributing/VERIFICATION_AND_GATES.md`

## Milestone B — Trust & governance

Goal: prevent capture and enforce safety without central control.

- **Attestation** and trust tiers for nodes
- **Anti-sybil** admission controls
- **Auditability** of privileged actions (signed receipts)
- **Governance publishing** (rules/policies as versioned artifacts)

Relevant docs:
- `docs/architecture/SECURITY_ATTESTATION_AND_TRUST.md`
- `docs/architecture/GOVERNANCE_AND_UPGRADES.md`
- `docs/contract/v6.0/specs/SECURITY_GOVERNANCE_API_v1.md`

## Milestone C — Fluid core & routing

Goal: make orchestration “fluid” (no fixed central brain), while keeping correctness.

- Multi-node routing policies
- Node selection based on trust, health, and policy constraints
- Failure handling (retries, rerouting, quarantines)
- Decentralized discovery primitives

Relevant docs:
- `docs/architecture/FLUID_CORE.md`
- `docs/contract/v6.0/specs/A1_PROTOCOL_P2P.md`
- `docs/contract/v6.0/specs/POLICY_ROUTING_STATE_SPEC_v1.md`

## Milestone D — Incentives & value system

Goal: align incentives so the network attracts real compute and real operators.

- Value ledger primitives
- Bridging and receipts
- Reputation-weighted rewards and penalties
- Treasury hooks and inheritance hooks

Relevant docs:
- `docs/architecture/INCENTIVES_AND_VALUE_SYSTEM.md`
- `docs/contract/v6.0/specs/VALUE_LEDGER_API_v1.md`
- `docs/contract/v6.0/specs/BRIDGING_PROTOCOL_v1.md`

## Milestone E — Rhelma Native Intelligence (MNI)

Goal: make AI workflows verifiable, safe, and composable.

- Stream approval and dataset anchoring
- Checkpoint propagation and lineage
- Secure runner contract
- Distillation/export anchoring

Relevant docs:
- `docs/architecture/RHELMA_NATIVE_INTELLIGENCE.md`
- `docs/contract/v6.0/specs/MNI_RAG_API_v1.md`
- `docs/contract/v6.0/specs/MNI_DATASET_LINEAGE_V1.md`
- `docs/contract/v6.0/specs/MNI_SECURE_RUNNER_CONTRACT_v1.md`

## Milestone F — Production hardening

Goal: scale and ship safely.

- Observability conformance gates (metrics/logs/traces)
- Cardinality + PII scrubbing enforcement
- Canary + rollback discipline
- Chaos drills and game days
- Multi-region readiness

Relevant docs:
- `docs/operations/OBSERVABILITY_AND_OPERATIONS.md`
- `docs/operations/LAUNCH_HARDENING_PLAYBOOK_v1.md`
- `docs/contract/v6.0/rules/01_GOLDEN_RULES.md`

## Historical phase notes

Earlier internal work used “phases”. Those documents are kept as historical reference only:

- `docs/archive/phases/`
- `docs/archive/phase-notes/phases/`
