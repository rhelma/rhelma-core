# Guardian Approval Signing (helper)

To approve a recovery session, the guardian signs this canonical JSON:

```json
{
  "session_id": "<uuid>",
  "payload_digest_hex": "<sha256 hex>",
  "intent": "approve_recovery_v1"
}
```

Signature is Ed25519 over UTF-8 JSON bytes (no whitespace guarantees).
The service uses `serde_json::to_vec` canonicalization for the verifier message, so guardians should do the same.

In early phases, you can create a small helper CLI locally to sign.
