# social-service

## Overview

`social-service` provides the **news + social graph** primitives for Rhelma realms:

- posts (post/article/link)
- comments
- reactions (like/bookmark)
- feed queries with cursor-based pagination

The service is tenant-aware via `RequestContext` (usually the `x-tenant-id` header).

## Run (local)

```bash
cargo run -p social-service
```

## Configuration

Source of truth: `.env.example`.

Key environment variables:

- `RHELMA_SOCIAL_LISTEN_ADDR` (default `0.0.0.0:8085`)
- `RHELMA_DATABASE_URL` / `RHELMA_DB__URL` (Postgres)
- `RHELMA_REDIS__URL` (used by rhelma-auth + token revocation)
- `RHELMA_SOCIAL_FEED_DEFAULT_LIMIT` (default `20`)
- `RHELMA_SOCIAL_FEED_MAX_LIMIT` (default `100`)

## Endpoints

### Health

- `GET /health`
- `GET /metrics`

### Social API

- `GET /feed/latest?limit=&cursor=`
- `POST /posts` (requires Bearer token)
- `GET /posts/:id`
- `GET /posts/:id/comments`
- `POST /posts/:id/comments` (requires Bearer token)
- `POST /posts/:id/reactions/:kind` (requires Bearer token)

## Security notes

Write endpoints require `Authorization: Bearer <access-token>` and rely on `rhelma-auth`.
Token revocation is checked via Redis keys:

- `revoke:token:<fingerprint>`
- `revoke:user:<user_id>`

## Verification

```bash
cargo test -p social-service
```

## Ownership
- **Owner:** TBD
- **Tier:** dev | staging | prod
- **Startup dependencies:** see `.env.example`
- **Data safety:** see service docs

## Observability
- Tracing: W3C Trace Context (`traceparent`).
- Metrics: `/metrics` when enabled.
- Logs: structured; include `request_id`.
