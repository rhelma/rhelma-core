#!/usr/bin/env bash
# Runs the standard developer verification suite.
#
# Default: fast, local, deterministic.
# Optional: set RHELMA_TEST_LIVE=1 to boot infra + core services and run live smoke.

set -euo pipefail

bash scripts/verify_all.sh
bash scripts/e2e_local.sh

if [[ "${RHELMA_TEST_LIVE:-0}" == "1" ]]; then
  RHELMA_E2E_MODE=live RHELMA_E2E_BOOT=1 RHELMA_E2E_SERVICES=core bash scripts/e2e_local.sh
fi

echo "[test_all] OK"
