# End-to-end testing

Rhelma has two complementary “end-to-end” layers. The goal is to keep **developer feedback fast**,
while still having a path to validate **real processes talking over real transports**.

## In-process contract/integration tests

These tests run **without booting Docker infra**. They validate:

- Contract v5.2 header handling and propagation
- Outbound HTTP context propagation (request-id/correlation-id/residency + traceparent)
- WS contract expectations where applicable

Run:

```bash
bash scripts/e2e_local.sh
```

Windows:

```powershell
./scripts/e2e_local.ps1
```

## Live stack smoke (real processes)

This tier is meant to catch “it works on my machine” problems by validating runtime behavior:

- Docker infra boots cleanly
- Selected services become live/ready
- API gateway can proxy to upstreams and preserves Rhelma headers

Run with infra + selected services:

```bash
RHELMA_E2E_MODE=live RHELMA_E2E_BOOT=1 RHELMA_E2E_SERVICES=api-gateway,search-service bash scripts/e2e_local.sh
```

Notes:

- `RHELMA_E2E_SERVICES` is a comma-separated list of **Cargo package names**.
- The harness writes logs to `.e2e/logs/`.
- If you already run services in another terminal, set `RHELMA_E2E_BOOT=0` and just run the smoke.
- The harness automatically sets `RHELMA_SMOKE_SKIP_*` flags so smoke checks only target the services you selected.

## Optional functional flows

By default, smoke checks verify **health/ready/metrics** endpoints. You can optionally enable
small “real flow” checks that exercise critical paths end-to-end:

### API gateway auth flow

```bash
RHELMA_E2E_MODE=live RHELMA_E2E_BOOT=1 RHELMA_E2E_SERVICES=api-gateway \
  RHELMA_SMOKE_AUTH_FLOW=1 \
  bash scripts/e2e_local.sh
```

This runs: `register -> login -> refresh` against the API gateway.

### Node registry lifecycle

```bash
RHELMA_E2E_MODE=live RHELMA_E2E_BOOT=1 RHELMA_E2E_SERVICES=node-registry \
  RHELMA_SMOKE_NODE_REGISTRY_FLOW=1 \
  bash scripts/e2e_local.sh
```

If admission PoW is enabled, the smoke runner will attempt to solve it. You can cap work:

```bash
RHELMA_SMOKE_POW_MAX_ITERS=2000000
```

## Design principles

- **No fake tests**: tests should verify observable behavior (HTTP/WS status, JSON shape, headers), not just “it compiles”.
- **Small surface first**: start with health + contracts, then expand to functional E2E flows.
- **Deterministic**: avoid time-sensitive assertions; use bounded retries and clear timeouts.

## Service groups & infra profiles

To avoid long command lines, the harness supports named service groups:

- `RHELMA_E2E_SERVICES=core` starts a practical "core" set:
  `api-gateway,ai-orchestrator,search-service,file-storage-service,realtime-service,node-registry`
- `RHELMA_E2E_SERVICES=all` starts a larger set of application packages (heavier).

Infra profiles are controlled via Docker Compose v2 profiles:

- Set `RHELMA_E2E_DOCKER_PROFILES=kafka,obs,s3` to force profiles.
- If unset, the harness auto-enables `kafka` when booting `ai-orchestrator` or `patch-applier`.
- Optional toggles:
  - `RHELMA_E2E_ENABLE_OBS=1`
  - `RHELMA_E2E_ENABLE_S3=1`

Smoke checks default to the common local ports:

- API Gateway: `http://127.0.0.1:3000`
- AI Orchestrator: `http://127.0.0.1:4000`
- Search: `http://127.0.0.1:8082`
- File storage: `http://127.0.0.1:3005`
- Realtime: `http://127.0.0.1:9000`
- Node registry: `http://127.0.0.1:8090`
- LLM node: `http://127.0.0.1:8088`
