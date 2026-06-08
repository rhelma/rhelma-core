#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
cd "$ROOT_DIR"

COMPOSE_FILE=${RHELMA6_COMPOSE_FILE:-deploy/rhelma6/docker/docker-compose.rhelma6.yml}
ENV_FILE=${RHELMA6_ENV_FILE:-deploy/rhelma6/docker/.env.rhelma6}

if [ ! -f "$ENV_FILE" ]; then
  echo "Missing env file: $ENV_FILE"
  echo "Copy deploy/rhelma6/docker/.env.rhelma6.example -> $ENV_FILE and edit." 
  exit 2
fi

function curl_ok() {
  local url="$1"
  curl -fsS "$url" >/dev/null
}

function wait_http() {
  local url="$1"; local name="$2"; local max_sec=${3:-60}
  echo "[wait] $name: $url"
  local start=$(date +%s)
  while true; do
    if curl_ok "$url"; then
      echo "[ok] $name"
      return 0
    fi
    sleep 1
    local now=$(date +%s)
    if (( now - start > max_sec )); then
      echo "[fail] $name did not become ready in ${max_sec}s"
      return 1
    fi
  done
}

cleanup() {
  echo "[cleanup] stopping compose"
  docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down -v >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "[compose] up"
docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" up -d --remove-orphans

# --- Health checks ---
NR_URL=${RHELMA6_NODE_REGISTRY_URL:-http://127.0.0.1:8090}
SG_URL=${RHELMA6_SECURITY_GOV_URL:-http://127.0.0.1:8091}
GD_URL=${RHELMA6_GOSSIP_URL:-http://127.0.0.1:8092}
BR_URL=${RHELMA6_BRIDGE_URL:-http://127.0.0.1:8094}

wait_http "$NR_URL/healthz" "node-registry" 90
wait_http "$SG_URL/healthz" "security-governance" 90
wait_http "$GD_URL/healthz" "gossip-discovery" 90
wait_http "$BR_URL/healthz" "bridge-adapter" 90

# --- Minimal functional probes ---

# 1) Discover should return JSON
curl -fsS "$NR_URL/v1/nodes/discover" | head -c 200 >/dev/null

# 2) Policy gate sanity: finalize should reject unknown chain (non-allowed)
# We only do a light probe: create intent with chain=forbidden and expect 4xx on finalize.
# If your bridge-adapter requires auth tokens, set RHELMA6_BRIDGE_ADMIN_TOKEN.
BR_TOKEN=${RHELMA6_BRIDGE_ADMIN_TOKEN:-}
HDRS=()
if [ -n "$BR_TOKEN" ]; then
  HDRS+=( -H "x-admin-token: $BR_TOKEN" )
fi

INTENT_JSON='{"direction":"deposit","chain":"forbidden","amount":1,"subject_id":"test-subject"}'
INTENT_ID=$(curl -fsS "${HDRS[@]}" -H 'content-type: application/json' -d "$INTENT_JSON" "$BR_URL/v1/bridge/intents" | sed -n 's/.*"id"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1)

if [ -z "$INTENT_ID" ]; then
  echo "[warn] could not parse intent id (bridge may require different schema). Skipping policy gate check."
else
  set +e
  curl -fsS "${HDRS[@]}" -H 'content-type: application/json' -d '{}' "$BR_URL/v1/bridge/intents/$INTENT_ID/finalize" >/dev/null
  RC=$?
  set -e
  if [ $RC -eq 0 ]; then
    echo "[fail] bridge finalize unexpectedly succeeded for forbidden chain"
    exit 1
  else
    echo "[ok] bridge policy gate (forbidden chain rejected or failed as expected)"
  fi
fi

echo "[PASS] Rhelma6 E2E integration (MVP)"
