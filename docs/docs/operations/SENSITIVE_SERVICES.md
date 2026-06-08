# Sensitive services improvement plan

This document is a **practical checklist** for hardening and standardizing Rhelma services.

Normative requirements live in `docs/contract/v6.0/`.

## Priority 0: Edge + Identity + Messaging
These components define the platform’s trust and communication surface:

1. **api-gateway** (`apps/api-gateway/`)
2. **rhelma-auth** (`crates/rhelma-auth/`) and any auth service endpoints
3. **rhelma-event / rhelma-event-kafka / rhelma-event-kafka-agent** (event envelope + subscription safety)
4. **ai-orchestrator** (`apps/ai-orchestrator/`) (policy enforcement + audit)
5. **security-governance** (`apps/security-governance/`) (policy actions + enforcement)

## Standardization checklist (apply to each service)

### Interfaces
- [ ] `GET /healthz`, `GET /readyz`, `GET /metrics` (where applicable)
- [ ] Stable JSON error model (sanitized; no internal details)
- [ ] Versioned API routes (`/v1/...`) for public-facing endpoints

### Security
- [ ] Input validation + request size limits
- [ ] AuthN/AuthZ for any admin/debug endpoints
- [ ] Secrets never logged; config validated at startup

### Observability
- [ ] W3C trace context (`traceparent`) propagated end-to-end
- [ ] Correlation: `request_id` consistently present in logs/events
- [ ] Metrics include request counts/latency/error rates

### Eventing
- [ ] Explicit topic allow-lists (no regex subscriptions)
- [ ] Event envelope carries trace + request_id
- [ ] Schema versioning; consumers protected from breaking changes

## Next steps
Start with `apps/api-gateway/README.md` and align implementation to `docs/architecture/COMMUNICATIONS.md`.
