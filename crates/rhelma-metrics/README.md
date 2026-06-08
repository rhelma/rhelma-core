# rhelma-metrics

Deterministic metrics helpers for Rhelma observability.

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
| `RHELMA_METRICS_ENABLED` |  |
| `RHELMA_METRICS__HTTP_ENDPOINT_LIMIT` |  |
| `RHELMA_OBSERVABILITY__METRICS_ENABLED` |  |
| `RHELMA_OBSERVABILITY__OTEL_ENDPOINT` |  |
| `RHELMA_OBSERVABILITY__PROMETHEUS_PORT` |  |
| `RHELMA_OBS__METRICS_ENABLED` |  |
| `RHELMA_OBS__OTEL_ENDPOINT` |  |
| `RHELMA_OBS__PROMETHEUS_PORT` |  |
| `RHELMA_OTEL_ENDPOINT` |  |
| `RHELMA_PROMETHEUS_PORT` |  |

## Security & Compliance

Normative requirements are in `docs/contract/v6.0/00_INDEX_v6.0.md`.
