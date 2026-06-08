# sandbox-runner

Service for the Rhelma platform.

## Contract

This component MUST comply with **Rhelma Contract v6.0**. See `docs/contract/v6.0/00_INDEX_v6.0.md`.

## Quickstart

```bash
cargo run -p sandbox-runner
```

## Configuration

Service-specific environment variables detected in source:

| Variable | Purpose |
|---|---|
| `RHELMA_KAFKA_BROKERS` |  |
| `RHELMA_KAFKA_TOPIC_PREFIX` |  |
| `RHELMA_SANDBOX_RUNNER__ALLOWED_COMMAND_PREFIXES` |  |
| `RHELMA_SANDBOX_RUNNER__COMMAND_TIMEOUT_MS` |  |
| `RHELMA_SANDBOX_RUNNER__DOCKER_CPUS` |  |
| `RHELMA_SANDBOX_RUNNER__DOCKER_ENABLED` |  |
| `RHELMA_SANDBOX_RUNNER__DOCKER_EXTRA_ARGS` |  |
| `RHELMA_SANDBOX_RUNNER__DOCKER_IMAGE` |  |
| `RHELMA_SANDBOX_RUNNER__DOCKER_MEMORY` |  |
| `RHELMA_SANDBOX_RUNNER__DOCKER_NETWORK` |  |
| `RHELMA_SANDBOX_RUNNER__DOCKER_USER` |  |
| `RHELMA_SANDBOX_RUNNER__FORBIDDEN_PATH_PREFIXES` |  |
| `RHELMA_SANDBOX_RUNNER__KAFKA_BROKERS` |  |
| `RHELMA_SANDBOX_RUNNER__KAFKA_GROUP_ID` |  |
| `RHELMA_SANDBOX_RUNNER__KAFKA_TOPIC_PREFIX` |  |
| `RHELMA_SANDBOX_RUNNER__MAX_LOG_BYTES` |  |
| `RHELMA_SANDBOX_RUNNER__MAX_PATCH_BYTES` |  |
| `RHELMA_SANDBOX_RUNNER__REDACTED_ENV_PREFIXES` |  |
| `RHELMA_SANDBOX_RUNNER__SERVICE_NAME` |  |
| `RHELMA_SANDBOX_RUNNER__WORKSPACE_ROOT` |  |

## Interfaces

### HTTP endpoints

- `GET /healthz` — health
- `GET /readyz` — readiness
- `GET /metrics` — Prometheus metrics (if enabled)

Check `src/routes/` (or the service router) for the authoritative list.

### Events

If this service produces/consumes events (Kafka/NATS), see `docs/contract/v6.0/04_EVENT_DRIVEN_v6.0.md` and the `docs/contract/v6.0/specs/` schemas.

## Observability

- Tracing: OpenTelemetry / W3C Trace Context (`traceparent`).
- Metrics: `/metrics` when enabled.
- Logs: structured logs; never log secrets.

## Security

Follow `docs/contract/v6.0/05_SECURITY_v6.0.md`. In particular: sanitize errors, validate inputs, enforce authZ on admin routes.

## Development

```bash
cargo test -p sandbox-runner
```

## Overview
Service for the Rhelma platform.

## Ownership
- **Owner:** TBD
- **Tier:** dev | staging | prod
- **Startup dependencies:** see `.env.example`
- **Data safety:** see service docs

## Run
```bash
cargo run -p sandbox-runner
```

## Endpoints
- `GET /healthz`
- `GET /readyz`
- `GET /metrics`
