# Rate Limiting & Admission Control v1

## Principles
- Protect the registry and the network from bursts.
- Apply **progressive** restrictions based on trust.

## Policies
### Registration
- Per-IP: max registrations per hour
- Per-pubkey prefix: max attempts per hour
- Global circuit breaker:
  - if load exceeds threshold → PoW required

### Heartbeats
- Accept at most 1 heartbeat per node per interval (drop extras)

### Discover
- Cache responses
- Apply per-client rate limits

## Implementation note
Start with in-memory counters + TTL.
Upgrade later to shared storage (Redis) for multi-instance.
