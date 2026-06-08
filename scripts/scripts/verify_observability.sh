#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/_lib.sh"

# Observability / Contract conformance verification (v5.3)
#
# This script is intentionally lightweight and uses best-effort discovery.
# It should be safe to run in CI on Linux.

repo_root="${1:-.}"

bash scripts/contract_guard.sh "$repo_root"
bash scripts/env_contract_guard.sh "$repo_root"
bash scripts/uuidv7_guard.sh "$repo_root"
bash scripts/event_contract_guard.sh "$repo_root"

# Observability anti-drift (metrics)
bash scripts/metrics_cardinality_guard.sh "$repo_root"

# Monitoring anti-drift (scrapeability)
bash scripts/scrapeability_guard.sh "$repo_root"

# Outbound HTTP propagation anti-drift (reqwest)
bash scripts/outbound_http_context_guard.sh "$repo_root"

# Run tests (includes observability conformance/chaos/roundtrip tests when present).
if command -v cargo >/dev/null 2>&1; then
  cargo test --workspace
else
  echo "cargo not found; skipping 'cargo test --workspace' (run scripts/verify.sh on a Rust toolchain host)" >&2
fi
