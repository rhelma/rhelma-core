# Bridge Audit Digest v1

## Purpose
A deterministic digest that binds a finalized bridge settlement to:
- **BridgeIntent** (what the user wanted)
- **ExternalProof** (what the outside world proved)
- **SettlementResult** (what the driver decided)
- **Policy head hash** (which governance policy was active)

This allows auditors to verify *exactly* what was settled and under which driver policy.

## Digest algorithm
- Input: canonical JSON of the `BridgeAuditEnvelopeV1` (see below)
- Hash: `SHA-256`
- Output: lowercase hex string (`audit_digest_hex`)

## Canonicalization
Use RFC 8785 (JSON Canonicalization Scheme) or an equivalent deterministic canonicalization:
- UTF-8
- sorted keys
- no insignificant whitespace
- stable number formatting

If you do not implement full RFC 8785 initially, you MUST:
- sort keys recursively,
- stringify numbers deterministically,
- and add a regression test that replays known vectors.

## BridgeAuditEnvelopeV1
```json
{
  "version": 1,
  "policy_head_hash": "<hex>",
  "intent": { /* BridgeIntentV1 */ },
  "proof":  { /* ExternalProofV1 */ },
  "settlement": { /* SettlementResultV1 */ }
}
```

### Notes
- `policy_head_hash` MUST be the active head returned by Value Ledger Federation policy artifact endpoint.
- If policy changes, the same intent/proof can produce a different digest, which is desirable.

## What must be included
Minimum required intent fields:
- `intent_id`
- `direction` (deposit/withdraw)
- `chain`
- `amount`
- `asset`
- `from` / `to` identifiers
- `created_at`

Minimum required proof fields:
- driver-specific proof payload (opaque), plus:
  - `proof_type`
  - `external_ref`
  - `observed_at`

Minimum required settlement fields:
- `settlement_ok`
- `settlement_id`
- `finalized_at`
- driver name/version
- any rejection reason (if rejected)

## Verification procedure
1) Fetch the finalized intent object from Bridge Adapter.
2) Fetch the policy head used at finalize time.
3) Reconstruct `BridgeAuditEnvelopeV1`.
4) Canonicalize JSON.
5) SHA-256 → hex.
6) Compare to stored `audit_digest_hex`.

## Security considerations
- The digest is not a signature. If you need non-repudiation, sign the digest with an Ed25519 key and publish the signature as part of the receipt.
- Canonicalization must be deterministic; otherwise auditors cannot reproduce digests.
