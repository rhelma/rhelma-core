# Rhelma6 unified rules (Contract v6.0)

This document captures **cross-cutting governance rules** for Rhelma Realm v6.

These rules are **normative** and use RFC2119 language.

## 1) Federated value system

1. Credits **MUST** be represented as an append-only transaction log; balances are derived and recomputable.
2. All credit transactions **MUST** be signed by an authorized issuer key.
3. Cross-domain settlements (bridging to “real world value”) **MUST** require quorum governance and auditable receipts.
4. Punitive deductions (slashing) **MUST** be proportional and **MUST** be backed by a governance/jury outcome.
5. Right-to-audit: non-sensitive metadata **MUST** be verifiable by participants.

Specs:
- `docs/contract/v6.0/specs/VALUE_LEDGER_API_v1.md`
- `docs/contract/v6.0/specs/FEDERATED_VALUE_LEDGER_PROTOCOL_v1.md`
- `docs/contract/v6.0/specs/BRIDGING_PROTOCOL_v1.md`

## 2) Rhelma Native Intelligence (MNI)

1. Training data and derived artifacts **MUST** be traceable (lineage).
2. Dataset anchoring/checkpointing **MUST** be signed and replayable.
3. Any export/distillation artifact that is used for serving **MUST** have an approval record and an anchoring record.
4. Evaluation/anti-poison gates **MUST** be enforceable by policy and **MUST** be auditable.

Specs:
- `docs/contract/v6.0/specs/MNI_DATASET_LINEAGE_V1.md`
- `docs/contract/v6.0/specs/MNI_DATASET_ANCHORING_PROTOCOL_v1.md`
- `docs/contract/v6.0/specs/MNI_DISTILLATION_PROTOCOL_v1.md`
- `docs/contract/v6.0/specs/MNI_ANTIPOISON_GATE_v1.md`

## 3) Live defense system

1. Security enforcement **MUST NOT** become a mechanism for central capture.
2. High-impact actions (suspension, slashing, irreversible sanctions) **MUST** be quorum-governed and fully auditable.
3. The default safety posture during uncertainty **MUST** be “deny / reduce privilege”, with explicit rollback.

Specs:
- `docs/contract/v6.0/specs/SECURITY_GOVERNANCE_API_v1.md`
- `docs/contract/v6.0/specs/SECURITY_GOV_ENFORCEMENT.md`
- `docs/contract/v6.0/specs/MERKLE_AUDIT_LOG_V1.md`

## 4) Change control

1. Any change that affects a public interface (HTTP, events, wire formats) **MUST** update the corresponding spec in `docs/contract/v6.0/specs/`.
2. Any new invariant that must be enforced by CI **MUST** be expressed as a deterministic verification step.
3. Developer documentation **MUST NOT** be organized by “phases”. Use domain-based docs (getting-started/architecture/operations/reference).
