# Rhelma6 Release Gate

The release gate is a **single command** that runs the core checks required before promoting a build.

It produces a Markdown report under `benchmarks/out/` so the outcome can be attached to a PR or a release ticket.

## What it runs

1. `scripts/verify.*` (format + lint + unit tests + repo guards)
2. `scripts/rhelma6/smoke_core.*` (fast health checks for critical services)
3. Optional: a **quick** k6 signal (if `k6` is available)

## What the report includes

- A **Decision block** (GO/NO-GO checklist) to fill in during promotion
- Context: host, git revision (when available), tool versions
- Per-step logs (tail) + duration
- A strict Summary table + recommendation

## Linux / macOS

```bash
./scripts/rhelma6/release_gate.sh
```

Artifacts:
- `benchmarks/out/release_gate_report.md` (human-readable report)
- `benchmarks/out/release_gate_manifest.json` (machine-readable summary + sha256)
- `benchmarks/out/release_gate_pr_comment.md` (copy/paste into PR)
- `benchmarks/out/release_gate_go_no_go_block.md` (copy/paste into release ticket)
- `benchmarks/out/release_gate_*.log` (step logs)

## CI / partial runs

For CI where the full stack isn't available, you can skip some steps:

- Skip smoke (required): `RHELMA_RELEASE_GATE_SKIP_SMOKE=1`  
  The report will mark the run as **INCOMPLETE** (not a GO decision).
- Skip load (optional): `RHELMA_RELEASE_GATE_SKIP_LOAD=1`

Example:

```bash
RHELMA_RELEASE_GATE_SKIP_SMOKE=1 RHELMA_RELEASE_GATE_SKIP_LOAD=1 ./scripts/rhelma6/release_gate.sh
```

### Optional: OTEL propagation regression tests

If you want the release gate to also validate OpenTelemetry/W3C propagation for Kafka headers, enable:

```bash
RHELMA_RELEASE_GATE_OTEL_VERIFY=1 ./scripts/rhelma6/release_gate.sh
```

Under the hood this runs `scripts/verify_otel.sh` (with `RHELMA_VERIFY_OTEL=1`) as a **separate step**, so the default `scripts/verify.sh` stays fast.

The `release_gate_ci.yml` workflow enables this by default.

### GitHub Actions

This repo includes two workflows:

- `release_gate_ci.yml` runs on **pull requests** and `main` pushes in **CI mode** (smoke/load skipped).
- `release_gate.yml` is a **workflow_dispatch** entrypoint for full runs (optionally enabling smoke/load
  when you can provide reachable service URLs).

## Windows (PowerShell)

```powershell
./scripts/rhelma6/release_gate.ps1 -ApiGatewayUrl "http://127.0.0.1:3000" -AiOrchUrl "http://127.0.0.1:4000" -NodeRegistryUrl "http://127.0.0.1:8090"
```

### Optional: include k6

If you have `k6` installed, pass `-K6BaseUrl`:

```powershell
./scripts/rhelma6/release_gate.ps1 -K6BaseUrl "http://127.0.0.1:3000"
```

## Pass/Fail meaning

The release gate exits non-zero if any **required** step fails.

- ✅ PASS: safe to continue to canary/rollout (subject to change management).
- ❌ FAIL: treat as **NO-GO**, investigate the failing section in the report, fix, and re-run.
