# Environment Variables

Rhelma uses **environment variables as the primary configuration surface**.

The authoritative list is `.env.example` at the repo root.
This document is intentionally **short** and focuses on:

- **Naming conventions**
- **Central identity** (required everywhere)
- The **small set of knobs** people change most often

If something in this file disagrees with `.env.example` or the service code, treat **the code + `.env.example`** as the source of truth.

---

## Naming conventions

- **Central identity keys** are flat `RHELMA_*` (shared across all services).
- **Service-specific keys** either:
  - use a prefix (`RHELMA_SEARCH_*`, `RHELMA_RT_*`, `RHELMA_GATEWAY_*`), or
  - use nested keys (`RHELMA_FILE_STORAGE__...`, `RHELMA_AI_ORCH__...`) via CentralEnv style.

---

## Central identity (required)

Most services load a strict `CentralEnv` and will fail fast if these are missing:

- `RHELMA_ENV` (or `RHELMA_ENVIRONMENT`) — e.g. `development`, `staging`, `production`
- `RHELMA_REGION` — e.g. `local`, `eu-west-1`
- `RHELMA_SERVICE_VERSION` — semantic version string for telemetry (e.g. `5.2.0`)

Common optional identity keys:

- `RHELMA_SERVICE_NAME` — defaults vary by service

---

## Common service knobs

### Shared HTTP security/observability (rhelma-http-observability)

- Audit log PII scrubbing:
  - `RHELMA_AUDIT__HASH_IP` (default `true`) — when enabled, audit layers log a hashed
    client IP hint (`h:...`) instead of the raw IP.

### api-gateway (apps/api-gateway)

- Bind:
  - `RHELMA_BIND_HOST` (default `0.0.0.0`)
  - `RHELMA_BIND_PORT` (default `8080`)

- Timeouts:
  - `RHELMA_GATEWAY_TIMEOUT_GLOBAL` (default `10s`)
  - `RHELMA_GATEWAY_TIMEOUT_UPSTREAM` (default `5s`)

- CORS:
  - `RHELMA_GATEWAY_CORS_ALLOWED_ORIGINS` (CSV; default `*` in non-prod)
  - `RHELMA_GATEWAY_CORS_ALLOW_CREDENTIALS` (default `false`)

- Redis (rate limiting, sessions):
  - `RHELMA_REDIS__URL` (preferred)
  - legacy: `RHELMA_REDIS_URL`

- Upstream endpoints (defaults are local placeholders):
  - `RHELMA_AUTH_SERVICE_URL`
  - `RHELMA_SEARCH_SERVICE_URL`
  - `RHELMA_SOCIAL_SERVICE_URL`
  - `RHELMA_FILE_STORAGE_SERVICE_URL` (gateway upstream for file-storage-service; falls back to `RHELMA_FILE_STORAGE_URL`)
  - `RHELMA_CONTROL_SERVICE_URL` (optional; enables realm discovery caching)
  - `RHELMA_GATEWAY_DISCOVERY_CACHE_TTL_SECONDS` (default `30`)
  - `RHELMA_USER_SERVICE_URL`
  - `RHELMA_AI_SERVICE_URL`

### search-service (apps/search-service)

- `RHELMA_SEARCH_LISTEN_ADDR` (default `0.0.0.0:8080`)
- `RHELMA_SEARCH_QDRANT_URL` (required)
- `RHELMA_SEARCH_MEILI_URL` (required)
- `RHELMA_SEARCH_DEFAULT_INDEX` (default `documents`)
- `RHELMA_SEARCH_REDIS_URL` (optional)

### realtime-service (apps/realtime-service)

- `RHELMA_RT_LISTEN_ADDR` (default `0.0.0.0:9000`)

WebSocket/rate-limit knobs (service-level safety net):

- `REALTIME_ALLOW_ANONYMOUS` (default: `true` only in development)
- `REALTIME_WS_MAX_MESSAGE_BYTES`
- `REALTIME_WS_PING_INTERVAL_SECS`
- `REALTIME_WS_PONG_TIMEOUT_SECS`
- `REALTIME_WS_MSGS_PER_SEC`
- `REALTIME_WS_MSG_BURST`
- `REALTIME_MAX_CONNECTIONS_PER_USER`
- `REALTIME_MAX_ROOMS_PER_CONN`

### file-storage-service (apps/file-storage-service)

Most keys are nested:

- `RHELMA_FILE_STORAGE__LISTEN_ADDR` (default `0.0.0.0:3005`)
- `RHELMA_FILE_STORAGE__DATABASE_URL` (or `RHELMA_DATABASE_URL`) (required)
- `RHELMA_FILE_STORAGE__PROVIDER` (`local`/`s3`)
- `RHELMA_FILE_STORAGE__LOCAL_ROOT` (default `./data`)

S3 (required when provider is `s3`):

- `RHELMA_FILE_STORAGE__S3_ENDPOINT`
- `RHELMA_FILE_STORAGE__S3_BUCKET`
- `RHELMA_FILE_STORAGE__S3_ACCESS_KEY`
- `RHELMA_FILE_STORAGE__S3_SECRET_KEY`

### ai-orchestrator (apps/ai-orchestrator)

Most keys are nested (`RHELMA_AI_ORCH__...`). Start from `.env.example`.

Common ones:

- `RHELMA_AI_ORCH__LISTEN_ADDR` (default `0.0.0.0:4000`)
- `RHELMA_AI_ORCH__SEARCH_SERVICE_URL` (required in most runs)
- `RHELMA_AI_ORCH__KAFKA_BROKERS` (use `noop` for local dev)

### observability-agent (observability/agent)

Topic allow-lists (no wildcards):

- `OBS_AGENT_COMMAND_TOPICS`
- `OBS_AGENT_SIGNAL_TOPICS`
- `OBS_AGENT_DECISION_TOPICS`

Optional:

- `OBS_AGENT_KAFKA_BROKERS`
- `OBS_AGENT_NATS_URL`

### control-service (apps/control-service)

- Listen:
  - `RHELMA_CONTROL_LISTEN_ADDR` (default `0.0.0.0:8086`)

- Admin / node security:
  - `RHELMA_CONTROL_ADMIN_TOKEN` — required for admin routes (`x-control-admin-token`)
  - `RHELMA_CONTROL_NODE_REGISTRATION_TOKEN` — required for node registration (`x-control-node-registration-token`)

- Node liveness:
  - `RHELMA_CONTROL_NODE_ONLINE_TTL_SECONDS` (default `90`) — how long after the last heartbeat a node is considered online

- Backup execution (admin-API `POST /v1/admin/backup` → `scripts/backup.sh`):
  - `RHELMA_CONTROL_BACKUP_DIR` (default `/var/backups/rhelma`) — output directory for archives
  - `RHELMA_CONTROL_BACKUP_TIMEOUT_SECS` (default `300`) — bound on the backup subprocess
  - `RHELMA_BACKUP_PASSPHRASE` — read from the service env (never the HTTP body); without it (or a
    GPG recipient) `backup.sh` refuses to write an unencrypted archive. See
    [`runbooks/backup_restore.md`](../runbooks/backup_restore.md).

### multi-frontend (apps/multi-frontend)

The public edge + `/admin` console. See
[`runbooks/admin_console_operations.md`](../runbooks/admin_console_operations.md) and
[`openapi/multi-frontend.yaml`](../openapi/multi-frontend.yaml).

- Admin auth / RBAC (three token-mapped roles, `viewer < operator < super_admin`):
  - `RHELMA_ADMIN_TOKEN` — **super_admin** bearer token
  - `RHELMA_ADMIN_TOKEN_OPERATOR` — operator token
  - `RHELMA_ADMIN_TOKEN_VIEWER` — viewer (read-only) token
  - `RHELMA_ADMIN_MTLS_REQUIRED` (default `false`) — when `true`, also require an allowlisted
    client-cert fingerprint (`x-rhelma-client-cert-sha256`)
  - `RHELMA_ADMIN_AUTH_URL` — auth-gateway verify URL backing the session-cookie login flow

- Management features (Step 3):
  - `RHELMA_MAINTENANCE_FLAG_PATH` (default `data/maintenance.flag`) — maintenance-mode flag file;
    presence ⇒ ON, so the state survives a restart
  - `RHELMA_LOKI_URL` (default `http://localhost:3100`) — Loki base URL for the admin log viewer
    (`GET /admin/api/logs`)

- Upstream proxy knobs:
  - `RHELMA_UPSTREAM_TIMEOUT_MS` (default `4000`)
  - `RHELMA_UPSTREAM_RETRIES`, `RHELMA_UPSTREAM_RETRY_BACKOFF_MS`
  - `RHELMA_ADMIN_DASHBOARD_CACHE_TTL_MS` (default `3000`) — dashboard/readiness fan-out cache TTL

### social-service (apps/social-service)

- Listen:
  - `RHELMA_SOCIAL_LISTEN_ADDR` (default `0.0.0.0:8085`)

- Feed limits:
  - `RHELMA_SOCIAL_FEED_DEFAULT_LIMIT` (default `20`)
  - `RHELMA_SOCIAL_FEED_MAX_LIMIT` (default `100`)

- Dependencies:
  - `RHELMA_DATABASE_URL` / `RHELMA_DB__URL` (Postgres)
  - `RHELMA_REDIS__URL` (Redis; used by rhelma-auth and token revocation)
