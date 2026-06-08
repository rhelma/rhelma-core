# MNI Export Anchoring Protocol v1

## Purpose
Anchor distillation exports into an append-only, federated artifact store so any node can verify:
- what dataset head was used
- which stream and revision produced the export
- the exact bundle content hash

## Anchor record
### MniExportAnchorV1
- `export_id` (string)
- `stream_id` (string)
- `stream_revision` (u64)
- `bundle_hash_sha256` (hex)
- `dataset_head_hash` (hex, optional)
- `created_at` (RFC3339)
- `issuer` (service id)
- `signature_b64` (optional, Ed25519)

## Recommended storage
- Policy subject: `__mni__exports`
- Append-only history
- Active head = latest record list OR Merkle root over history window

## Rollback
- Rollback MAY exist but MUST be bounded by a time window, and history must remain immutable.
