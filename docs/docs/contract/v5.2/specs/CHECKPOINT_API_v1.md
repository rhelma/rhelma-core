# Checkpoint API v1 (gossip-discovery)

## POST /v1/checkpoints/push

### Body
```json
{
  "domain": "bridge-audit",
  "kind": "merkle-root",
  "root_hex": "64-lower-hex",
  "ts_unix": 1730000000,
  "signer_pubkey_b64": "BASE64(ed25519-pubkey)",
  "signature_b64": "BASE64(ed25519-signature over canonical payload)"
}
```

### Canonical payload for signing
UTF-8 bytes of:
```
domain=<domain>
kind=<kind>
root=<root_hex>
ts=<ts_unix>

```

### Response
- 200: accepted `{ "status":"ok", "checkpoint_id":"..." }`
- 400: invalid
- 401: signature invalid

## GET /v1/checkpoints/head?domain=...&kind=...

Returns:
```json
{
  "domain":"...",
  "kind":"...",
  "root_hex":"...",
  "ts_unix":1730000000,
  "signer_peer_id":"...",
  "checkpoint_id":"..."
}
```

## GET /v1/checkpoints/history?domain=...&kind=...&limit=...

Returns list of accepted checkpoints (newest first).
