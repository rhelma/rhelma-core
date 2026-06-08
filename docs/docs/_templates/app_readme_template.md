# <service-name>

## Overview

What this service does, in one paragraph.

## Ownership + lifecycle

- **Owner:** (team/person)
- **Tier:** (dev | staging | prod)
- **Startup dependencies:** (e.g., NATS, DB, upstream HTTP)
- **Shutdown behavior:** graceful shutdown + max drain time
- **Data safety:** what is persisted; what is ephemeral

## Run (local)

```bash
cargo run -p <crate-name>
```

## Configuration

- Source of truth: `.env.example`
- Naming conventions: `docs/reference/ENVIRONMENT_VARIABLES.md`

List the **key env vars** here (service-specific + critical shared ones).

### Recommended baseline

- Health: `GET /healthz` (liveness)
- Readiness: `GET /readyz` (readiness)
- Metrics: `GET /metrics` (Prometheus)
- Tracing: W3C Trace Context (`traceparent`) propagated across calls
- Correlation: stable `request_id` across service boundaries

## Endpoints

List public endpoints (health, metrics, and main API surface).

### Health + readiness

| Endpoint | Purpose | Notes |
|---|---|---|
| `GET /healthz` | Liveness | Must be fast; no downstream calls |
| `GET /readyz` | Readiness | May validate critical dependencies |

### Metrics

| Endpoint | Purpose |
|---|---|
| `GET /metrics` | Prometheus scrape |

## Observability

- `GET /metrics`
- tracing/log fields worth knowing

### Log/trace fields to standardize

- `request_id`
- `trace_id` / `span_id`
- `tenant_id` (if applicable)
- `realm_id` (if applicable)
- `route` (normalized endpoint)

## Security / policy notes

Anything that operators must not forget (PII rules, auth requirements, allow-lists, etc).

### Admin surfaces

If this service exposes sensitive routes (admin, governance, secrets, policy write paths):

- Prefer serving those routes from the **Rust admin surface** (e.g., via `multi-frontend` under `/admin`).
- Require at least one of:
  - `RHELMA_ADMIN_TOKEN`
  - mTLS allowlist / client cert fingerprint verification
  - signed attestations / quorum for write actions

## Verification

- Recommended: `./scripts/verify_pre_frontend.sh`
- Service tests (if any): `cargo test -p <crate-name>`

## Troubleshooting

- Common failure modes
- How to validate env/config quickly
- How to inspect metrics and traces
