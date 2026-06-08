#!/usr/bin/env bash
set -euo pipefail

# Minimal launcher for the Multi-Frontend service.
# - Serves Rust/Dioxus admin at /admin
# - Serves Svelte build from apps/web/build if present
# - Otherwise redirects / to RHELMA_WEB_DEV_URL (defaults to http://localhost:3000)

if [[ -f ".env" ]]; then
  set -a
  # shellcheck disable=SC1091
  source ".env"
  set +a
fi

export RHELMA_ENV=${RHELMA_ENV:-development}
export RHELMA_ENVIRONMENT="${RHELMA_ENVIRONMENT:-$RHELMA_ENV}"
export RHELMA_REGION="${RHELMA_REGION:-local}"
export RHELMA_SERVICE_VERSION="${RHELMA_SERVICE_VERSION:-0.0.0-dev}"

export RHELMA_MULTI_FRONTEND_SERVICE_NAME="${RHELMA_MULTI_FRONTEND_SERVICE_NAME:-multi-frontend}"
export RHELMA_MULTI_FRONTEND_LISTEN_ADDR="${RHELMA_MULTI_FRONTEND_LISTEN_ADDR:-0.0.0.0:8080}"
export RHELMA_WEB_DIST_DIR="${RHELMA_WEB_DIST_DIR:-apps/web/build}"
export RHELMA_WEB_DEV_URL="${RHELMA_WEB_DEV_URL:-http://localhost:5173}"

# Optional reverse proxy targets / dashboard upstreams
export RHELMA_API_GATEWAY_URL="${RHELMA_API_GATEWAY_URL:-http://localhost:8081}"
export RHELMA_NODE_REGISTRY_URL="${RHELMA_NODE_REGISTRY_URL:-http://localhost:9100}"
export RHELMA_REALM_HUB_URL="${RHELMA_REALM_HUB_URL:-http://localhost:9110}"
export RHELMA_DEFAULT_REALM_ID="${RHELMA_DEFAULT_REALM_ID:-realm_first}"

echo "Starting multi-frontend on ${RHELMA_MULTI_FRONTEND_LISTEN_ADDR}..."
cargo run -p multi-frontend
