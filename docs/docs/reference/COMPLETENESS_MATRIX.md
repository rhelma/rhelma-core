# Completeness matrix

This document is the **single place** to track platform completeness in a way that is aligned with:

- Contract v6.0 **Golden rules** (`docs/contract/v6.0/rules/`)
- Service README standard (`docs/contributing/SERVICE_README_STANDARD.md`)
- Verification & gates (`docs/contributing/VERIFICATION_AND_GATES.md`)
- Known stubs and phased wiring (`docs/reference/KNOWN_STUBS_AND_PHASED_WIRING.md`)

The point is **design-first correctness** (contracts, trust boundaries, observability, event discipline),
before moving into "MVP production hardening".

## What "complete" means (baseline)

For each service under `apps/*`, we consider it "baseline-complete" when it has:

1) **Contract alignment**
   - Any public HTTP API or event produced/consumed is defined in `docs/contract/v6.0/specs/`.
   - Consumers subscribe to allow-listed topics (no wildcard/regex in prod).

2) **Observability correctness**
   - Request context propagation across HTTP/event boundaries.
   - Daemon services: `GET /healthz` (or equivalent) and `GET /metrics` (Prometheus) exposed.
   - Non-daemon apps (static sites / CLI tools) are exempt: `docs/reference/non_daemon_apps.txt`.
   - Low-cardinality metrics labels.

3) **Security boundary clarity**
   - AuthN/AuthZ decisions are explicit.
   - No secrets/PII in logs.

4) **Operational readiness (dev-grade)**
   - Standard README sections.
   - A runbook entry **or** a clear link to the closest applicable runbook.
   - At least smoke tests (unit/integration or E2E harness coverage).

5) **API surface clarity**
   - If the service exposes HTTP, it must have an OpenAPI spec scaffold at `docs/openapi/<service>.yaml`.
   - Source of truth for which apps are considered "HTTP services": `docs/reference/http_services.txt`.

## How to measure

### Quick report (recommended)

Linux/macOS/WSL:

```bash
bash scripts/dev/completeness-report.sh
```

Windows:

```powershell
.\scripts\dev\completeness-report.ps1
```

### Making it a gate

Set `RHELMA_VERIFY_COMPLETENESS=1` and run:

```bash
bash scripts/verify.sh
```

or

```powershell
.\scripts\verify.ps1
```

## Service matrix (fill as you wire / harden)

Legend:

- ✅ = done
- ⚠️ = partial / review
- ❌ = missing
- — = not applicable

> Tip: Treat this as **living documentation**. If a new invariant becomes important, add a column
> and then express it as a deterministic verification step.

| Service | README standard | Health/metrics documented | Runbook exists | OpenAPI spec | Notes |
|---|---:|---:|---:|---:|---|
| admin-web | ✅ | — | ✅ | — | static UI (served by multi-frontend) |
| agent-handoff | ⚠️ | ✅ | ✅ | ✅ | needs tests coverage |
| ai-companion | ✅ | ✅ | ✅ | ✅ | /metrics not wired yet (doc only) |
| ai-orchestrator | ⚠️ | ✅ | ✅ | ✅ | |
| api-gateway | ⚠️ | ✅ | ✅ | ✅ | |
| bridge-adapter | ⚠️ | ✅ | ✅ | ✅ | |
| digital-family-vault | ⚠️ | ✅ | ✅ | ✅ | |
| edge-worker | ⚠️ | ✅ | ✅ | — | non-HTTP daemon |
| file-storage | ⚠️ | ✅ | ✅ | ✅ | |
| gossip-discovery | ⚠️ | ✅ | ✅ | ✅ | |
| guardian-agent | ⚠️ | ✅ | ✅ | ✅ | |
| rhelma-attestation-verifier | ✅ | — | ✅ | — | CLI tool |
| rhelma-bridge-drivers | ⚠️ | ✅ | ✅ | — | non-HTTP daemon |
| rhelma-governance-signer | ✅ | — | ✅ | — | CLI tool |
| rhelma-node | ⚠️ | ✅ | ✅ | — | non-HTTP daemon |
| mni-rag | ⚠️ | ✅ | ✅ | ✅ | |
| multi-frontend | ✅ | ✅ | ✅ | ✅ | |
| node-registry | ⚠️ | ✅ | ✅ | ✅ | |
| patch-applier | ⚠️ | ✅ | ✅ | — | event consumer |
| realm-hub | ✅ | ✅ | ✅ | ✅ | |
| realtime-service | ⚠️ | ✅ | ✅ | ✅ | |
| region-health-aggregator | ✅ | ✅ | ✅ | ✅ | |
| sandbox-runner | ⚠️ | ✅ | ✅ | — | event consumer |
| search-service | ⚠️ | ✅ | ✅ | ✅ | |
| security-governance | ⚠️ | ✅ | ✅ | ✅ | |
| value-ledger | ⚠️ | ✅ | ✅ | ✅ | |
| value-ledger-federation | ⚠️ | ✅ | ✅ | ✅ | |
| web | ⚠️ | ✅ | ✅ | — | static assets |
