# Federated Value Ledger Protocol v1 (Rhelma 6)

## Scope
Defines a federation overlay for value credits:
- Transaction log replication
- Quorum governance
- Quorum signed receipts (bridge-ready)

## Data model
### Transaction (Tx)
- `tx_id`: sha256(canonical_json(tx_body_without_sig))
- `issuer_peer_id`
- `issued_at_unix`
- `subject_id` (e.g., node:<id>, citizen:<id>, org:<id>)
- `delta` (i64)
- `reason`
- `tags` (optional)
- `sig_b64` (Ed25519 over canonical tx body)

**Rule:** balances are computed from the set-union of unique `tx_id`s.
No tx can be removed; reversals are done via compensating tx.

## APIs
### Public
- `GET /v1/credits/{subject_id}` -> {balance, tx_count, last_seen}
- `POST /v1/receipts/verify` -> verifies multi-signer receipt

### Federation
- `GET /v1/federation/snapshot?since=<cursor>` -> {cursor, txs[]}
- `POST /v1/federation/push` -> accept tx set (idempotent)

### Governance
- `POST /v1/gov/proposals` -> create proposal (rotation/bridge enablement)
- `POST /v1/gov/proposals/{id}/sign` -> add signer signature
- `POST /v1/gov/proposals/{id}/commit` -> commit if quorum met

### Admin (bootstrap only)
- `POST /v1/admin/tx` -> issue signed tx (requires admin token)

## Quorum receipts
Receipt includes:
- receipt_id, subject_id, amount, issued_at, purpose, tx_refs[]
- signatures: [{signer_pubkey, sig_b64}]
- quorum: N required

Any peer can verify the receipt using the signer set config.

## Anti-abuse / Sybil
- Default: run federation only across attested registrars / trusted operators.
- If used in public, require:
  - stake / bonding
  - reputation-weighted peer selection
  - rate limits and evidence requirements for high-impact spends.
