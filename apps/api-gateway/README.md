# api-gateway

## Overview
`api-gateway` is the public HTTP entrypoint for Rhelma. It routes tenant/realm-aware requests to the right
backend service (e.g. `social-service`, `search-service`, `file-storage`, `realtime-service`) using the
control-plane discovery data.

## Ownership
- **Owner:** Platform / Edge
- **Tier:** dev | staging | prod
- **Startup dependencies:** `control-service`, Redis (rate limit/session), Postgres (optional, depending on features)
- **Data safety:** stateless; never persist secrets to disk

## Run
Recommended for local dev: use the bundled profile script:

```bash
bash scripts/dev/run-social-mvp.sh
```

Or run the gateway alone:

```bash
cargo run -p api-gateway
```

## Configuration
Source of truth: `.env.example`.

Key variables:
- `RHELMA_BIND_HOST`, `RHELMA_BIND_PORT` — listen address (default `0.0.0.0:3000`)
- `RHELMA_CONTROL_SERVICE_URL` — discovery source (default `http://127.0.0.1:8086`)
- `RHELMA_GATEWAY_DISCOVERY_CACHE_TTL_SECONDS` — cache TTL for discovery (dev default `30`)
- `RHELMA_REDIS__URL` — required for rate limiting in dev
- `RHELMA_ENV`, `RHELMA_REGION`, `RHELMA_SERVICE_VERSION` — required by strict env contract

## Endpoints
Health + metrics:
- `GET /health` (liveness)
- `GET /healthz` (alias)
- `GET /readyz` (readiness)
- `GET /metrics` (Prometheus)

Routing:
- `/social/*` → `social-service`
- `/search/*` → `search-service`
- `/files/*`  → `file-storage-service`
- `/realtime/*` → `realtime-service`

> Note: the authoritative surface is the OpenAPI scaffold at `docs/openapi/api-gateway.yaml`.

## Observability
- Propagates W3C Trace Context (`traceparent`) on outbound HTTP.
- Standard fields: `request_id`, `trace_id`, `span_id`, `realm/tenant`.
- Metrics are exposed at `/metrics` when enabled.

## Security
- Treat this as an **edge** component: validate inputs and do not leak internals in errors.
- Enforce authn/authz at the gateway boundary where possible (tenant headers, admin tokens).
- Follow: `docs/contract/v6.0/05_SECURITY_v6.0.md`.

## Verification
```bash
cargo test -p api-gateway
```
