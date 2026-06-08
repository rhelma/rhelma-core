# Security Policy Actions API v1 (Rhelma 6)

This document describes internal endpoints used to apply jury-approved outcomes.

## Authentication
All endpoints below require:
- Header: `x-registry-admin-token: <token>`

## POST /v1/internal/nodes/quarantine
Quarantine a node for a bounded TTL.

**Body**
```json
{
  "node_id": "base58-or-hex",
  "incident_id": "uuid",
  "reason": "string",
  "ttl_seconds": 86400
}
```

**Rules**
- `ttl_seconds` must be <= `RHELMA_NODE_REGISTRY__QUARANTINE__MAX_TTL_SECONDS`
- if omitted, registry uses `DEFAULT_TTL_SECONDS`

## POST /v1/internal/nodes/unquarantine
Remove quarantine status immediately.

**Body**
```json
{
  "node_id": "base58-or-hex",
  "incident_id": "uuid",
  "reason": "string"
}
```

## Discover behavior
`GET /v1/nodes/discover` excludes quarantined nodes by default.
Use `include_quarantined=true` for admin debug.
