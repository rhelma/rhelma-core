#!/usr/bin/env bash
# scripts/dev/run-control-plane-social.sh
# Bring up: Postgres+Redis (docker) + control-service + social-service + api-gateway
# Then registers a local social node into the central realm and keeps it online via heartbeat.

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

# Best-effort: load repo-level .env so CentralEnv strict contract is satisfied.
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

# DB + Redis (CoreConfig)
export RHELMA_DB__URL="${RHELMA_DB__URL:-${DATABASE_URL:-postgres://rhelma_user:password@127.0.0.1:5432/rhelma_platform}}"
export DATABASE_URL="${DATABASE_URL:-$RHELMA_DB__URL}"
export RHELMA_REDIS__URL="${RHELMA_REDIS__URL:-redis://127.0.0.1:6379/0}"

# Auto-migrate in dev (safe for local)
export RHELMA_DB__AUTO_MIGRATE="${RHELMA_DB__AUTO_MIGRATE:-1}"

# Service endpoints
export RHELMA_SOCIAL_SERVICE_URL="${RHELMA_SOCIAL_SERVICE_URL:-http://127.0.0.1:8085}"
export RHELMA_CONTROL_SERVICE_URL="${RHELMA_CONTROL_SERVICE_URL:-http://127.0.0.1:8086}"
export RHELMA_GATEWAY_DISCOVERY_CACHE_TTL_SECONDS="${RHELMA_GATEWAY_DISCOVERY_CACHE_TTL_SECONDS:-30}"

# control-service tokens (dev)
export RHELMA_CONTROL_LISTEN_ADDR="${RHELMA_CONTROL_LISTEN_ADDR:-0.0.0.0:8086}"
export RHELMA_CONTROL_ADMIN_TOKEN="${RHELMA_CONTROL_ADMIN_TOKEN:-dev-admin}"
export RHELMA_CONTROL_NODE_REGISTRATION_TOKEN="${RHELMA_CONTROL_NODE_REGISTRATION_TOKEN:-dev-node-token}"

# social-service bind (dev)
export RHELMA_SOCIAL_LISTEN_ADDR="${RHELMA_SOCIAL_LISTEN_ADDR:-0.0.0.0:8085}"

# api-gateway bind (dev)
export RHELMA_SERVICE_NAME="api-gateway"
export RHELMA_BIND_HOST="${RHELMA_BIND_HOST:-0.0.0.0}"
export RHELMA_BIND_PORT="${RHELMA_BIND_PORT:-3000}"

cleanup() {
  warn "Shutting down background services..."
  [[ -n "${GW_PID:-}" ]] && kill "$GW_PID" 2>/dev/null || true
  [[ -n "${SS_PID:-}" ]] && kill "$SS_PID" 2>/dev/null || true
  [[ -n "${CS_PID:-}" ]] && kill "$CS_PID" 2>/dev/null || true
  [[ -n "${HB_PID:-}" ]] && kill "$HB_PID" 2>/dev/null || true
}
trap cleanup EXIT

log "Starting Postgres + Redis (docker-compose.dev.yml)..."
docker compose -f docker-compose.dev.yml up -d postgres redis >/dev/null
success "Docker services started"

log "Waiting for Postgres to become healthy..."
for i in {1..40}; do
  status="$(docker inspect -f '{{.State.Health.Status}}' rhelma-postgres 2>/dev/null || echo starting)"
  if [[ "$status" == "healthy" ]]; then
    success "Postgres is healthy"
    break
  fi
  sleep 1
  if [[ "$i" -eq 40 ]]; then
    error "Postgres did not become healthy in time"
  fi
done

log "Starting control-service on ${RHELMA_CONTROL_LISTEN_ADDR}..."
cargo run -p control-service >/tmp/control-service.log 2>&1 &
CS_PID=$!
sleep 1

log "Waiting for control-service health..."
for i in {1..60}; do
  if curl -fsS "${RHELMA_CONTROL_SERVICE_URL}/health" >/dev/null 2>&1; then
    success "control-service is up"
    break
  fi
  sleep 1
  if [[ "$i" -eq 60 ]]; then
    error "control-service didn't come up (see /tmp/control-service.log)"
  fi
done

log "Starting social-service on ${RHELMA_SOCIAL_LISTEN_ADDR}..."
cargo run -p social-service >/tmp/social-service.log 2>&1 &
SS_PID=$!
sleep 1

log "Waiting for social-service health..."
for i in {1..60}; do
  if curl -fsS "${RHELMA_SOCIAL_SERVICE_URL}/health" >/dev/null 2>&1; then
    success "social-service is up"
    break
  fi
  sleep 1
  if [[ "$i" -eq 60 ]]; then
    error "social-service didn't come up (see /tmp/social-service.log)"
  fi
done

log "Registering local social node into realm 'central'..."
REGISTER_JSON="$(curl -fsS -X POST "${RHELMA_CONTROL_SERVICE_URL}/v1/nodes/register" \
  -H "content-type: application/json" \
  -H "x-control-node-registration-token: ${RHELMA_CONTROL_NODE_REGISTRATION_TOKEN}" \
  -d "{
    \"name\": \"local-social\",
    \"region\": \"${RHELMA_REGION}\",
    \"public_base_url\": \"${RHELMA_SOCIAL_SERVICE_URL}\",
    \"realm_slug\": \"central\",
    \"capabilities\": { \"social-service\": true },
    \"version\": \"${RHELMA_SERVICE_VERSION}\"
  }")"

NODE_ID="$(python3 - <<'PY'
import json,sys
obj=json.loads(sys.stdin.read())
print(obj.get("node_id",""))
PY
<<<"$REGISTER_JSON")"

API_KEY="$(python3 - <<'PY'
import json,sys
obj=json.loads(sys.stdin.read())
print(obj.get("api_key",""))
PY
<<<"$REGISTER_JSON")"

if [[ -z "$NODE_ID" || -z "$API_KEY" ]]; then
  error "Failed to register node. Response: $REGISTER_JSON"
fi
success "Registered node_id=$NODE_ID (api_key_hint=$(python3 - <<'PY'
import json,sys
obj=json.loads(sys.stdin.read())
print(obj.get("api_key_hint",""))
PY
<<<"$REGISTER_JSON"))"

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

log "Starting api-gateway on ${RHELMA_BIND_HOST}:${RHELMA_BIND_PORT}..."
cargo run -p api-gateway >/tmp/api-gateway.log 2>&1 &
GW_PID=$!

log "Waiting for api-gateway health..."
for i in {1..60}; do
  if curl -fsS "http://127.0.0.1:${RHELMA_BIND_PORT}/health" >/dev/null 2>&1; then
    success "api-gateway is up"
    break
  fi
  sleep 1
  if [[ "$i" -eq 60 ]]; then
    warn "api-gateway health not detected yet (see /tmp/api-gateway.log)"
    break
  fi
done

success "READY 🎉"
echo ""
echo "Try:"
echo "  curl -H 'x-tenant-id: central' http://127.0.0.1:${RHELMA_BIND_PORT}/social/health"
echo "  curl -H 'x-tenant-id: central' http://127.0.0.1:${RHELMA_BIND_PORT}/social/feed/latest"
echo ""
echo "Logs:"
echo "  tail -f /tmp/control-service.log /tmp/social-service.log /tmp/api-gateway.log"
echo ""
log "Press Ctrl-C to stop (background services will be killed)."

# Keep script running in foreground
wait "$GW_PID"
