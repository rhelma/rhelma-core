# file-storage-service

## Overview
Multi-tenant file storage service (local or S3 provider).

## Ownership
- **Owner:** Storage
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
cargo run -p file-storage-service
```

## Configuration
Source of truth: `.env.example`.

Key variables:
- `RHELMA_FILE_STORAGE__LISTEN_ADDR`
- `RHELMA_FILE_STORAGE__DATABASE_URL`
- `RHELMA_FILE_STORAGE__PROVIDER`
- `RHELMA_FILE_STORAGE__LOCAL_ROOT`
- `RHELMA_FILE_STORAGE__S3_ENDPOINT (when provider=s3)`

## Endpoints
Health + readiness:
- `GET /healthz`
- `GET /readyz`

Metrics:
- `GET /metrics` (Prometheus)

Primary API: see `docs/openapi/file-storage.yaml`.

## Observability
- Tracing: W3C Trace Context (`traceparent`).
- Logs: structured; include `request_id` / `trace_id` on errors.
- Metrics: `/metrics` when enabled.

## Security
Follow `docs/contract/v6.0/05_SECURITY_v6.0.md`.


## Verification
```bash
cargo test -p file-storage-service
```
