# Rhelma Platform

## Docs

- Documentation index: `docs/INDEX.md`
- Contract v6.0: `docs/contract/v6.0/00_INDEX_v6.0.md`
- Rhelma6 program: `docs/architecture/OVERVIEW_RHELMA6.md`
- Open-source strategy: `docs/open-source/README.md`
- Public release checklist: `docs/open-source/RELEASE_CHECKLIST.md`
- Site plans: `docs/sites/rhelma-ir.md` and `docs/sites/asrnegar-ir.md`
- Release manifests: `OPEN_SOURCE_MANIFEST.md`, `COMMERCIAL_BOUNDARY.md`, and `SOCIAL_MANIFEST.md`

Rhelma is a **multi-tenant, multi-region, AI-native** platform built around:

- **Zero-trust** security defaults
- **Event-driven** workflows (Kafka/NATS-ready)
- **Observability-first** primitives (logs/traces/metrics)
- A **self-improvement loop** (proposal → evaluation → approval → apply)

This repository is aligned with **Rhelma Contract v6.0** (with v5.2 kept for compatibility).

## Open-source model

Rhelma follows an open-core operating model:

- Public core: reusable Rust crates, public service contracts, local development tooling, SDKs, examples, and documentation.
- Social product: Asrnegar is the operational social system built on top of the public Rhelma core.
- Commercial layer: customer-specific deployments, advanced administration, billing, private integrations, hosted operations, and enterprise support stay outside the public core.

Before publishing a public release, use `docs/open-source/RELEASE_CHECKLIST.md`.

## Repository layout

- `apps/` — runnable services (api-gateway, ai-orchestrator, search-service, ...)
- `crates/` — reusable libraries (rhelma-core, rhelma-auth, rhelma-config, ...)
- `observability/` — observability wiring (logger/tracing/metrics core)
- `docs/` — Rhelma6-first documentation + developer/ops guides
- `docs/contract/` — versioned production contracts & rules
- `infra/` — local infra (docker-compose, monitoring, provisioning)
- `scripts/` — verification helpers and smoke tests

## Quick start

Fastest path (recommended):

```bash
bash scripts/bootstrap.sh
bash scripts/run-world.sh
```

Docs: `docs/getting-started/QUICKSTART_MVP.md`

1) Configure environment variables:

- Start from `.env.example` and copy the needed variables into `.env`.
- See the full list in `docs/reference/ENVIRONMENT_VARIABLES.md`.

2) Run the local dev stack:

- See `docs/getting-started/LOCAL_DEV_STACK.md`.

3) Run a service:

```bash
cargo run -p ai-orchestrator
```

## Verification

Preferred (pre-frontend gate):

```bash
./scripts/verify_pre_frontend.sh
```

Windows:

```powershell
.\scripts\verify_pre_frontend.ps1
```

## Documentation

- Start here: `docs/README.md`
- Contract index: `docs/contract/v6.0/00_INDEX_v6.0.md`
- Policies & rules: `docs/contract/v6.0/rules/README.md`
