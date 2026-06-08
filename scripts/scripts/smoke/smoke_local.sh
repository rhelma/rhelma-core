#!/usr/bin/env bash
# Local smoke checks for a developer-run Rhelma stack.
#
# This is intentionally a thin wrapper around the stable endpoint checks in
# scripts/smoke_staging.sh, but uses "local" naming to avoid confusion.

set -euo pipefail

# Provide local defaults consistent with docker-compose.dev.yml + common cargo run ports.
export RHELMA_SMOKE_TIMEOUT_SEC="${RHELMA_SMOKE_TIMEOUT_SEC:-${RHELMA_E2E_WAIT_TIMEOUT_SEC:-2}}"
export RHELMA_SMOKE_API_GATEWAY_URL="${RHELMA_SMOKE_API_GATEWAY_URL:-${RHELMA_E2E_API_GATEWAY_URL:-http://127.0.0.1:3000}}"
export RHELMA_SMOKE_AI_ORCH_URL="${RHELMA_SMOKE_AI_ORCH_URL:-${RHELMA_E2E_AI_ORCH_URL:-http://127.0.0.1:4000}}"
export RHELMA_SMOKE_SEARCH_URL="${RHELMA_SMOKE_SEARCH_URL:-${RHELMA_E2E_SEARCH_URL:-http://127.0.0.1:8082}}"
export RHELMA_SMOKE_FILE_STORAGE_URL="${RHELMA_SMOKE_FILE_STORAGE_URL:-${RHELMA_E2E_FILE_STORAGE_URL:-http://127.0.0.1:3005}}"
export RHELMA_SMOKE_REALTIME_URL="${RHELMA_SMOKE_REALTIME_URL:-${RHELMA_E2E_REALTIME_URL:-http://127.0.0.1:9000}}"
export RHELMA_SMOKE_NODE_REGISTRY_URL="${RHELMA_SMOKE_NODE_REGISTRY_URL:-${RHELMA_E2E_NODE_REGISTRY_URL:-http://127.0.0.1:8090}}"
export RHELMA_SMOKE_LLM_NODE_URL="${RHELMA_SMOKE_LLM_NODE_URL:-${RHELMA_E2E_LLM_NODE_URL:-http://127.0.0.1:8088}}"

bash scripts/smoke_staging.sh
