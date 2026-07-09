# Rhelma6 — Node Lifecycle & Registry

**Status:** Draft (Roadmap)

This document defines how nodes join, advertise capabilities, and stay registered.

## 1) Node identity

Each node has:

- **NodeId**: UUIDv7
- **NodeKey**: Ed25519 keypair (hardware-backed if possible)
- **NodeProfile**: declared capabilities (CPU, RAM, GPU, region, roles)
- **Attestation**: proof that the node runs approved software in an expected environment (phase-based)

The registry stores NodeProfile + public key + reputation metrics.

## 2) Lifecycle states

1. **Candidate** — downloaded binary, not registered
2. **Registered** — identity created, keys generated, profile submitted
3. **Verified** — attestation verified, basic anti-sybil checks passed
4. **Active** — receiving tasks
5. **Suspended** — temporary ban, unhealthy, or policy violation
6. **Retired** — gracefully removed

## 3) Registration flow (Phase 1 bootstrap)

1. Node generates keypair + NodeId locally.
2. Node calls Bootstrap Coordinator:
   - `POST /v1/nodes/register`
   - sends NodeProfile + public key + proof-of-control signature
3. Coordinator returns:
   - registration receipt (signed)
   - bootstrap peer list (signed)
   - required policies (limits, allowed roles)

## 4) Heartbeats & health

Nodes send heartbeats:

- `POST /v1/nodes/heartbeat`
- includes: NodeId, timestamp, load, queue depth, optional metrics summary
- signed by node key

Health scoring uses:
- heartbeat freshness
- task success rate
- latency distribution
- anomaly flags

## 5) Capability discovery

Routing requires a fresh view of:
- role support (inference/retrieval/storage)
- per-role limits (tokens/sec, QPS, storage GB)
- region/residency compatibility

Initially: coordinator serves a directory.
Later: distributed discovery (gossip / DHT / CRDT).

## 6) Minimal data model (registry)

- nodes:
  - node_id
  - public_key
  - profile_json
  - status
  - first_seen_at / last_seen_at
  - attestation_level
  - reputation_score
  - value_balance (if used)
  - ban_reason (optional)

## 7) Privacy and safety

- Do not store personal data of node operators by default.
- If contact info is needed (for ops), store it encrypted and access-controlled.
