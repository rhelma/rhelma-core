# Scripts

This directory is focused on **verification, gates, and dev ergonomics**.

## Primary entrypoints

- `bootstrap.(sh|ps1)` — first-time developer setup (creates .env, generates keys)
- `setup/preflight.(sh|ps1)` — quick environment sanity-check (recommended before first verify)

- `verify_all.(sh|ps1)` — recommended "one command" gate (structure + verification + guards)
- `test_all.(sh|ps1)` — run `verify_all` + in-process e2e (optionally live smoke)

- `verify_pre_frontend.(sh|ps1)` — legacy pre-frontend gate (kept for compatibility)
- `verify.(sh|ps1)` — core workspace verification (fmt + clippy + tests)
- `verify_observability.(sh|ps1)` — observability-focused verification
- `smoke_local.(sh|ps1)` and `smoke_staging.(sh|ps1)` — quick HTTP smoke checks against running services
- `dev/stub-report.(sh|ps1)` — list phase-scoped "stub" code paths (planning aid)
- `dev/completeness-report.(sh|ps1)` — docs/readme/runbook/openapi completeness snapshot (planning + optional gate)

## Layout

We keep **stable root entrypoints** for CI/docs, while implementations are organized under subfolders:

- `guards/` — deterministic contract/anti-drift guards
- `dev/` — local run helpers
- `setup/` — key generation and hook installation
- `smoke/` — smoke implementations (root names are wrappers)
- `e2e/` — e2e harness implementations (root names are wrappers)
- `rhelma6/` — rhelma6 deploy-related helpers

> The root-level scripts (e.g. `contract_guard.sh`) are thin wrappers that call into these folders.

## Guards

Guards are fast, deterministic checks that prevent contract drift:

- `contract_guard.*`
- `env_contract_guard.*`
- `header_contract_guard.*`
  - Optional strict mode: set `RHELMA_GUARDS_STRICT_X_MACH=1` to require that *only* `x-rhelma-request-id` and `x-rhelma-correlation-id` appear in app surfaces (useful for hardening public ingress).

- `guards/env_contract_guard_allowlist.txt` — temporary allowlist of legacy direct env reads (migrate over time)
- `event_contract_guard.*`
- `uuidv7_guard.*`
- `scrapeability_guard.*`
- `metrics_cardinality_guard.*`
- `outbound_http_context_guard.(py|ps1|sh)`
- `todo_guard.*`

## Verification documentation

- `docs/contributing/VERIFICATION_AND_GATES.md`
- `docs/contract/` (v5.2 rules)

## Resource tuning (avoiding laptop hangups)

`verify.(sh|ps1)` runs `cargo clippy` and `cargo test` across the whole workspace.
Cargo will parallelize aggressively by default, which can overwhelm low-RAM machines.

Both verify entrypoints now auto-tune concurrency conservatively, and you can override them:

> Note: when `CI` is set, the scripts prefer speed and will use higher defaults (up to 16).

- `RHELMA_VERIFY_JOBS` — max parallel compilation jobs
- `RHELMA_VERIFY_TEST_THREADS` — test thread count (passed to `--test-threads`)
- `RHELMA_VERIFY_LOW_RESOURCE=1` — force `jobs=1` and `threads=1`

You can also use the standard knobs:

- `CARGO_BUILD_JOBS`
- `RUST_TEST_THREADS`
- `RAYON_NUM_THREADS`

Examples:

```powershell
$env:RHELMA_VERIFY_LOW_RESOURCE="1"
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\verify.ps1
```

```bash
RHELMA_VERIFY_JOBS=2 RHELMA_VERIFY_TEST_THREADS=1 bash scripts/verify.sh
```

## Control Plane + Social (local)

Bring up Postgres/Redis + control-service + social-service + api-gateway and register a local node:

```bash
chmod +x scripts/dev/run-control-plane-social.sh
scripts/dev/run-control-plane-social.sh
```

If you need to re-register the node:

```bash
chmod +x scripts/dev/register-local-social-node.sh
scripts/dev/register-local-social-node.sh
```

## Social MVP

- `scripts/dev/run-social-mvp.sh` — run minimal social stack (docker + core services)
- `scripts/dev/run-social-mvp.ps1` — PowerShell version

