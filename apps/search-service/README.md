# search-service

## Overview
Hybrid search service (vector + full-text) backed by Qdrant + Meilisearch.

## Ownership
- **Owner:** Search
- **Tier:** dev | staging | prod
- **Startup dependencies:** see `.env.example` + docker-compose.dev.yml
- **Data safety:** service-owned DB state only; keep uploads/indices out of git

## Run
Recommended for local dev:
```bash
bash scripts/dev/run-social-mvp.sh
```

Run the service alone:
```bash
cargo run -p search-service
```

## Configuration
Source of truth: `.env.example`.

Key variables:
- `RHELMA_SEARCH_LISTEN_ADDR`
- `RHELMA_SEARCH_QDRANT_URL`
- `RHELMA_SEARCH_MEILI_URL`
- `RHELMA_SEARCH_DEFAULT_INDEX`

## Endpoints
Health + readiness:
- `GET /healthz`
- `GET /readyz`

Metrics:
- `GET /metrics` (Prometheus)

Primary API: see `docs/openapi/search-service.yaml`.

## Observability
- Tracing: W3C Trace Context (`traceparent`).
- Logs: structured; include `request_id` / `trace_id` on errors.
- Metrics: `/metrics` when enabled.

## Security
Follow `docs/contract/v6.0/05_SECURITY_v6.0.md`.


## Verification
```bash
cargo test -p search-service
```
