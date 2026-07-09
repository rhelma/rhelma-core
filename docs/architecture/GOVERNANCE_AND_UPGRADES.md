# Rhelma6 — Governance & Upgrades

**Status:** Draft (Roadmap)

Governance must allow evolution without central fragility.

## 1) What needs governance?

- Node admission rules (attestation requirements, quotas)
- Scheduler policy (weights, caps)
- Protocol versions (P2P message schemas)
- Security keys (root keys, signing rotations)
- Emergency actions (suspensions, rollback)

## 2) Bootstrap governance (Phase 1)

Start with a **human-controlled governance board**:

- Coordinator signs policy bundles (Ed25519)
- Nodes accept policies only if signed by trusted keys
- Key rotation is documented and auditable

## 3) Transition governance (Phase 3+)

Move to a **quorum-signature** model:

- N-of-M governance keys required to publish new policies
- Policies are stored in an append-only log (hash chained)
- Nodes accept the newest valid policy chain

## 4) “Bitcoin-like” property (what it means here)

Bitcoin's strength is not “no leaders”, but:
- **verifiable history**
- **consensus on state**
- **difficulty of rewriting history**

For Rhelma6, a practical equivalent is:

- Append-only governance log
- Quorum signatures for state transitions
- Auditability + rollback policies

## 5) Upgrade safety

- Protocol upgrades require a compatibility window.
- Nodes must support at least two versions during migration (N and N-1).
- Kill-switch / rollback exists for critical failures.

## 6) RFC process

Any change to rules/policies should have:
- motivation
- spec
- security analysis
- rollout plan
- acceptance criteria
