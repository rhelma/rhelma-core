#!/usr/bin/env bash
set -euo pipefail

# Run k6 load profiles with consistent defaults.
#
# Usage:
#   run_k6_profiles.sh <quick|standard|full> [gateway|node-registry|both]
#
# Notes:
# - This script does NOT start services. Start api-gateway (8080) and/or node-registry (9010) first.

PROFILE=${1:-quick}
TARGET=${2:-both}

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
OUT_DIR=${RHELMA_K6_OUT_DIR:-"$ROOT_DIR/benchmarks/out"}
mkdir -p "$OUT_DIR"

function run_k6() {
  local name=$1
  local script=$2
  local summary=$3
  echo "[k6] $name -> $script" >&2
  k6 run --summary-export "$summary" "$script"
}

case "$PROFILE" in
  quick)
    GW_SCRIPT="$ROOT_DIR/benchmarks/k6/profiles/api_gateway_quick.js"
    NR_SCRIPT="$ROOT_DIR/benchmarks/k6/profiles/node_registry_quick.js"
    ;;
  standard)
    GW_SCRIPT="$ROOT_DIR/benchmarks/k6/profiles/api_gateway_standard.js"
    NR_SCRIPT="$ROOT_DIR/benchmarks/k6/profiles/node_registry_standard.js"
    ;;
  full)
    GW_SCRIPT="$ROOT_DIR/benchmarks/k6/api_gateway_load.js"
    NR_SCRIPT="$ROOT_DIR/benchmarks/k6/node_registry_load.js"
    ;;
  *)
    echo "unknown profile: $PROFILE" >&2
    exit 2
    ;;
 esac

if [[ "$TARGET" == "gateway" || "$TARGET" == "both" ]]; then
  run_k6 "api-gateway:$PROFILE" "$GW_SCRIPT" "$OUT_DIR/k6_gateway_${PROFILE}.json"
fi

if [[ "$TARGET" == "node-registry" || "$TARGET" == "both" ]]; then
  run_k6 "node-registry:$PROFILE" "$NR_SCRIPT" "$OUT_DIR/k6_node_registry_${PROFILE}.json"
fi

echo "[k6] summaries written to: $OUT_DIR" >&2
