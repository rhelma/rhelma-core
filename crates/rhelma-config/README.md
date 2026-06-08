# rhelma-config

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
| `DATABASE_URL` |  |
| `RHELMA_DB__MAX_CONNECTIONS` |  |
| `RHELMA_DB__MIN_CONNECTIONS` |  |
| `RHELMA_DB__READ_REPLICA_URL` |  |
| `RHELMA_DB__URL` |  |
| `RHELMA_OBSERVABILITY__METRICS_ENABLED` |  |
| `RHELMA_OBSERVABILITY__OTEL_ENABLED` |  |
| `RHELMA_OBSERVABILITY__OTEL_ENDPOINT` |  |
| `RHELMA_OBSERVABILITY__OTEL_REQUIRED` |  |
| `RHELMA_OBSERVABILITY__PROMETHEUS_PORT` |  |
| `RHELMA_OBS__METRICS_ENABLED` |  |
| `RHELMA_OBS__OTEL_ENABLED` |  |
| `RHELMA_OBS__OTEL_ENDPOINT` |  |
| `RHELMA_OBS__OTEL_REQUIRED` |  |
| `RHELMA_OBS__PROMETHEUS_PORT` |  |
| `RHELMA_REDIS__DEFAULT_TTL_SECS` |  |
| `RHELMA_REDIS__URL` |  |

## Security & Compliance

Normative requirements are in `docs/contract/v6.0/00_INDEX_v6.0.md`.
