#!/usr/bin/env bash
set -euo pipefail

REGISTRY_URL="${REGISTRY_URL:-http://127.0.0.1:8090}"
NODE_BIN="${NODE_BIN:-cargo run -q -p rhelma-node --}"

echo "[e2e] init node"
$NODE_BIN init >/dev/null

echo "[e2e] register once (should succeed)"
$NODE_BIN register --registry "$REGISTRY_URL" >/dev/null

echo "[e2e] attempt replay register (should fail)"
set +e
$NODE_BIN register --registry "$REGISTRY_URL" --replay-last-admission-proof >/dev/null
RC=$?
set -e

if [ "$RC" -eq 0 ]; then
  echo "FAIL: replay register unexpectedly succeeded"
  exit 1
fi

echo "OK: replay rejected"
