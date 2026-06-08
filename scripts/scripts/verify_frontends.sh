#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

run() {
  local name="$1"; shift
  echo "\n=== $name ==="
  (cd "$ROOT_DIR" && "$@")
}

# 1) Run the standard pre-frontend gates first.
run "Pre-frontend verification" "$ROOT_DIR/scripts/verify_pre_frontend.sh"

# 2) Backend entry service for multi-frontend must at least typecheck.
run "Rust: multi-frontend (cargo check)" cargo check -p multi-frontend

# 3) Web build should succeed when Node tooling is available.
if command -v npm >/dev/null 2>&1 && [[ -f "$ROOT_DIR/apps/web/package.json" ]]; then
  # Avoid failing repos that don't have optional scripts.
  run "Web: install deps (if needed)" bash -lc "cd apps/web && npm install"
  run "Web: lint (if present)" bash -lc "cd apps/web && npm run lint --if-present"
  run "Web: build (if present)" bash -lc "cd apps/web && npm run build --if-present"
else
  echo "\n[skip] npm/apps/web not available; skipping web checks"
fi

echo "\nverify_frontends: OK"
