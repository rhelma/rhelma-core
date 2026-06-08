#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# Services that expose HTTP and therefore should have OpenAPI scaffolds.
HTTP_SERVICES_FILE="$ROOT_DIR/docs/reference/http_services.txt"

# Apps that are not long-running services (static sites / CLI tools).
NON_DAEMON_APPS_FILE="$ROOT_DIR/docs/reference/non_daemon_apps.txt"

req_sections=("Overview" "Run" "Configuration" "Endpoints" "Observability" "Security" "Verification")

missing_count=0

echo "# Rhelma completeness report"
echo
echo "Generated: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
echo
echo "Legend: ✅ ok · ⚠️ partial · ❌ missing"
echo

printf "| Service | README | Sections | Health/metrics mentioned | Runbook | Tests dir | OpenAPI |\n"
printf "|---|---:|---:|---:|---:|---:|---:|\n"

for d in "$ROOT_DIR"/apps/*; do
  [[ -d "$d" ]] || continue
  svc="$(basename "$d")"
  readme="$d/README.md"

  readme_status="❌"
  sections_status="❌"
  hm_status="❌"
  runbook_status="❌"
  tests_status="❌"
  openapi_status="❌"

  is_http_service=0
  if [[ -f "$HTTP_SERVICES_FILE" ]]; then
    if grep -Fxq "${svc}" <(grep -Ev '^[[:space:]]*(#|$)' "$HTTP_SERVICES_FILE") 2>/dev/null; then
      is_http_service=1
    fi
  fi

  is_non_daemon=0
  if [[ -f "$NON_DAEMON_APPS_FILE" ]]; then
    if grep -Fxq "${svc}" <(grep -Ev '^[[:space:]]*(#|$)' "$NON_DAEMON_APPS_FILE") 2>/dev/null; then
      is_non_daemon=1
    fi
  fi

  if [[ -f "$readme" ]]; then
    readme_status="✅"

    # Sections check (case-insensitive): any "##" line containing the keyword.
    sec_ok=0
    for s in "${req_sections[@]}"; do
      if grep -Eqi "^##+\s+.*${s}" "$readme"; then
        ((sec_ok++)) || true
      fi
    done
    if [[ $sec_ok -ge 6 ]]; then
      sections_status="✅"
    elif [[ $sec_ok -ge 3 ]]; then
      sections_status="⚠️"
    else
      sections_status="❌"
    fi

    if [[ $is_non_daemon -eq 1 ]]; then
      hm_status="—"
    else
      if grep -Eqi "healthz|/health" "$readme" && grep -Eqi "metrics|/metrics" "$readme"; then
        hm_status="✅"
      elif grep -Eqi "healthz|/health|metrics|/metrics" "$readme"; then
        hm_status="⚠️"
      else
        hm_status="❌"
      fi
    fi
  else
    missing_count=$((missing_count + 1))
  fi

  # Runbook naming convention: docs/runbooks/service_<name>.md (hyphens -> underscores)
  rb_name="service_${svc//-/_}.md"
  if [[ -f "$ROOT_DIR/docs/runbooks/$rb_name" ]]; then
    runbook_status="✅"
  fi

  if [[ -d "$d/tests" ]]; then
    tests_status="✅"
  fi

  if [[ $is_http_service -eq 1 ]]; then
    if [[ -f "$ROOT_DIR/docs/openapi/${svc}.yaml" ]]; then
      openapi_status="✅"
    else
      openapi_status="❌"
    fi
  else
    openapi_status="—"
  fi

  printf "| %s | %s | %s | %s | %s | %s | %s |\n" "$svc" "$readme_status" "$sections_status" "$hm_status" "$runbook_status" "$tests_status" "$openapi_status"

  # Completeness gating heuristics (opt-in)
  if [[ "${RHELMA_VERIFY_COMPLETENESS:-0}" == "1" ]]; then
    if [[ "$readme_status" != "✅" || "$hm_status" == "❌" ]]; then
      missing_count=$((missing_count + 1))
    fi
  fi
done

echo
echo "Tip: known phased stubs -> bash scripts/dev/stub-report.sh"
echo "Tip: completeness target -> docs/reference/COMPLETENESS_MATRIX.md"

if [[ "${RHELMA_VERIFY_COMPLETENESS:-0}" == "1" && $missing_count -gt 0 ]]; then
  echo
  echo "Completeness gate failed: $missing_count issue(s)" >&2
  exit 1
fi

exit 0
