# MNI Dataset Lineage V1

## Goals
- Provide **tamper-evident** provenance for every dataset item used by MNI.
- Allow offline verification using:
  - deterministic signing payload,
  - signature verification,
  - merkle inclusion proofs (Phase 32).

## Record: LineageRecordV1 (JSON)

Fields:
- `doc_id` (string)
- `source` (string)
- `tier` (`public_commons | consent_based | sensitive_jury_gated | private_no_train`)
- `consent` (bool)
- `tags` (string[])
- `content_sha256_hex` (lowercase hex sha256 of content)
- `content_len` (u32)
- `ingested_at_unix` (i64)
- `ingester_pubkey_b64` (base64)
- `signature_b64` (base64) — Ed25519 signature over the **Signing Payload V1**

### Signing Payload V1 (deterministic)
ASCII string:

`mni_lineage_v1|{doc_id}|{source}|{tier}|{consent}|{content_sha256_hex}|{content_len}|{ingested_at_unix}|{tags_joined}`

Where:
- `tags_joined` is tags joined by `,` after sorting lexicographically.

### Leaf Hash (for Merkle tree)
`leaf_hash = sha256( signing_payload_bytes || signature_bytes )`

(If signatures are disabled in dev, signature_bytes is empty.)

## Verification
To verify a record:
1) Recompute signing payload.
2) Verify Ed25519 signature against `ingester_pubkey_b64`.
3) Recompute `leaf_hash` and use inclusion proof to check it is in the dataset root.
