#!/usr/bin/env bash
# E2E harness for local/dev. Supports:
# - live: boots infra + selected services, then runs smoke checks
# - inprocess: runs Rust test crate(s) directly (fast, no servers)

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

MODE="${RHELMA_E2E_MODE:-live}"
BOOT="${RHELMA_E2E_BOOT:-1}"
INFRA="${RHELMA_E2E_INFRA:-1}"
SERVICES_CSV="${RHELMA_E2E_SERVICES:-api-gateway,search-service}"
WAIT_TIMEOUT_SEC="${RHELMA_E2E_WAIT_TIMEOUT_SEC:-45}"

# URLs (override per env if needed)
API_GATEWAY_URL="${RHELMA_E2E_API_GATEWAY_URL:-http://127.0.0.1:3000}"
AI_ORCH_URL="${RHELMA_E2E_AI_ORCH_URL:-http://127.0.0.1:4000}"
SEARCH_URL="${RHELMA_E2E_SEARCH_URL:-http://127.0.0.1:8082}"
FILE_STORAGE_URL="${RHELMA_E2E_FILE_STORAGE_URL:-http://127.0.0.1:3005}"
REALTIME_URL="${RHELMA_E2E_REALTIME_URL:-http://127.0.0.1:9000}"
NODE_REGISTRY_URL="${RHELMA_E2E_NODE_REGISTRY_URL:-http://127.0.0.1:8090}"
LLM_NODE_URL="${RHELMA_E2E_LLM_NODE_URL:-http://127.0.0.1:8088}"

E2E_DIR="$ROOT_DIR/.e2e"
LOGS_DIR="$E2E_DIR/logs"
PIDS_DIR="$E2E_DIR/pids"
mkdir -p "$LOGS_DIR" "$PIDS_DIR"

need() {
  command -v "$1" >/dev/null 2>&1 || { echo "e2e_local: missing dependency: $1" >&2; exit 2; }
}

need curl

has_service() {
  local name="$1"
  [[ ",${SERVICES_CSV}," == *",${name},"* ]]
}

wait_for_http_ok() {
  local url="$1"
  local deadline=$(( $(date +%s) + WAIT_TIMEOUT_SEC ))
  while true; do
    if curl -fsS --max-time 2 "$url" >/dev/null 2>&1; then
      return 0
    fi
    if [[ $(date +%s) -ge $deadline ]]; then
      echo "e2e_local: timed out waiting for $url" >&2
      return 1
    fi
    sleep 0.5
  done
}

dump_logs() {
  echo "\n--- e2e_local: logs (tail) ---" >&2
  if [[ -d "$LOGS_DIR" ]]; then
    shopt -s nullglob
    for f in "$LOGS_DIR"/*.log; do
      echo "\n### $(basename "$f")" >&2
      tail -n 120 "$f" >&2 || true
    done
    shopt -u nullglob
  fi
}

cleanup() {
  local code=$?

  # Stop booted services.
  if [[ -d "$PIDS_DIR" ]]; then
    for pidfile in "$PIDS_DIR"/*.pid; do
      [[ -e "$pidfile" ]] || continue
      pid="$(cat "$pidfile" 2>/dev/null || true)"
      if [[ -n "$pid" ]]; then
        kill "$pid" >/dev/null 2>&1 || true
      fi
    done
  fi

  # Stop infra.
  if [[ "$MODE" == "live" && "$INFRA" == "1" ]]; then
    if command -v docker >/dev/null 2>&1; then
      (docker compose -f docker-compose.dev.yml down >/dev/null 2>&1) || true
    fi
  fi

  if [[ $code -ne 0 ]]; then
    dump_logs
  fi
  exit $code
}
trap cleanup EXIT

boot_infra() {
  if [[ "$INFRA" != "1" ]]; then
    return 0
  fi
  need docker

  echo "e2e_local: starting infra (docker-compose.dev.yml)"
  docker compose -f docker-compose.dev.yml up -d
}

boot_service() {
  local pkg="$1"
  local log="$LOGS_DIR/${pkg}.log"
  local pidfile="$PIDS_DIR/${pkg}.pid"

  echo "e2e_local: starting service $pkg"
  (cargo run -q -p "$pkg" >"$log" 2>&1 & echo $! >"$pidfile")

  sleep 0.3
  local pid
  pid="$(cat "$pidfile" 2>/dev/null || true)"
  if [[ -n "$pid" ]] && ! kill -0 "$pid" >/dev/null 2>&1; then
    echo "e2e_local: service $pkg exited early" >&2
    tail -n 120 "$log" >&2 || true
    exit 1
  fi
}

wait_for_readiness() {
  echo "e2e_local: waiting for services (timeout ${WAIT_TIMEOUT_SEC}s)"

  if has_service api-gateway; then
    wait_for_http_ok "${API_GATEWAY_URL%/}/health/ready"
  fi
  if has_service ai-orchestrator; then
    wait_for_http_ok "${AI_ORCH_URL%/}/ready"
  fi
  if has_service search-service; then
    wait_for_http_ok "${SEARCH_URL%/}/admin/health"
  fi
  if has_service file-storage-service; then
    wait_for_http_ok "${FILE_STORAGE_URL%/}/health"
  fi
  if has_service realtime-service; then
    wait_for_http_ok "${REALTIME_URL%/}/readyz"
  fi
  if has_service node-registry; then
    wait_for_http_ok "${NODE_REGISTRY_URL%/}/readyz"
  fi
  if has_service rhelma-node; then
    wait_for_http_ok "${LLM_NODE_URL%/}/health"
  fi
}

run_smoke() {
  echo "e2e_local: running smoke checks"

  # Wire smoke URLs from the E2E harness.
  export RHELMA_E2E_API_GATEWAY_URL="$API_GATEWAY_URL"
  export RHELMA_E2E_AI_ORCH_URL="$AI_ORCH_URL"
  export RHELMA_E2E_SEARCH_URL="$SEARCH_URL"
  export RHELMA_E2E_FILE_STORAGE_URL="$FILE_STORAGE_URL"
  export RHELMA_E2E_REALTIME_URL="$REALTIME_URL"
  export RHELMA_E2E_NODE_REGISTRY_URL="$NODE_REGISTRY_URL"
  export RHELMA_E2E_LLM_NODE_URL="$LLM_NODE_URL"
  export RHELMA_E2E_WAIT_TIMEOUT_SEC="$WAIT_TIMEOUT_SEC"

  # Set smoke skip flags based on selected services.
  export RHELMA_SMOKE_SKIP_API_GATEWAY="$(has_service api-gateway && echo 0 || echo 1)"
  export RHELMA_SMOKE_SKIP_AI_ORCH="$(has_service ai-orchestrator && echo 0 || echo 1)"
  export RHELMA_SMOKE_SKIP_SEARCH="$(has_service search-service && echo 0 || echo 1)"
  export RHELMA_SMOKE_SKIP_FILE_STORAGE="$(has_service file-storage-service && echo 0 || echo 1)"
  export RHELMA_SMOKE_SKIP_REALTIME="$(has_service realtime-service && echo 0 || echo 1)"
  export RHELMA_SMOKE_SKIP_NODE_REGISTRY="$(has_service node-registry && echo 0 || echo 1)"
  export RHELMA_SMOKE_SKIP_LLM_NODE="$(has_service rhelma-node && echo 0 || echo 1)"

  bash scripts/smoke_local.sh
}

case "$MODE" in
  inprocess)
    echo "e2e_local: inprocess mode"
    cargo test -q -p e2e-tests
    ;;
  live)
    echo "e2e_local: live mode"

    if command -v bash >/dev/null 2>&1; then
      # Optional preflight (best-effort).
      if [[ -x scripts/setup/preflight.sh ]]; then
        bash scripts/setup/preflight.sh || true
      fi
    fi

    boot_infra

    if [[ "$BOOT" == "1" ]]; then
      IFS=',' read -r -a svcs <<<"$SERVICES_CSV"
      for s in "${svcs[@]}"; do
        s="$(echo "$s" | xargs)"
        [[ -n "$s" ]] || continue
        boot_service "$s"
      done
    fi

    wait_for_readiness
    run_smoke
    echo "e2e_local: OK"
    ;;
  *)
    echo "e2e_local: unknown RHELMA_E2E_MODE='$MODE' (expected live|inprocess)" >&2
    exit 2
    ;;
esac
