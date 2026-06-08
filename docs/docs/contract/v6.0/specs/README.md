# Contract specs & protocols (v6.0)

This folder hosts the **protocol, API, and data-format specifications** that the codebase must conform to.

> If you change a public interface (HTTP, events, wire formats), update the relevant spec and keep changes versioned.

## Core networking and node APIs

- `A1_PROTOCOL_P2P.md` — P2P overlay protocol (sketch)
- `A2_EVENT_TOPICS_RHELMA6.md` — event topics taxonomy for the node swarm
- `A3_NODE_API_SPEC.md` — minimal Node Registry + Node HTTP APIs
- `POLICY_ROUTING_STATE_SPEC_v1.md` — routing policy state model

## AI orchestration

- `AI_ORCH_ROUTING_HOOKS.md` — orchestration routing hooks
- `AGENT_LIQUIDITY_PROTOCOL_v1.md` — agent liquidity & migration protocol

## Security and governance

- `SECURITY_GOVERNANCE_API_v1.md` — security governance API
- `SECURITY_GOV_ENFORCEMENT.md` — enforcement semantics
- `SECURITY_POLICY_ACTIONS_API_v1.md` — policy actions API
- `PERMISSIONLESS_ANTISYBIL_V1.md` — anti-sybil scheme
- `RATE_LIMITING_ADMISSION_CONTROL_V1.md` — admission and rate limiting
- `PROOF_OF_CONTRIBUTION_V1.md` — proof-of-contribution model
- `MERKLE_AUDIT_LOG_V1.md` — append-only audit log format

## Value and bridging

- `VALUE_LEDGER_API_v1.md` — value ledger API
- `FEDERATED_VALUE_LEDGER_PROTOCOL_v1.md` — federated value ledger protocol
- `BRIDGING_PROTOCOL_v1.md` — bridging protocol
- `BRIDGE_RECEIPTS_V2.md` — signed bridge receipts
- `BRIDGE_AUDIT_DIGEST_v1.md` — audit digest format
- `BRIDGE_DRIVER_INTERFACE_v1.md` — bridge driver interface
- `BRIDGE_DRIVER_POLICY_ARTIFACT_v1.md` — bridge driver policy artifact
- `TREASURY_AND_INHERITANCE_HOOKS.md` — treasury and inheritance hooks
- `DIGITAL_FAMILY_VAULT_PROTOCOL_v1.md` — digital family & inheritance vault protocol
- `DIGITAL_FAMILY_GUARDIAN_SIGNING.md` — guardian signing helper

## Checkpoints and propagation

- `CHECKPOINT_API_v1.md` — checkpoint API
- `CHECKPOINT_EVENTS_V1.md` — checkpoint events
- `CHECKPOINT_PROPAGATION_PROTOCOL_v1.md` — checkpoint propagation protocol

## Rhelma Native Intelligence (MNI)

- `MNI_RAG_API_v1.md` — RAG API
- `MNI_DATA_CONSTITUTION_V1.md` — dataset/data constitution
- `MNI_STREAM_APPROVAL_PROTOCOL_v1.md` — stream approval protocol
- `MNI_APPROVED_STREAMS_v1.md` — approved streams format
- `MNI_CHALLENGE_SET_PROTOCOL_v1.md` — challenge set protocol
- `MNI_DATASET_ANCHORING_PROTOCOL_v1.md` — dataset anchoring protocol
- `MNI_DATASET_CHECKPOINTS_V1.md` — dataset checkpoints
- `MNI_DATASET_LINEAGE_V1.md` — dataset lineage
- `MNI_DATASET_POLICY_ARTIFACT_v1.md` — dataset policy artifact
- `MNI_ANTIPOISON_GATE_v1.md` — anti-poison gate
- `MNI_SECURE_RUNNER_CONTRACT_v1.md` — secure runner contract
- `MNI_DISTILLATION_PROTOCOL_v1.md` — distillation protocol
- `MNI_EXPORT_ANCHORING_PROTOCOL_v1.md` — export anchoring protocol
- `MNI_EXPERT_ROUTING_POLICY_v1.md` — expert routing policy
- `MNI_SERVING_MOE_LITE_v1.md` — serving (MoE-lite) spec
- `MNI_FEDERATED_FINETUNE_PROTOCOL_v1.md` — federated fine-tune protocol
