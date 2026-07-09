# Rhelma Project Structure

Rhelma is organized as a domain-first monorepo. Runtime services live under `apps/`, reusable platform capabilities live under `crates/`, browser and SDK packages live under `packages/`, and durable API/event/RLS contracts live under `contracts/`.

## Canonical layout

```text
apps/
  api-gateway/              # public API entrypoint, routing, auth, rate limits, audit
  admin-web/                # SvelteKit admin/workspace UI
  web/                      # public web/PWA frontend
  social-service/           # social graph, posts, comments, feed, profiles
  file-storage-service/     # tenant-scoped file/media storage
  search-service/           # search API and indexing
  realtime-service/         # websocket/realtime collaboration
  multi-frontend/           # Rust frontend gateway/static bridge
crates/
  rhelma-core/              # shared primitives
  rhelma-db/                # database helpers and core migrations
  rhelma-auth/              # auth/session/JWT capabilities
  rhelma-config/            # central environment loading
  rhelma-health/            # health/readiness/liveness primitives
  rhelma-http-observability/# HTTP tracing/metrics helpers
packages/
  ui/                       # shared UI package
  sdk-js/                   # JavaScript client SDK
  sdk-python/               # Python client SDK
  sdk-go/                   # Go client SDK
contracts/
  openapi/                  # gateway-facing HTTP contracts
  events/                   # event schemas
  schemas/                  # shared JSON schemas
  rls/                      # tenant-isolation contract docs
migrations/                 # domain migration ownership documentation
```

## Service boundary rules

1. Browser clients should call `apps/api-gateway` as the public entrypoint.
2. Internal services may expose local routes, but the gateway owns public auth, audit, rate limits, CORS, and tenant-header normalization.
3. Tenant-scoped database access must happen inside a transaction that sets `app.tenant_id` before querying RLS-protected tables.
4. Contracts in `contracts/` are the source of truth for public API/event payload drift checks.
5. Service-local `sqlx::migrate!()` paths remain in place until each service is migrated to the central migrator wrapper.

## File storage rename

The legacy file storage app path has been renamed to `apps/file-storage-service` to match the Rust package name `file-storage-service`. Scripts, documentation, and Cargo workspace membership must use the new path.

## Frontend workspace

Root Node workspace files are now present:

- `package.json`
- `pnpm-workspace.yaml`

Use:

```bash
pnpm install
pnpm check
pnpm build
pnpm dev:admin
pnpm dev:web
```

## Verification

Use these checks before shipping structural changes:

```bash
bash scripts/check-structure.sh
bash scripts/guards/env_example_sync_guard.sh .
bash scripts/migrate.sh info
cargo check --workspace
pnpm check
```


## Gateway-first API layout

The api-gateway is the canonical public entrypoint for new clients. The current v1 contract stub lives at `contracts/openapi/gateway.v1.yaml`. Current compatibility routes are:

```text
/api/v1/social/*  -> social-service
/api/v1/search    -> search-service
/api/v1/media/*   -> file-storage-service
```

Unversioned gateway routes (`/social`, `/search`, `/media`) remain as temporary compatibility aliases. Direct service URLs are allowed only for local debugging and tests.
