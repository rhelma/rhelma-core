# rhelma-sandbox-runner

Crate for the Rhelma platform.

## Contract

This component MUST comply with **Rhelma Contract v6.0**. See `docs/contract/v6.0/00_INDEX_v6.0.md`.

## Usage

Add as a dependency and follow the public API.

```toml
# In Cargo.toml
# [dependencies]
```

## Configuration

This crate reads the following environment variables (directly or via configs):

| Variable | Purpose |
|---|---|
| `RHELMA_SANDBOX_RUNNER__ALLOWED_COMMAND_PREFIXES` |  |
| `RHELMA_SANDBOX_RUNNER__ALLOWED_PATH_PREFIXES` |  |
| `RHELMA_SANDBOX_RUNNER__APPLY_BRANCH_PREFIX` |  |
| `RHELMA_SANDBOX_RUNNER__APPLY_FETCH_REMOTE` |  |
| `RHELMA_SANDBOX_RUNNER__APPLY_GIT_REMOTE` |  |
| `RHELMA_SANDBOX_RUNNER__APPLY_PUSH_ENABLED` |  |
| `RHELMA_SANDBOX_RUNNER__ATTESTATION_REQUIRED` |  |
| `RHELMA_SANDBOX_RUNNER__COMMAND_TIMEOUT_MS` |  |
| `RHELMA_SANDBOX_RUNNER__DOCKER_ENABLED` |  |
| `RHELMA_SANDBOX_RUNNER__DOCKER_IMAGE` |  |
| `RHELMA_SANDBOX_RUNNER__FORBIDDEN_PATH_PREFIXES` |  |
| `RHELMA_SANDBOX_RUNNER__MAX_PATCH_BYTES` |  |
| `RHELMA_SANDBOX_RUNNER__ROLLBACK_BASE_BRANCH` |  |
| `RHELMA_SANDBOX_RUNNER__ROLLBACK_BRANCH_PREFIX` |  |
| `RHELMA_SANDBOX_RUNNER__ROLLBACK_FETCH_REMOTE` |  |
| `RHELMA_SANDBOX_RUNNER__ROLLBACK_GIT_REMOTE` |  |
| `RHELMA_SANDBOX_RUNNER__ROLLBACK_PUSH_ENABLED` |  |
| `RHELMA_SANDBOX_RUNNER__WORKSPACE_ROOT` |  |

## Security & Compliance

Normative requirements are in `docs/contract/v6.0/00_INDEX_v6.0.md`.
