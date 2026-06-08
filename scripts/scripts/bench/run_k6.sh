#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

SCENARIO="${1:-api-gateway}"

case "$SCENARIO" in
  api-gateway)
    SCRIPT="benchmarks/k6/api_gateway_load.js"
    ENV_VAR="RHELMA_API_URL"
    DEFAULT_URL="http://localhost:3000"
    ;;
  node-registry)
    SCRIPT="benchmarks/k6/node_registry_load.js"
    ENV_VAR="RHELMA_NODE_REGISTRY_URL"
    DEFAULT_URL="http://localhost:3001"
    ;;
  *)
    echo "Unknown scenario: $SCENARIO"
    echo "Usage: $0 {api-gateway|node-registry}"
    exit 2
    ;;
esac

BASE_URL="${!ENV_VAR:-$DEFAULT_URL}"

echo "==> Running k6: $SCENARIO"
echo "    script: $SCRIPT"
echo "    base url: $BASE_URL"

# Uses the official k6 container so you don't need k6 installed locally.
# Requirements: Docker.
docker run --rm -i \
  -e "$ENV_VAR=$BASE_URL" \
  -v "$ROOT_DIR:/work" \
  -w /work \
  grafana/k6 run "$SCRIPT"
