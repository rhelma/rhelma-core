# Bridge Receipts V2 (Ed25519 Signed)

Receipts are public, portable attestations that an action occurred.

## Canonical payload (ReceiptPayloadV2)
- `receipt_version`: "2"
- `receipt_id`: stable unique id (hex)
- `issued_at_unix`: i64
- `issuer`: string (service id)
- `intent_id`: string
- `action`: "intent_created" | "proof_submitted" | "finalized" | "cancelled" | "rejected"
- `chain`: string
- `direction`: "deposit" | "withdraw"
- `amount`: string (decimal string)
- `asset`: string
- `subject_id`: string (who benefits)
- `audit_digest_hex`: string (sha256 hex)
- `audit_entry_hash_hex`: string (sha256 hex of canonical audit entry JSON)
- `merkle_root_hex`: string (latest root after append)
- `policy_head_hash_hex`: optional string (Phase 25 policy head)
- `external_ref`: optional string (external settlement reference)

## Signature envelope
- `payload_b64`: base64url(canonical_json(payload))
- `signature_b64`: base64url(ed25519_sign(payload_bytes))

## Verification rules
- payload must be canonical JSON (sorted keys) before signing
- signature verifies against published receipt public key
- `audit_entry_hash_hex` must exist in audit log and verify inclusion to `merkle_root_hex`

## Privacy
- receipts must not contain raw proofs or sensitive user data
- put only digests/references
