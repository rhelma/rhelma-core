#!/usr/bin/env bash
set -euo pipefail

# Scrapeability / metrics endpoint guard (Phase 93)
#
# Ensures:
# 1) Prometheus local config (Phase 92) includes the expected jobs.
# 2) Each job uses metrics_path: /metrics.
# 3) Each service contains a /metrics route (best-effort static scan).

repo_root="${1:-.}"

prom_cfg="$repo_root/infra/monitoring/prometheus/prometheus.yml"

if [[ ! -f "$prom_cfg" ]]; then
  echo "[scrapeability_guard] FAILED: missing $prom_cfg"
  exit 1
fi

need_jobs=(
  "api-gateway"
  "search-service"
  "ai-orchestrator"
  "rhelma-llm-node"
  "realtime-service"
  "file-storage"
)

fail=0

for job in "${need_jobs[@]}"; do
  # NOTE: Use POSIX character classes for portability. (grep -E does NOT support \\s)
  if ! grep -qE "^[[:space:]]*-[[:space:]]*job_name:[[:space:]]*$job[[:space:]]*$" "$prom_cfg"; then
    echo "[FAIL] prometheus.yml is missing job_name: $job"
    fail=1
  fi

done

# Ensure metrics_path is /metrics for all jobs in the file.
# If you need a different path, adjust prometheus.yml and update this guard.
if ! grep -qE "^[[:space:]]*metrics_path:[[:space:]]*/metrics[[:space:]]*$" "$prom_cfg"; then
  echo "[FAIL] prometheus.yml does not contain metrics_path: /metrics"
  fail=1
fi

# Best-effort: verify each service codebase contains the /metrics route.
# Note: this is a static scan; it does not boot the servers.

require_metrics_route() {
  local name="$1"
  shift
  local dirs=("$@")
  local hit=0

  for d in "${dirs[@]}"; do
    local p="$repo_root/$d"
    if [[ -d "$p" ]]; then
      if command -v rg >/dev/null 2>&1; then
        if rg -g'*.rs' -n "\"/metrics\"" "$p" >/dev/null 2>&1; then
          hit=1
          break
        fi
      else
        if grep -R --line-number --include='*.rs' --exclude-dir=target --exclude-dir=.git "\"/metrics\"" "$p" >/dev/null 2>&1; then
          hit=1
          break
        fi
      fi
    fi
  done

  if [[ $hit -eq 0 ]]; then
    echo "[FAIL] could not find a /metrics route for $name (scanned: ${dirs[*]})"
    fail=1
  fi
}

require_metrics_route "api-gateway" "apps/api-gateway"
require_metrics_route "search-service" "apps/search-service"
require_metrics_route "ai-orchestrator" "apps/ai-orchestrator"
require_metrics_route "realtime-service" "apps/realtime-service"
require_metrics_route "file-storage" "apps/file-storage" "apps/file-storage-service" "apps/file-storage-service"
# llm-node lives in extras in this repo layout
require_metrics_route "rhelma-llm-node" "extras/llm-node" "apps/llm-node"

if [[ $fail -ne 0 ]]; then
  echo "[scrapeability_guard] FAILED"
  exit 1
fi

echo "[scrapeability_guard] OK"
