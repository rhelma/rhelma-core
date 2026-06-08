# Value Ledger API v1 (Phase 18)

## Auth
Privileged endpoints require:
- header `x-admin-token: <RHELMA_VALUE_LEDGER__ADMIN_TOKEN>`

## Endpoints

### `GET /v1/credits/{subject_id}`
Returns balance.

Response:
```json
{ "subject_id": "node-123", "balance": 10, "updated_at": "2025-12-28T00:00:00Z" }
```

### `POST /v1/credits/earn` (admin)
Body:
```json
{ "subject_id":"node-123", "amount":10, "reason":"uptime", "reference":"hb:..." }
```

### `POST /v1/credits/spend` (admin)
Body:
```json
{ "subject_id":"node-123", "amount":5, "reason":"priority", "reference":"job:..." }
```

### `POST /v1/receipts/issue` (admin)
Body:
```json
{ "subject_id":"node-123", "amount":10, "purpose":"monthly reward", "external_ref":"cycle-1" }
```

Response:
```json
{
  "receipt_id":"...",
  "subject_id":"node-123",
  "amount":10,
  "purpose":"monthly reward",
  "issued_at":"...",
  "external_ref":"cycle-1",
  "signature_b64":"...",
  "signer_pubkey_b64":"..."
}
```

### `POST /v1/receipts/verify`
Body:
```json
{ "receipt": { /* SignedReceipt */ } }
```

Response:
```json
{ "valid": true }
```

## Notes
- Phase 18 uses a single signer (service key).
- Phase 19+ upgrades to multi-signer governance and federated snapshots.
