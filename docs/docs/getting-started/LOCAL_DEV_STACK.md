# Local Development Stack (docker-compose)

This repository ships with a pragmatic dev stack: `docker-compose.dev.yml`.

It starts the core infrastructure required by the Rust services:

- Postgres (used by `DATABASE_URL` / `RHELMA_DB__URL`)
- Redis (used by `RHELMA_REDIS__URL`)
- Qdrant (vector search; used by `RHELMA_SEARCH_QDRANT_URL`)
- Meilisearch (text search; used by `RHELMA_SEARCH_MEILI_URL`)

Optional profiles:

- `kafka`: Redpanda (Kafka-compatible) — useful for `rhelma-event-kafka` and `ai-orchestrator`
- `s3`: MinIO (S3-compatible) — useful for `file-storage` in S3 mode
- `obs`: Jaeger (OTLP HTTP) — useful for tracing

## Start the stack

Core infrastructure only:

```bash
docker compose -f docker-compose.dev.yml up -d
```

Add Kafka:

```bash
docker compose -f docker-compose.dev.yml --profile kafka up -d
```

Add S3 + tracing:

```bash
docker compose -f docker-compose.dev.yml --profile s3 --profile obs up -d
```

## Connection URLs

The compose file maps container ports to your host, so the default `.env.example` URLs work as-is:

- Postgres: `postgres://rhelma_user:password@127.0.0.1:5432/rhelma_platform`
- Redis: `redis://127.0.0.1:6379/0`
- Qdrant: `http://127.0.0.1:6333`
- Meilisearch: `http://127.0.0.1:7700`

If you run the Rust services *inside* Docker later, switch the hostnames to the service names
(`postgres`, `redis`, `qdrant`, `meilisearch`).


## Troubleshooting

### Windows: `rdkafka-sys` / `librdkafka` cache problems

Common symptoms:

- CMakeCache directory mismatch errors (often after moving the repo between drives, using `SUBST`, or changing paths)
- Repeated `rdkafka-sys` build failures after toolchain upgrades

Fix:

1) Delete the build cache:

- Remove `target/` (or at least the `rdkafka-sys-*` / `cmake-*` directories under `target/`)

2) Clear the `rdkafka-sys` user cache (Windows):

- Delete `%LOCALAPPDATA%\rdkafka-sys\`

3) Rebuild / re-run verification:

```powershell
cargo check
.\scripts\verify.ps1
```

If you still hit native build errors, ensure you have a working C toolchain + CMake installed.
