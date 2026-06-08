# Agent Liquidity & Migration Protocol (v1)

This document defines a minimal protocol for **Agent Leases** and **Agent Handoffs** to enable
agent mobility across nodes in Rhelma 6.

## Concepts

### Agent
A long-lived logical entity (conversation agent, guardian agent, automation agent, etc).

### Lease
A short-lived binding from `agent_id` → `node_id` that:
- enables **sticky routing**
- reduces cold-starts
- provides an explicit TTL for safe migration

Leases are **verifiable** via a signed token.

### Handoff
A two-step migration:
1) **prepare** (from-node produces a checkpoint reference)
2) **commit** (to-node confirms it can resume from checkpoint)
Rollback is supported if commit fails.

Handoffs are **verifiable** via a signed token.

## Data model (MVP)
- `agent_id`: string (stable ID)
- `node_id`: string (target compute node)
- `checkpoint_ref`: string (URI-like reference, e.g. `rhelma://fs/<hash>`)

## Security model (MVP)
- Server signs tokens using Ed25519.
- Clients present the token when renewing a lease or committing/rolling back a handoff.
- Tokens bind:
  - ids (lease_id/handoff_id)
  - agent_id
  - node ids
  - expiry timestamp
- Expired tokens are rejected.

## Endpoints

### Issue Lease
`POST /v1/leases/issue`

Request:
```json
{ "agent_id": "agent_123", "node_id": "node_A", "ttl_sec": 600 }
```

Response:
```json
{ "lease_id": "...", "agent_id": "...", "node_id": "...", "expires_at_unix": 0, "issued_at_unix": 0, "token_b64": "..." }
```

### Renew Lease
`POST /v1/leases/renew`

Request:
```json
{ "lease_id": "...", "ttl_sec": 600, "token_b64": "..." }
```

### Prepare Handoff
`POST /v1/handoff/prepare`

Request:
```json
{
  "agent_id": "agent_123",
  "from_node_id": "node_A",
  "to_node_id": "node_B",
  "lease_id": "...",
  "lease_token_b64": "...",
  "checkpoint_ref": "rhelma://fs/sha256:...",
  "ttl_sec": 600
}
```

### Commit Handoff
`POST /v1/handoff/commit`

Request:
```json
{ "handoff_id": "...", "token_b64": "..." }
```

### Rollback Handoff
`POST /v1/handoff/rollback`

Request:
```json
{ "handoff_id": "...", "token_b64": "...", "reason": "optional" }
```

## Routing integration (recommended)
- Orchestrator caches lease bindings for TTL.
- On node failure: attempt handoff using latest checkpoint.
- Quarantine/soft-quarantine MUST override any lease preference.

## Future improvements
- Multi-signer quorum for tokens (federated control)
- Binding to RequestContext (tenant/residency)
- Checkpoint integrity verification (hash + signature)
