#!/usr/bin/env bash
set -euo pipefail

root="${1:-.}"

echo "[contract_guard] scanning for legacy HTTP headers under: $root"

patterns=(
  '"x-request-id"' 'x-request-id'
  '"x-correlation-id"' 'x-correlation-id'
  '"x-trace-id"' 'x-trace-id'
  '"x-span-id"' 'x-span-id'
  '"x-rhelma-trace-id"' 'x-rhelma-trace-id'
  '"x-rhelma-span-id"' 'x-rhelma-span-id'
)

# Paths we allow to *mention* legacy headers (docs, tests, internal libs, tooling).
allow_re='(README|CHANGELOG|docs/|\.md$|migrations/|tests/|crates/|extras/|scripts/|target/|\.cargo/|\.git/|node_modules/)'

rg_excludes=(
  --glob '!.git/**'
  --glob '!target/**'
  --glob '!.cargo/**'
  --glob '!node_modules/**'
  --glob '!scripts/guards/**'
)

violations=0

for p in "${patterns[@]}"; do
  if command -v rg >/dev/null 2>&1; then
    # Use globs to skip build artifacts + VCS + guard scripts themselves.
    if matches=$(rg -n --fixed-strings --hidden --no-ignore-vcs "${rg_excludes[@]}" "$p" "$root" 2>/dev/null | rg -v "$allow_re" || true); then
      if [[ -n "$matches" ]]; then
        echo "[contract_guard] FAIL: found legacy header pattern: $p" >&2
        echo "$matches" >&2
        echo >&2
        violations=1
      fi
    fi
  else
    # Fallback grep (best-effort exclusions).
    if matches=$(grep -RIn --exclude-dir=.git --exclude-dir=target --exclude-dir=.cargo --exclude-dir=node_modules "$p" "$root" 2>/dev/null | grep -Ev "$allow_re" || true); then
      if [[ -n "$matches" ]]; then
        echo "[contract_guard] FAIL: found legacy header pattern: $p" >&2
        echo "$matches" >&2
        echo >&2
        violations=1
      fi
    fi
  fi

done

if [[ "$violations" -ne 0 ]]; then
  exit 1
fi

echo "[contract_guard] OK"
exit 0
