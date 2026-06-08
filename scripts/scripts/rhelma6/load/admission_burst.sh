#!/usr/bin/env bash
set -euo pipefail

# Simple admission/register burst generator (for staging only).
# Requires curl.

REGISTRY_URL="${RHELMA_REGISTRY_URL:-http://127.0.0.1:8090}"
N="${N:-50}"

for i in $(seq 1 "$N"); do
  curl -sS "$REGISTRY_URL/v1/admission/challenge?node_id=test-$i" >/dev/null || true
done

echo "sent $N challenges"
