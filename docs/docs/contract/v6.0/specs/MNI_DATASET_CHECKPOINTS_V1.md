# MNI Dataset Checkpoints V1

## Checkpoint Object
- `ts_unix` (i64)
- `leaf_count` (u64)
- `merkle_root_hex` (lowercase hex sha256)
- `publisher_pubkey_b64` (base64)
- `signature_b64` (base64)

### Checkpoint Signing Payload (deterministic)
`mni_checkpoint_v1|{leaf_count}|{merkle_root_hex}|{ts_unix}`

## Purpose
- Provide a stable "head" identifier for dataset state.
- Enable network anchoring/propagation in later phases (e.g., via gossip checkpoints).

## Security notes
- Publishing a checkpoint does **not** grant permission to ingest.
- Checkpoints are verifiable, append-only artifacts and can be replayed.
