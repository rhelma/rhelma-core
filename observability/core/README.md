# rhelma-observability-core (v5.2)

This crate wires Rhelma observability primitives in a **service-friendly** way:

- **Logger** (`rhelma-logger`) — **fatal** if it fails (service should not start).
- **Tracing** (`rhelma-tracing`) — best-effort; failure results in *Degraded* health.
- **Metrics** (`rhelma-metrics`) — best-effort; global singleton semantics.

It exposes a small in-process handle (`ObservabilityCore`) for:

- reading a **health snapshot** (logger/tracing/metrics)
- accessing optional tracing/metrics handles (if enabled)

## Design notes

- `rhelma-metrics` is a global singleton. `reload_metrics()` can enable metrics if not yet
  initialized and can locally drop the handle to “disable”, but it cannot safely uninstall
  the global instance.

## Usage

```rust
let central = rhelma_config::CentralEnv::from_env();
let core = rhelma_observability_core::ObservabilityCore::init_from_central(&central, "api-gateway").await?;
let health = core.health();
```






## Notes

- Health metadata uses a normalized lowercase environment string (e.g., `development`, `staging`, `production`) to keep labels stable across services.
