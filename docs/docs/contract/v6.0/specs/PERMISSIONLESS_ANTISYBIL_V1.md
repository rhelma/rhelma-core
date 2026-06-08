# Permissionless Anti-Sybil v1

## Threat model
- Mass node registrations to gain routing share (Sybil swarm)
- Registration spam to exhaust registry resources
- Reputation inflation via collusion

## Controls
### A) PoW challenge (registration throttle)
- Registry issues a short-lived challenge:
  - `challenge_id`, `nonce`, `difficulty`, `expires_at`
- Client submits `solution` where:
  - `sha256(nonce || solution)` has `difficulty` leading zero bits

### B) Refundable deposit holds
- A node can lock a small deposit in Value Ledger Federation.
- Misbehavior triggers slashing/quarantine; otherwise deposit can be released.

### C) Time-based trust
- Reputation gain is time-gated.
- New identities start with a hard cap on routing weight.

### D) Challenge tasks
- Random tasks requiring deterministic verification (latency, correctness checks).

## Recommended defaults
- PoW expires: 5 minutes
- Difficulty: adaptive based on registry load
- Deposit: small (configurable)
- New node routing weight cap: 0.05–0.10 until verified
