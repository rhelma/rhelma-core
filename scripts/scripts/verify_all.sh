#!/usr/bin/env bash
# Full verification suite for the entire repository.
#
# This is the "one command" gate you can run locally and in CI before adding/merging changes.
# It runs:
#   - Repo structure checks
#   - Rust fmt / clippy / tests
#   - Observability verification
#   - Contract & env/event anti-drift guards
#   - UUIDv7, scrapeability, metrics-cardinality, TODO guards
#   - Optional outbound HTTP context guard

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

run() {
  local name="$1"; shift
  echo -e "\n=== ${name} ==="
  (cd "$ROOT_DIR" && "$@")
}

run "Repo structure" "$ROOT_DIR/scripts/check-structure.sh"

# Core Rust verification
run "Rust verify (fmt/clippy/tests)" bash "$ROOT_DIR/scripts/verify.sh"

# Observability + contract gates
run "Observability verify" bash "$ROOT_DIR/scripts/verify_observability.sh" .
run "Contract guard" bash "$ROOT_DIR/scripts/contract_guard.sh"
run "Env contract guard" bash "$ROOT_DIR/scripts/env_contract_guard.sh"
run "Event contract guard" bash "$ROOT_DIR/scripts/event_contract_guard.sh"

# Quality/consistency guards
run "UUIDv7 guard" bash "$ROOT_DIR/scripts/uuidv7_guard.sh"
run "Scrapeability guard" bash "$ROOT_DIR/scripts/scrapeability_guard.sh"
run "Metrics cardinality guard" bash "$ROOT_DIR/scripts/metrics_cardinality_guard.sh"

# Optional: outbound HTTP context guard
if [[ -f "$ROOT_DIR/scripts/outbound_http_context_guard.py" ]]; then
  if command -v python3 >/dev/null 2>&1; then
    run "Outbound HTTP context guard" python3 "$ROOT_DIR/scripts/outbound_http_context_guard.py"
  else
    echo "python3 not found; skipping outbound_http_context_guard.py"
  fi
fi

run "TODO/FIXME/HACK guard" bash "$ROOT_DIR/scripts/todo_guard.sh"

echo -e "\nverify_all: OK"
