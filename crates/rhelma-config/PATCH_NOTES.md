# Patch Notes

## 5.2.1 (2025-12-18)
- Added optional `dashmap-cache` feature to use `DashMap` for higher-concurrency caching.
- `load_with_prefix` / `load_with_prefix_strict` now default **only when the prefixed config is truly absent** (empty builder); deserialization errors no longer silently fall back to defaults.
- Added loader regression tests to ensure non-silent failure on invalid values.

## 5.2.0 (2025-12-18)
- Added `CentralRuntime` helper to read central env + `RHELMA_SERVICE_NAME` (strict requires it in production).
- Added strict-required loaders to avoid silently defaulting on deserialization errors.
- Added `CentralEnv::from_env_model_v1_strict()` gated by `RHELMA_ENV_MODEL_v1`.
- Expanded prelude exports for strict loader functions.
- Added `set_deprecation_handler(...)` to route deprecation warnings (default: stderr).
- Added TTL support to `CachedProvider::new_with_ttl(...)` plus cache invalidation helpers.
- Added optional `dashmap-cache` feature to use `DashMap` for higher-concurrency caching.
- `load_with_prefix*` now defaults **only when the prefixed config is truly absent** ("configuration is empty"); deserialization errors are no longer silently defaulted.
- Stabilized env-mutating tests using a global env lock (`tests/common`).

# rhelma-config alignment bundle (Rhelma v5.1)

This zip contains an updated `rhelma-config` crate with:

- Strict, contract-aligned `CentralEnv::from_env_strict()` (fail-closed in production).
- Strong-ID alignment in strict validation (RegionId/TenantId via rhelma-core).
- Deprecation warnings for `RHELMA_OBSERVABILITY__*` (canonical is `RHELMA_OBS__*`):
  - `src/models.rs` (direct env overrides)
  - `src/core_config.rs` (obs-related env)
  - `src/sources/env.rs` (env override source)

No public APIs were removed. Existing `CentralEnv::from_env()` remains.

## Follow-ups outside this crate (recommended)
- Downstream services should switch to `CentralEnv::from_env_strict()` for runtime identity.
- Prefer `RHELMA_OBS__*` env vars; remove legacy `RHELMA_OBSERVABILITY__*` exports.








