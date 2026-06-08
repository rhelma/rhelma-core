#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

run() {
  local name="$1"; shift
  echo "\n=== $name ==="
  (cd "$ROOT_DIR" && "$@")
}

run "Repo structure" "$ROOT_DIR/scripts/check-structure.sh"
run "Core verify" "$ROOT_DIR/scripts/verify.sh"
run "Observability verify" "$ROOT_DIR/scripts/verify_observability.sh"
run "Contract guard" "$ROOT_DIR/scripts/contract_guard.sh"
run "Env contract guard" "$ROOT_DIR/scripts/env_contract_guard.sh"
run "Event contract guard" "$ROOT_DIR/scripts/event_contract_guard.sh"
run "UUIDv7 guard" "$ROOT_DIR/scripts/uuidv7_guard.sh"
run "Scrapeability guard" "$ROOT_DIR/scripts/scrapeability_guard.sh"
run "Metrics cardinality guard" "$ROOT_DIR/scripts/metrics_cardinality_guard.sh"

# Optional: outbound HTTP context guard (introduced in later phases)
if [[ -f "$ROOT_DIR/scripts/outbound_http_context_guard.py" ]]; then
  run "Outbound HTTP context guard" python3 "$ROOT_DIR/scripts/outbound_http_context_guard.py"
fi

run "TODO/FIXME/HACK guard" "$ROOT_DIR/scripts/todo_guard.sh"

echo "\nverify_pre_frontend: OK"
