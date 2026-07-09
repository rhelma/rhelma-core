# OpenAPI standard (Rhelma6)

This document defines the minimum OpenAPI requirements for Rhelma HTTP services.

## File location
- `docs/openapi/<service>.yaml`

## Minimum required fields
- `openapi: 3.0.3`
- `info.title`, `info.version`
- At least one `servers` entry

## Minimum required paths
- `GET /healthz`
- `GET /metrics` (if the service exposes Prometheus metrics)

## Rhelma extensions
Use these vendor extensions:
- `x-rhelma-service: <service-name>`
- `x-rhelma-internal: true|false` (on operations)
- `x-rhelma-auth: jwt|mtls|none` (on operations)

## Security blocks
If the endpoint requires auth, define:
- `components.securitySchemes` (JWT bearer or mTLS)
- `security` on each protected operation

## Operational notes
- Keep request/response schemas low-cardinality and stable.
- Avoid including any PII fields unless explicitly required and approved.
- Prefer shared schema refs where possible.
