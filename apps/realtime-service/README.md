# realtime-service

## Overview
Realtime WebSocket service for channels/rooms (dev default allows anonymous).

## Ownership
- **Owner:** Realtime
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
cargo run -p realtime-service
```

## Configuration
Source of truth: `.env.example`.

Key variables:
- `RHELMA_RT_LISTEN_ADDR (or RHELMA_RT_LISTEN_ADDR via .env.example)`
- `REALTIME_ALLOW_ANONYMOUS`
- `RHELMA_AUTH_* (only if anonymous=false)`

## Endpoints
Health + readiness:
- `GET /healthz`
- `GET /readyz`

Metrics:
- `GET /metrics` (Prometheus)

WebSocket surface and rooms: see `docs/openapi/realtime-service.yaml` (and code in `src/`).

## Observability
- Tracing: W3C Trace Context (`traceparent`).
- Logs: structured; include `request_id` / `trace_id` on errors.
- Metrics: `/metrics` when enabled.

## Security
Follow `docs/contract/v6.0/05_SECURITY_v6.0.md`.


## Verification
```bash
cargo test -p realtime-service
```
