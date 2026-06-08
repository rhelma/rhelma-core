#!/usr/bin/env bash
set -euo pipefail

REGISTRY_URL="${REGISTRY_URL:-http://127.0.0.1:8090}"
PARALLEL="${PARALLEL:-50}"
NODE_BIN="${NODE_BIN:-cargo run -q -p rhelma-node --}"

echo "[e2e] burst $PARALLEL registrations (expect rate-limit kicks in)"
fail=0
pids=()

for i in $(seq 1 "$PARALLEL"); do
  (
    $NODE_BIN init >/dev/null
    $NODE_BIN register --registry "$REGISTRY_URL" >/dev/null
  ) &
  pids+=($!)
done

for p in "${pids[@]}"; do
  wait "$p" || fail=$((fail+1))
done

echo "[e2e] completed; failures=$fail (non-zero expected if rate limiting enabled)"
exit 0
