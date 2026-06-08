# Checkpoint Propagation Protocol v1 (Gossip Layer)

## Purpose
Propagate audit checkpoints (Merkle roots + anchor references) across a peer-to-peer membership graph so
any node can discover the latest checkpoint head for a domain.

## Message: `rhelma.checkpoint.propagate.v1`
Canonical JSON (UTF-8), signed by sender (Ed25519).

### Fields
- `domain` (string): e.g. `bridge.audit`
- `height` (u64): monotonic increasing sequence number for the domain
- `merkle_root_hex` (string): 32-byte hex
- `anchor_ref` (string|null): reference to the external anchor (ledger tx id, etc.)
- `produced_at_unix` (u64): timestamp of production
- `issuer_peer_id` (string): peer id derived from pubkey (Phase 15)
- `issuer_pubkey_b64` (string)
- `signature_b64` (string): signature over the canonical payload *excluding* `signature_b64`

### Canonical signing payload
The signature is computed over:
- JSON with stable key ordering (lexicographic),
- without `signature_b64`.

## Rules
- Peers MUST verify:
  1) `issuer_peer_id` matches `sha256(pubkey)` hex
  2) signature is valid
  3) `domain` is in allow-list
  4) `produced_at_unix` within max skew
- Head update:
  - accept if `height` > current
  - if equal height, accept only if `produced_at_unix` is newer and `merkle_root_hex` differs (rare)
- Propagation:
  - on accept, enqueue for fanout to a subset of peers (weighted selection).

## HTTP Endpoints (recommended)
- `POST /v1/checkpoints/push`
  - body: `CheckpointPropagateV1`
  - returns: `{ accepted: bool, reason?: string, head?: CheckpointHeadV1 }`
- `GET /v1/checkpoints/head?domain=...`
  - returns: `CheckpointHeadV1` or 404

## Security Notes
- This protocol is designed to be compatible with the crypto membership introduced in Phase 15.
- For open networks, combine with Phase 16 allow/deny policies.
