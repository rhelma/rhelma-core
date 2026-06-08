#!/usr/bin/env bash
set -euo pipefail

# Starts the "First Realm" stack + multi-frontend gateway.
# Optionally start admin-web dev server separately using scripts/run-admin-web-dev.sh

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}" )/../.." && pwd)"
cd "$ROOT_DIR"

export RUST_LOG="${RUST_LOG:-info}"

# Start core realm stack (node-registry + gossip-discovery + realm-hub + ai-companion)
"$ROOT_DIR/scripts/run-first-realm.sh" &
CORE_PID=$!

# Start multi-frontend gateway (serves /, /admin, /admin/app)
"$ROOT_DIR/scripts/run-multi-frontend.sh" &
FRONT_PID=$!

trap 'echo "stopping..."; kill $FRONT_PID $CORE_PID 2>/dev/null || true; wait 2>/dev/null || true' INT TERM

echo ""
echo "Rhelma world stack is running."
echo "- Web:           http://localhost:8080/"
echo "- Admin (Rust):  http://localhost:8080/admin"
echo "- Admin Web:     http://localhost:8080/admin/app"
echo "- Realm Hub:     http://localhost:9110/healthz"
echo "- AI Companion:  http://localhost:9120/healthz"
echo ""

echo "Tip: start the admin-web dev server with: $ROOT_DIR/scripts/run-admin-web-dev.sh"

wait $CORE_PID $FRONT_PID
