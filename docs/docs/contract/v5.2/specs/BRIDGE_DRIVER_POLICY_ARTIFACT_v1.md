# Bridge Driver Policy Artifact v1

This document defines how `value-ledger-federation` publishes and replicates bridge-driver policy.

## Artifact encoding
Policy artifacts are encoded as **SignedTx** (append-only), replicated via `/v1/federation/snapshot`.

### Invariants
- `subject_id` MUST be: `__policy__bridge_drivers`
- `delta` MUST be: `0`
- `reason` MUST be:
  - `policy.bridge_drivers.set` or
  - `policy.bridge_drivers.rollback`

## Tags
Tags are key/value strings.

### Common tags
- `policy_digest:<hex>` — SHA-256 digest of the policy artifact fields.
- `policy_prev:<hex>` — previous head digest (optional).
- `proposal_id:<hex>` — governance proposal id that produced this artifact (optional).

### SET tags
- `allowed_chains:<b64>` — base64 of comma-separated chains (e.g. `mocknet,eth`).
- `allowed_drivers:<b64>` — base64 of comma-separated driver IDs (e.g. `mock,evm`).

### ROLLBACK tags
- `target_digest:<hex>` — digest of a prior policy to roll back to.

## Applicability rules
A policy artifact may be **recorded** but not necessarily **applied**.

- `policy.bridge_drivers.set` is always applied as the new head.
- `policy.bridge_drivers.rollback` is applied only if:
  1) there is a current head, and
  2) the rollback is within `RHELMA_VLF__POLICY_ROLLBACK_WINDOW_SEC` from the head's `issued_at_unix`, and
  3) the target digest exists in history.

If not applicable, it remains in history with `applied=false` and an error string.

## Public endpoints
- `GET /v1/policy/bridge-drivers` — returns current head policy.
- `GET /v1/policy/bridge-drivers/history?limit=N` — returns policy history.

## Consumer expectations
`bridge-adapter` should:
- query `/v1/policy/bridge-drivers` before settling;
- require `chain ∈ allowed_chains` and `driver ∈ allowed_drivers`;
- treat the returned `digest_hex` as the **policy version** used for its audit trail.
