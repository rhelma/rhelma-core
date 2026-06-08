#!/usr/bin/env bash
set -euo pipefail

# Run ignored chaos tests with a minimal Docker Compose cluster.

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)

COMPOSE_FILE=${RHELMA_CHAOS_COMPOSE_FILE:-"$ROOT_DIR/deploy/rhelma6/docker/docker-compose.chaos.yml"}
export RHELMA_CHAOS_COMPOSE_FILE="$COMPOSE_FILE"
export COMPOSE_PROJECT_NAME=${COMPOSE_PROJECT_NAME:-rhelma6chaos}

cleanup() {
  set +e
  docker compose -f "$COMPOSE_FILE" down -v --remove-orphans
}

trap cleanup EXIT

docker compose -f "$COMPOSE_FILE" up -d --remove-orphans

start_epoch=$(date +%s)

wait_http_ok() {
  local url=${1:?url}
  local name=${2:-service}
  local max=${3:-60}
  local i=0
  until curl -fsS "$url" >/dev/null 2>&1; do
    i=$((i+1))
    if [[ $i -ge $max ]]; then
      echo "[chaos] timeout waiting for $name at $url" >&2
      return 1
    fi
    sleep 1
  done
  echo "[chaos] ready: $name ($url)" >&2
}

# Default network name used by docker compose.
export RHELMA_CHAOS_DOCKER_NETWORK=${RHELMA_CHAOS_DOCKER_NETWORK:-"${COMPOSE_PROJECT_NAME}_default"}

export RHELMA_CHAOS_NODE_REGISTRY=${RHELMA_CHAOS_NODE_REGISTRY:-"http://localhost:4010,http://localhost:4011,http://localhost:4012"}
export RHELMA_CHAOS_PARTITION_CMD=${RHELMA_CHAOS_PARTITION_CMD:-"$ROOT_DIR/scripts/rhelma6/chaos/network_partition.sh"}
export RHELMA_CHAOS_RESTART_CMD=${RHELMA_CHAOS_RESTART_CMD:-"$ROOT_DIR/scripts/rhelma6/chaos/restart_service.sh"}
export RHELMA_CHAOS_LATENCY_CMD=${RHELMA_CHAOS_LATENCY_CMD:-"$ROOT_DIR/scripts/rhelma6/chaos/netem_latency.sh"}

# Value Ledger (persistent) is included in the chaos compose bundle.
export RHELMA_CHAOS_VALUE_LEDGER=${RHELMA_CHAOS_VALUE_LEDGER:-"http://localhost:${RHELMA_CHAOS_VALUE_LEDGER_PORT:-4030}"}
export RHELMA_CHAOS_VALUE_LEDGER_ADMIN_TOKEN=${RHELMA_CHAOS_VALUE_LEDGER_ADMIN_TOKEN:-"${RHELMA_VALUE_LEDGER__ADMIN_TOKEN:-change-me}"}

wait_http_ok "http://localhost:${RHELMA_CHAOS_NODE_REGISTRY_0_PORT:-4010}/healthz" "node-registry-0" 120
wait_http_ok "http://localhost:${RHELMA_CHAOS_NODE_REGISTRY_1_PORT:-4011}/healthz" "node-registry-1" 120
wait_http_ok "http://localhost:${RHELMA_CHAOS_NODE_REGISTRY_2_PORT:-4012}/healthz" "node-registry-2" 120
wait_http_ok "http://localhost:${RHELMA_CHAOS_VALUE_LEDGER_PORT:-4030}/healthz" "value-ledger" 120

cd "$ROOT_DIR"

# Emit deterministic output for CI artifacting.
mkdir -p benchmarks/out

set +e
cargo test -p e2e-tests --features chaos-tests -- --ignored --nocapture \
  2>&1 | tee "benchmarks/out/chaos_test.log"
status=${PIPESTATUS[0]}
set -e

end_epoch=$(date +%s)
duration_sec=$((end_epoch - start_epoch))

log_file="benchmarks/out/chaos_test.log"
summary_file="benchmarks/out/chaos_summary.json"

extract_num() {
  local label=${1:?label}
  local line=${2:-}
  echo "$line" | sed -nE "s/.* ([0-9]+) ${label}.*/\1/p" | tail -n 1
}

result_line=$(grep -E "^test result:" "$log_file" | tail -n 1 || true)
passed=$(extract_num "passed" "$result_line")
failed=$(extract_num "failed" "$result_line")
ignored=$(extract_num "ignored" "$result_line")
measured=$(extract_num "measured" "$result_line")
filtered=$(extract_num "filtered out" "$result_line")

passed=${passed:-$(grep -E "^test .* \.\.\. ok$" "$log_file" | wc -l | tr -d ' ')}
failed=${failed:-$(grep -E "^test .* \.\.\. FAILED$" "$log_file" | wc -l | tr -d ' ')}
ignored=${ignored:-0}
measured=${measured:-0}
filtered=${filtered:-0}

overall="ok"
if [[ "$status" != "0" ]]; then
  overall="fail"
fi

cat >"$summary_file" <<JSON
{
  "overall": "${overall}",
  "exit_code": ${status},
  "passed": ${passed},
  "failed": ${failed},
  "ignored": ${ignored},
  "measured": ${measured},
  "filtered_out": ${filtered},
  "duration_sec": ${duration_sec}
}
JSON

echo "[chaos] wrote summary: $summary_file" >&2

exit $status
