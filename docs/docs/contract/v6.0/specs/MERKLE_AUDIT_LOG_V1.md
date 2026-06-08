# Merkle Audit Log v1 (Append-only JSONL)

## Format
Each appended line is a single JSON object (AuditEntryV1) with canonical key ordering.

Suggested fields:
- `v`: 1
- `ts_unix`: i64
- `intent_id`: string
- `action`: string
- `audit_digest_hex`: string
- `chain`: string
- `direction`: string
- `subject_id`: string
- `meta`: object (small, non-sensitive references only)

## Hashing
- `entry_hash = sha256(canonical_json(entry))` (hex)
- Merkle leaves are `entry_hash_bytes` (32 bytes).

## Proof
Inclusion proof is a list of sibling hashes with left/right position.
Verification recomputes up to root.

## Root monotonicity
Root changes after each append. Store roots over time; clients may pin roots for checkpoints.
