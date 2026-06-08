#!/usr/bin/env bash
set -euo pipefail

ROOT="${1:-.}"
echo "[uuidv7_guard] scanning for UUIDv4 usage near request/correlation/event identifiers under: $ROOT"

ALLOW_RE='(tests/|\.md$|migrations/)'
FAIL=0

if command -v rg >/dev/null 2>&1; then
  PATTERNS=(
    'request[_-]?id[^\n]{0,120}Uuid::new_v4\('
    'correlation[_-]?id[^\n]{0,120}Uuid::new_v4\('
    'event[_-]?id[^\n]{0,120}Uuid::new_v4\('
    'generate_(request|correlation|event)_id\([^\)]*\)[^{]*\{[^}]*Uuid::new_v4\('
  )

  for p in "${PATTERNS[@]}"; do
    if rg -n --hidden --no-ignore-vcs -U "$p" "$ROOT" | rg -v "$ALLOW_RE" ; then
      echo ""
      echo "[uuidv7_guard] FAIL: possible UUIDv4 usage where UUIDv7 is expected: $p"
      FAIL=1
    fi
  done
else
  # Fallback: approximate scan without multiline regex support.
  # We flag any `Uuid::new_v4()` line that also mentions request/correlation/event identifiers.
  matches=$(grep -RIn --exclude-dir=target --exclude-dir=.git --exclude-dir=node_modules \
    --include='*.rs' 'Uuid::new_v4(' "$ROOT" 2>/dev/null | \
    grep -Ev "$ALLOW_RE" | \
    grep -Ei 'request[_-]?id|correlation[_-]?id|event[_-]?id|generate_(request|correlation|event)_id' || true)

  if [[ -n "$matches" ]]; then
    echo ""
    echo "[uuidv7_guard] FAIL: possible UUIDv4 usage where UUIDv7 is expected (fallback scan)"
    echo "$matches"
    FAIL=1
  fi
fi

if [[ "$FAIL" -eq 1 ]]; then
  echo ""
  echo "[uuidv7_guard] Expected: Uuid::now_v7() for request_id / correlation_id / event_id"
  exit 1
fi

echo "[uuidv7_guard] OK"
