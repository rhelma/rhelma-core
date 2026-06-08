#!/usr/bin/env bash
set -euo pipefail

ROOT="${1:-.}"
cd "$ROOT"

fail() { echo "❌ $1" >&2; exit 1; }

# Legacy headers that must not appear in app surfaces.
DISALLOWED=(
  'x-request-id'
  'x-correlation-id'
  'x-rhelma-trace-id'
  'x-rhelma-span-id'
)

# If RHELMA_GUARDS_STRICT_X_RHELMA=1, enforce that only these x-rhelma-* headers are allowed
# in app surfaces. Default is off because some subsystems use additional x-rhelma-* headers
# (admin tokens, error envelopes, ingress metadata, etc.).
ALLOWED_X_RHELMA_RE='x-rhelma-(request-id|correlation-id)'
STRICT_X_RHELMA="${RHELMA_GUARDS_STRICT_X_RHELMA:-0}"

# Scan only app entrypoints/surfaces.
SCAN_ROOTS=(
  'apps'
)

# Skip obvious non-source locations.
SKIP_RE='(target/|node_modules/|\.git/|dist/|build/|coverage/|tests/|fixtures/)'

ALLOWLIST_FILE="scripts/guards/header_contract_guard_allowlist.txt"

filter_hits() {
  local in="$1"
  local out="$in"
  if [[ -n "$out" ]]; then
    out=$(printf "%s\n" "$out" | grep -Ev "$SKIP_RE" || true)
  fi
  if [[ -n "$out" && -f "$ALLOWLIST_FILE" ]]; then
    out=$(printf "%s\n" "$out" | grep -vFf "$ALLOWLIST_FILE" || true)
  fi
  printf "%s" "$out"
}

scan_any() {
  local ptn="$1"
  local insensitive="${2:-0}"
  local raw=""
  if command -v rg >/dev/null 2>&1; then
    if [[ "$insensitive" == "1" ]]; then
      raw=$(rg -n -i --hidden --no-ignore-vcs --glob '!target/**' --glob '!node_modules/**' --glob '!**/*.md' --glob '*.rs' --glob '*.js' --glob '*.ts' "$ptn" "${SCAN_ROOTS[@]}" 2>/dev/null || true)
    else
      raw=$(rg -n --hidden --no-ignore-vcs --glob '!target/**' --glob '!node_modules/**' --glob '!**/*.md' --glob '*.rs' --glob '*.js' --glob '*.ts' "$ptn" "${SCAN_ROOTS[@]}" 2>/dev/null || true)
    fi
  else
    if [[ "$insensitive" == "1" ]]; then
      raw=$(grep -RIn --exclude-dir=target --exclude-dir=.git --exclude-dir=node_modules --include='*.rs' --include='*.js' --include='*.ts' -i -F "$ptn" "${SCAN_ROOTS[@]}" 2>/dev/null || true)
    else
      raw=$(grep -RIn --exclude-dir=target --exclude-dir=.git --exclude-dir=node_modules --include='*.rs' --include='*.js' --include='*.ts' -F "$ptn" "${SCAN_ROOTS[@]}" 2>/dev/null || true)
    fi
  fi
  filter_hits "$raw"
}

HITS=0
for ptn in "${DISALLOWED[@]}"; do
  hits=$(scan_any "$ptn")
  if [[ -n "$hits" ]]; then
    echo "----"
    echo "Found disallowed legacy header token: ${ptn}"
    echo "$hits"
    HITS=1
  fi
done

if [[ "$HITS" -eq 1 ]]; then
  fail "Legacy headers are forbidden in apps/. Use x-rhelma-request-id, x-rhelma-correlation-id, and W3C traceparent."
fi

# Optional strict whitelist for x-rhelma-* headers.
if [[ "$STRICT_X_RHELMA" == "1" ]]; then
  raw_xrhelma=$(scan_any 'x-rhelma-' 1)
  if [[ -n "$raw_xrhelma" ]]; then
    disallowed=$(printf "%s\n" "$raw_xrhelma" | grep -viE "$ALLOWED_X_RHELMA_RE" || true)
    if [[ -n "$disallowed" ]]; then
      echo "----"
      echo "Found disallowed x-rhelma-* header token(s) in apps/ (strict mode):"
      echo "$disallowed"
      fail "Disallowed x-rhelma-* headers detected in apps/ (strict mode)."
    fi
  fi
fi

echo "✅ header_contract_guard: OK"
