# Checkpoint Events V1 (Rhelma6)

## Purpose
Standardize how services publish **externally verifiable checkpoints** (signed Merkle roots) and how the network anchors them.

## Domain
A checkpoint applies to a *domain* (string), e.g.
- `bridge.audit`
- `node.registry.snapshot`
- `value.ledger.snapshot`

## Checkpoint Envelope (V1)
Canonical JSON (sorted keys, UTF-8) signing input:

```json
{
  "v": 1,
  "checkpoint_id": "uuid",
  "domain": "bridge.audit",
  "sequence": 123,
  "merkle_root_hex": "<64 hex>",
  "created_at_unix": 1730000000,
  "producer_pubkey_b64": "...",
  "signature_b64": "..."
}
```

### Signature
- `signature_b64` = Ed25519 signature over canonical bytes of the object **excluding** `signature_b64`.
- `producer_pubkey_b64` is the verifying key.

## Event: rhelma.checkpoint.pinned.v1
Emitted when a checkpoint is pinned into VLF (or another append-only anchor).

### Payload
```json
{
  "checkpoint": { "...": "..." },
  "anchor": {
    "service": "value-ledger-federation",
    "tx_id": "...",
    "subject_id": "__checkpoint__bridge.audit"
  }
}
```

## Anchoring rule
Pinned checkpoints MUST be:
- append-only
- replicated (federated)
- queryable by latest + history

## Anti-abuse
- domains may define max frequency
- require attestation for producers
- require multi-signer quorum for acceptance
