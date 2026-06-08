# Policy Routing State Spec v1 (Rhelma6)

## Purpose
Define a canonical representation of node routing state that can be:
- used by registries for coherent routing,
- replicated across federated registries,
- and eventually propagated via gossip/DHT.

## Entities

### NodeRoutingState
- `node_id: string`
- `status: "active" | "quarantined" | "banned"`
- `routing_weight: float` (0.0..1.0)
- `risk_score: int` (0..100)
- `attested: bool`
- `reputation: int` (0..1000)
- `dampened_until: RFC3339?`
- `updated_at: RFC3339`
- `updated_by: "ops" | "guardian" | "judge" | "jury" | "system"`

### RoutingSnapshot
- `snapshot_id: string` (content-addressed)
- `created_at: RFC3339`
- `merkle_root: string` (hex)
- `nodes_count: int`
- `signature: string?` (ed25519)
- `states: NodeRoutingState[]` (or a compact encoding)

## Coherence Rules (normative)
1. `status=banned` implies `routing_weight=0.0`.
2. `status=quarantined` implies discover exclusion by default.
3. Automated sources (guardian/judge) may set dampening only with TTL <= MAX_TTL.
4. Jury decisions can set/clear quarantine and optionally set a cool-down dampening.
5. If conflicts occur, resolve by precedence:
   - banned > quarantined > active
   - jury > ops > judge > guardian
   - newer updated_at wins within same precedence

## Replication
- Snapshots are merged by the coherence rules.
- If signature is present, peers MUST verify before applying.
