#!/usr/bin/env bash
# scripts/dev/run-social-mvp.sh
# Local "social-mvp" stack:
#   - docker: Postgres + Redis + Qdrant + Meilisearch (and MinIO if file-storage provider=s3)
#   - services: control-service, search-service, file-storage-service, realtime-service, social-service, api-gateway
#   - registers local social node into realm=central and keeps it alive via heartbeat

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

log() { echo -e "${BLUE}➤${NC} $1"; }
success() { echo -e "${GREEN}✓${NC} $1"; }
warn() { echo -e "${YELLOW}⚠${NC} $1"; }
error() { echo -e "${RED}✗${NC} $1"; exit 1; }

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

# Best-effort: load repo-level .env so strict env contract is satisfied.
if [[ -f ".env" ]]; then
  set -a
  # shellcheck disable=SC1091
  source ".env"
  set +a
fi

# CentralEnv strict fields (required by most Rust services)
export RHELMA_ENVIRONMENT="${RHELMA_ENVIRONMENT:-${RHELMA_ENV:-development}}"
export RHELMA_ENV="${RHELMA_ENV:-${RHELMA_ENVIRONMENT}}"
export RHELMA_REGION="${RHELMA_REGION:-local}"
export RHELMA_SERVICE_VERSION="${RHELMA_SERVICE_VERSION:-0.0.0-dev}"

# Core infra
export RHELMA_DB__URL="${RHELMA_DB__URL:-${DATABASE_URL:-postgres://rhelma_user:password@127.0.0.1:5432/rhelma_platform}}"
export DATABASE_URL="${DATABASE_URL:-$RHELMA_DB__URL}"
export RHELMA_REDIS__URL="${RHELMA_REDIS__URL:-redis://127.0.0.1:6379/0}"
export RHELMA_DB__AUTO_MIGRATE="${RHELMA_DB__AUTO_MIGRATE:-1}"

# Service URLs (used by scripts/tests and some clients)
export RHELMA_SEARCH_SERVICE_URL="${RHELMA_SEARCH_SERVICE_URL:-http://127.0.0.1:8082}"
export FILE_STORAGE_URL="${FILE_STORAGE_URL:-http://127.0.0.1:3005}"
export RHELMA_SOCIAL_SERVICE_URL="${RHELMA_SOCIAL_SERVICE_URL:-http://127.0.0.1:8085}"
export RHELMA_CONTROL_SERVICE_URL="${RHELMA_CONTROL_SERVICE_URL:-http://127.0.0.1:8086}"
export RHELMA_SMOKE_REALTIME_URL="${RHELMA_SMOKE_REALTIME_URL:-http://127.0.0.1:9000}"

# Search backends
export RHELMA_SEARCH_QDRANT_URL="${RHELMA_SEARCH_QDRANT_URL:-http://127.0.0.1:6333}"
export RHELMA_SEARCH_MEILI_URL="${RHELMA_SEARCH_MEILI_URL:-http://127.0.0.1:7700}"

# Listeners
export RHELMA_SEARCH_LISTEN_ADDR="${RHELMA_SEARCH_LISTEN_ADDR:-0.0.0.0:8082}"
export RHELMA_FILE_STORAGE__LISTEN_ADDR="${RHELMA_FILE_STORAGE__LISTEN_ADDR:-0.0.0.0:3005}"
export RHELMA_FILE_STORAGE__DATABASE_URL="${RHELMA_FILE_STORAGE__DATABASE_URL:-$DATABASE_URL}"
export RHELMA_FILE_STORAGE__PROVIDER="${RHELMA_FILE_STORAGE__PROVIDER:-local}"
export RHELMA_FILE_STORAGE__LOCAL_ROOT="${RHELMA_FILE_STORAGE__LOCAL_ROOT:-./data/files}"

export RHELMA_RT_LISTEN_ADDR="${RHELMA_RT_LISTEN_ADDR:-0.0.0.0:9000}"
export REALTIME_ALLOW_ANONYMOUS="${REALTIME_ALLOW_ANONYMOUS:-true}"

export RHELMA_CONTROL_LISTEN_ADDR="${RHELMA_CONTROL_LISTEN_ADDR:-0.0.0.0:8086}"
export RHELMA_CONTROL_ADMIN_TOKEN="${RHELMA_CONTROL_ADMIN_TOKEN:-dev-admin}"
export RHELMA_CONTROL_NODE_REGISTRATION_TOKEN="${RHELMA_CONTROL_NODE_REGISTRATION_TOKEN:-dev-node-token}"

export RHELMA_SOCIAL_LISTEN_ADDR="${RHELMA_SOCIAL_LISTEN_ADDR:-0.0.0.0:8085}"

export RHELMA_GATEWAY_DISCOVERY_CACHE_TTL_SECONDS="${RHELMA_GATEWAY_DISCOVERY_CACHE_TTL_SECONDS:-30}"
export RHELMA_BIND_HOST="${RHELMA_BIND_HOST:-0.0.0.0}"
export RHELMA_BIND_PORT="${RHELMA_BIND_PORT:-3000}"

cleanup() {
  warn "Shutting down background services..."
  for pid in "${GW_PID:-}" "${SS_PID:-}" "${CS_PID:-}" "${SEARCH_PID:-}" "${FS_PID:-}" "${RT_PID:-}" "${HB_PID:-}"; do
    [[ -n "${pid}" ]] && kill "${pid}" 2>/dev/null || true
  done
}
trap cleanup EXIT

log "Starting docker infra (Postgres + Redis + Qdrant + Meilisearch)..."
compose_args=(-f docker-compose.dev.yml)
if [[ "${RHELMA_FILE_STORAGE__PROVIDER}" == "s3" ]]; then
  compose_args+=(--profile s3)
  docker compose "${compose_args[@]}" up -d postgres redis qdrant meilisearch minio >/dev/null
else
  docker compose "${compose_args[@]}" up -d postgres redis qdrant meilisearch >/dev/null
fi
success "Docker services started"

log "Waiting for Postgres to become healthy..."
for i in {1..40}; do
  status="$(docker inspect -f '{{.State.Health.Status}}' rhelma-postgres 2>/dev/null || echo starting)"
  if [[ "$status" == "healthy" ]]; then
    success "Postgres is healthy"
    break
  fi
  sleep 1
  [[ "$i" -eq 40 ]] && error "Postgres did not become healthy in time"
done

log "Starting control-service on ${RHELMA_CONTROL_LISTEN_ADDR}..."
(RHELMA_SERVICE_NAME="control-service" cargo run -p control-service) >/tmp/control-service.log 2>&1 &
CS_PID=$!

log "Waiting for control-service health..."
for i in {1..60}; do
  if curl -fsS "${RHELMA_CONTROL_SERVICE_URL}/health" >/dev/null 2>&1; then
    success "control-service is up"
    break
  fi
  sleep 1
  [[ "$i" -eq 60 ]] && error "control-service didn't come up (see /tmp/control-service.log)"
done

log "Starting search-service on ${RHELMA_SEARCH_LISTEN_ADDR}..."
(RHELMA_SERVICE_NAME="search-service" cargo run -p search-service) >/tmp/search-service.log 2>&1 &
SEARCH_PID=$!

log "Waiting for search-service health..."
for i in {1..60}; do
  if curl -fsS "${RHELMA_SEARCH_SERVICE_URL}/healthz" >/dev/null 2>&1; then
    success "search-service is up"
    break
  fi
  sleep 1
  [[ "$i" -eq 60 ]] && warn "search-service health not detected yet (see /tmp/search-service.log)"
done

log "Starting file-storage-service on ${RHELMA_FILE_STORAGE__LISTEN_ADDR}..."
(RHELMA_SERVICE_NAME="file-storage-service" cargo run -p file-storage-service) >/tmp/file-storage.log 2>&1 &
FS_PID=$!

log "Waiting for file-storage-service health..."
for i in {1..60}; do
  if curl -fsS "${FILE_STORAGE_URL}/healthz" >/dev/null 2>&1; then
    success "file-storage-service is up"
    break
  fi
  sleep 1
  [[ "$i" -eq 60 ]] && warn "file-storage-service health not detected yet (see /tmp/file-storage.log)"
done

log "Starting realtime-service on ${RHELMA_RT_LISTEN_ADDR}..."
(RHELMA_SERVICE_NAME="realtime-service" cargo run -p realtime-service) >/tmp/realtime-service.log 2>&1 &
RT_PID=$!

log "Waiting for realtime-service health..."
for i in {1..60}; do
  if curl -fsS "${RHELMA_SMOKE_REALTIME_URL}/healthz" >/dev/null 2>&1; then
    success "realtime-service is up"
    break
  fi
  sleep 1
  [[ "$i" -eq 60 ]] && warn "realtime-service health not detected yet (see /tmp/realtime-service.log)"
done

log "Starting social-service on ${RHELMA_SOCIAL_LISTEN_ADDR}..."
(RHELMA_SERVICE_NAME="social-service" cargo run -p social-service) >/tmp/social-service.log 2>&1 &
SS_PID=$!

log "Waiting for social-service health..."
for i in {1..60}; do
  if curl -fsS "${RHELMA_SOCIAL_SERVICE_URL}/health" >/dev/null 2>&1; then
    success "social-service is up"
    break
  fi
  sleep 1
  [[ "$i" -eq 60 ]] && error "social-service didn't come up (see /tmp/social-service.log)"
done

log "Registering local social node into realm=central..."
register_out="$(bash scripts/dev/register-local-social-node.sh 2>/dev/null || true)"
NODE_ID="$(echo "$register_out" | sed -n 's/.*"node_id"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1)"
API_KEY="$(echo "$register_out" | sed -n 's/.*"api_key"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1)"

if [[ -z "${NODE_ID}" || -z "${API_KEY}" ]]; then
  warn "Could not parse node_id/api_key from registration response."
  warn "Response was: ${register_out}"
else
  success "Registered node_id=${NODE_ID}"
  log "Starting heartbeat loop..."
  (
    while true; do
      curl -fsS -X POST "${RHELMA_CONTROL_SERVICE_URL}/v1/nodes/${NODE_ID}/heartbeat" \
        -H "authorization: Bearer ${API_KEY}" \
        -H "content-type: application/json" \
        -d '{"checks":{"ok":true}}' >/dev/null 2>&1 || true
      sleep 20
    done
  ) &
  HB_PID=$!
fi

log "Starting api-gateway on ${RHELMA_BIND_HOST}:${RHELMA_BIND_PORT}..."
(RHELMA_SERVICE_NAME="api-gateway" cargo run -p api-gateway) >/tmp/api-gateway.log 2>&1 &
GW_PID=$!

log "Waiting for api-gateway health..."
for i in {1..60}; do
  if curl -fsS "http://127.0.0.1:${RHELMA_BIND_PORT}/health" >/dev/null 2>&1; then
    success "api-gateway is up"
    break
  fi
  sleep 1
  [[ "$i" -eq 60 ]] && warn "api-gateway health not detected yet (see /tmp/api-gateway.log)"
done

success "READY 🎉"
echo ""
echo "Gateway:         http://127.0.0.1:${RHELMA_BIND_PORT}/"
echo "Social health:   curl -H 'x-tenant-id: central' http://127.0.0.1:${RHELMA_BIND_PORT}/social/health"
echo "Search health:   curl -H 'x-tenant-id: central' http://127.0.0.1:${RHELMA_BIND_PORT}/search/healthz"
echo "Files health:    curl -H 'x-tenant-id: central' http://127.0.0.1:${RHELMA_BIND_PORT}/files/healthz"
echo "Realtime health: curl -H 'x-tenant-id: central' http://127.0.0.1:${RHELMA_BIND_PORT}/realtime/healthz"
echo ""
echo "Logs:"
echo "  tail -f /tmp/control-service.log /tmp/search-service.log /tmp/file-storage.log /tmp/realtime-service.log /tmp/social-service.log /tmp/api-gateway.log"
echo ""
log "Press Ctrl-C to stop (background services will be killed)."

wait "$GW_PID"
