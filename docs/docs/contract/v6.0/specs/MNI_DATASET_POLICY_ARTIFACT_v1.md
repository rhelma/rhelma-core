# MNI Dataset Policy Artifact v1 (Value Ledger Federation)

This document specifies the policy artifact stored in VLF for MNI dataset anchoring.

## Storage Model
A policy update is recorded as a signed txlog entry:
- subject_id: `__policy__mni_datasets`
- delta: 0
- reason:
  - `policy.mni_datasets.set`
  - `policy.mni_datasets.rollback`
- metadata_json: Policy payload (below)

## Policy Payload (JSON)
### Set
```json
{
  "kind": "set",
  "dataset_id": "lexicon-core",
  "merkle_root_hex": "…32-byte hex…",
  "ts_unix": 1735350000,
  "publisher_node_id": "node_…",
  "publisher_pubkey_b64": "…",
  "publisher_sig_b64": "…",
  "note": "optional human note"
}
```

### Rollback
```json
{
  "kind": "rollback",
  "to_merkle_root_hex": "…",
  "ts_unix": 1735350100,
  "reason": "incident-123"
}
```

## Read APIs
- `GET /v1/policy/mni-datasets`
Returns:
- active_head (payload)
- allowlist / denylist (resolved from env)
- policy_head_hash (sha256 of canonical JSON)

- `GET /v1/policy/mni-datasets/history?limit=N`
Returns last N records (append-only).

## Guardrails
- If dataset_id is denylisted, `set` is rejected.
- If allowlist is set and dataset_id not in it, `set` is rejected.
- Rollback is only applied if within `RHELMA_VLF__POLICY_ROLLBACK_WINDOW_SEC`.
