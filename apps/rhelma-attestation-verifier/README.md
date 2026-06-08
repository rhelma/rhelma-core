# rhelma-attestation-verifier

## Overview

`rhelma-attestation-verifier` is a small **CLI helper** for validating attestation evidence envelopes in a deterministic way.
It is intended for **dev/staging** pipelines and for wiring the attestation contract end-to-end.
Real deployments may replace this with environment-specific verification (TPM/SGX vendor tooling, policy engines, etc.).

## Ownership + lifecycle

- **Owner:** Security
- **Tier:** dev | staging
- **Startup dependencies:** none
- **Shutdown behavior:** exits when stdin is consumed
- **Data safety:** reads evidence from stdin; does not persist

## Run (local)

Verify evidence from stdin:

```bash
cat evidence.json | cargo run -p rhelma-attestation-verifier -- verify <kind> <node_id_hex>
```

Where `<kind>` is one of: `software`, `tpm`, `sgx` (others fall back to minimal sanity checks).

## Configuration

- Source of truth: `.env.example`
- Naming conventions: `docs/reference/ENVIRONMENT_VARIABLES.md`

Key env vars:

- `RHELMA_ATTEST_VERIFIER__STRICT` (default: false) — fail closed when envelopes are malformed
- `RHELMA_ATTEST_VERIFIER__TPM_PCR_ALLOWLIST_JSON` (optional) — JSON map `{ "7": "<sha256hex>", ... }`
- `RHELMA_ATTEST_VERIFIER__SGX_MRENCLAVE_ALLOWLIST` (optional) — JSON array or comma-separated list

## Endpoints

This is a CLI tool (no HTTP endpoints).

## Observability

- Output is JSON on success, and a non-zero exit code on failure.
- Do not rely on logs for evidence content.

## Security / policy notes

- Treat evidence as **sensitive** (may contain device identifiers / platform measurements).
- Avoid storing evidence in CI logs. Prefer piping directly.
- If allowlists are set, `STRICT=true` is recommended.

## Verification

- Repo-wide: `./scripts/verify.sh` or `./scripts/verify.ps1`
- Local: `cargo test -p rhelma-attestation-verifier`

## Troubleshooting

- If you see `verification failed`, re-check:
  - `kind` value
  - allowlists (TPM PCR values / SGX MRENCLAVE)
  - evidence encoding (hex/base64 fields)
