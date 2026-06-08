# node-registry

## Overview
Registry for Rhelma nodes (manifests, reputation, attestation policy gates).

## Ownership
- **Owner:** Platform / Governance
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
cargo run -p node-registry
```

## Configuration
Source of truth: `.env.example`.

Key variables:
- `RHELMA_BIND_HOST`
- `RHELMA_BIND_PORT`
- `RHELMA_NODE_REGISTRY__POLICY__REQUIRE_MANIFEST_SIGNATURE`
- `RHELMA_NODE_REGISTRY__POLICY__REQUIRE_ATTESTATION_VERIFICATION`

## Endpoints
Health + readiness:
- `GET /healthz`
- `GET /readyz`

Metrics:
- `GET /metrics` (Prometheus)

OpenAPI: `docs/openapi/node-registry.yaml` (generated OpenAPI available with `--features openapi`).

## Observability
- Tracing: W3C Trace Context (`traceparent`).
- Logs: structured; include `request_id` / `trace_id` on errors.
- Metrics: `/metrics` when enabled.

## Security
Follow `docs/contract/v6.0/05_SECURITY_v6.0.md`.


## Verification
```bash
cargo test -p node-registry
```
