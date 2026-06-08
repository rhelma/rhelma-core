#!/usr/bin/env bash
# scripts/run-first-realm.sh
# Bring up the "First Realm" stack locally:
#   - node-registry
#   - gossip-discovery
#   - realm-hub
#
# This is intentionally light-weight (no docker required) and follows the
# repo's CentralEnv strict requirements.

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

# Best-effort: load repo-level .env so the CentralEnv contract is satisfied.
if [[ -f ".env" ]]; then
  set -a
  # shellcheck disable=SC1091
  source ".env"
  set +a
fi

# CentralEnv strict fields (required by most Rust services)
export RHELMA_ENVIRONMENT="${RHELMA_ENVIRONMENT:-development}"
export RHELMA_REGION="${RHELMA_REGION:-local}"
export RHELMA_SERVICE_VERSION="${RHELMA_SERVICE_VERSION:-0.0.0-dev}"

# Ports (override as needed)
export RHELMA_NODE_REGISTRY_PORT="${RHELMA_NODE_REGISTRY_PORT:-9010}"
export RHELMA_GOSSIP_DISCOVERY_PORT="${RHELMA_GOSSIP_DISCOVERY_PORT:-9020}"
export RHELMA_REALM_HUB_PORT="${RHELMA_REALM_HUB_PORT:-9110}"
export RHELMA_AI_COMPANION_PORT="${RHELMA_AI_COMPANION_PORT:-9120}"

# node-registry uses RHELMA_BIND_HOST / RHELMA_BIND_PORT
export RHELMA_BIND_HOST="${RHELMA_BIND_HOST:-0.0.0.0}"

# realm-hub uses RHELMA_REALM_HUB_LISTEN_ADDR
export RHELMA_REALM_HUB_LISTEN_ADDR="${RHELMA_REALM_HUB_LISTEN_ADDR:-0.0.0.0:${RHELMA_REALM_HUB_PORT}}"
export RHELMA_REALM_HUB_DEFAULT_REALM_ID="${RHELMA_REALM_HUB_DEFAULT_REALM_ID:-realm_first}"
export RHELMA_REALM_HUB_MANIFEST_PATH="${RHELMA_REALM_HUB_MANIFEST_PATH:-docs/realms/first_realm_manifest.json}"

# ai-companion uses RHELMA_AI_COMPANION_LISTEN_ADDR and talks to realm-hub
export RHELMA_AI_COMPANION_LISTEN_ADDR="${RHELMA_AI_COMPANION_LISTEN_ADDR:-0.0.0.0:${RHELMA_AI_COMPANION_PORT}}"
export RHELMA_REALM_HUB_URL="${RHELMA_REALM_HUB_URL:-http://localhost:${RHELMA_REALM_HUB_PORT}}"
export RHELMA_DEFAULT_REALM_ID="${RHELMA_DEFAULT_REALM_ID:-${RHELMA_REALM_HUB_DEFAULT_REALM_ID}}"
export RHELMA_AI_COMPANION_ACTOR_DID="${RHELMA_AI_COMPANION_ACTOR_DID:-did:rhelma:ai-companion}"

# Start processes
PIDS=()
cleanup() {
  log "Shutting down..."
  for pid in "${PIDS[@]:-}"; do
    kill "$pid" >/dev/null 2>&1 || true
  done
}
trap cleanup EXIT

log "Starting node-registry on :${RHELMA_NODE_REGISTRY_PORT}"  
(
  export RHELMA_SERVICE_NAME="node-registry"
  export RHELMA_BIND_PORT="${RHELMA_NODE_REGISTRY_PORT}"
  cargo run -p node-registry
) &
PIDS+=("$!")

log "Starting gossip-discovery on :${RHELMA_GOSSIP_DISCOVERY_PORT}"
(
  export RHELMA_SERVICE_NAME="gossip-discovery"
  export RHELMA_BIND_PORT="${RHELMA_GOSSIP_DISCOVERY_PORT}"
  cargo run -p gossip-discovery
) &
PIDS+=("$!")

log "Starting realm-hub on ${RHELMA_REALM_HUB_LISTEN_ADDR}"
(
  export RHELMA_SERVICE_NAME="realm-hub"
  cargo run -p realm-hub
) &
PIDS+=("$!")

log "Starting ai-companion on ${RHELMA_AI_COMPANION_LISTEN_ADDR}"
(
  export RHELMA_SERVICE_NAME="ai-companion"
  export RHELMA_AI_COMPANION_SERVICE_NAME="ai-companion"
  export RHELMA_AI_COMPANION_LISTEN_ADDR
  export RHELMA_REALM_HUB_URL
  export RHELMA_DEFAULT_REALM_ID
  export RHELMA_AI_COMPANION_ACTOR_DID
  cargo run -p ai-companion
) &
PIDS+=("$!")

success "First Realm stack is running"

echo -e "${GREEN}
Endpoints:
  node-registry:     http://localhost:${RHELMA_NODE_REGISTRY_PORT}/healthz
  gossip-discovery:  http://localhost:${RHELMA_GOSSIP_DISCOVERY_PORT}/healthz
  realm-hub:         http://localhost:${RHELMA_REALM_HUB_PORT}/healthz

Realm API:
  manifest:  http://localhost:${RHELMA_REALM_HUB_PORT}/v1/realms/${RHELMA_REALM_HUB_DEFAULT_REALM_ID}/manifest
  channels:  http://localhost:${RHELMA_REALM_HUB_PORT}/v1/realms/${RHELMA_REALM_HUB_DEFAULT_REALM_ID}/channels
${NC}"

log "Press Ctrl+C to stop."
wait
