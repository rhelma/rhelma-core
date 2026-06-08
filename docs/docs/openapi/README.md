# OpenAPI (HTTP surface) — Rhelma6

This folder contains **OpenAPI 3.0** specifications for Rhelma HTTP-facing services.

## Why
- Contract-first external surfaces
- SDK generation (future)
- Security review / threat modeling
- Operational clarity (what exists, what is internal)

## Conventions
- One file per service: `docs/openapi/<service>.yaml`
  - Source of truth for which apps are considered "HTTP services": `docs/reference/http_services.txt`
- Prefer **public** endpoints only. Internal-only endpoints should be marked with:
  - `x-rhelma-internal: true`
- All specs must include at least:
  - a liveness/health endpoint (commonly `/healthz` or `/health`)
  - a metrics endpoint (`/metrics`) if the service exposes one

## Status
These files start as **scaffolding**.
As part of the “Design & Rules completion” phase, each service owner should:
1) Verify paths, query params, and schemas against the real router.
2) Add request/response schemas.
3) Add auth (JWT/RBAC) sections where applicable.

