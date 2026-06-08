#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}" )/../.." && pwd)"
ALLOWLIST_FILE="$ROOT_DIR/.todo-allowlist"

# Scan only code-bearing directories. We intentionally exclude docs/ and scripts/
# to avoid false positives from policy text.
SEARCH_DIRS=(
  "$ROOT_DIR/apps"
  "$ROOT_DIR/crates"
  "$ROOT_DIR/observability"
  "$ROOT_DIR/extras"
  "$ROOT_DIR/infra"
)

# NOTE: Prefer grep's built-in word matching for portability.
# (grep -E does NOT support \\b on many platforms)

matches=()
for d in "${SEARCH_DIRS[@]}"; do
  [[ -d "$d" ]] || continue
  while IFS= read -r line; do
    matches+=("$line")
  done < <(grep -RIn -w --exclude-dir=target --exclude-dir=node_modules \
    --exclude=Cargo.lock --exclude=package-lock.json --exclude=yarn.lock \
    -e 'TODO' -e 'FIXME' -e 'HACK' "$d" || true)
done

if [[ ${#matches[@]} -eq 0 ]]; then
  echo "todo_guard: OK (no TODO/FIXME/HACK found in code dirs)"
  exit 0
fi

# Apply allowlist (line-based, extended regexes). Empty allowlist => no filtering.
filtered=()
if [[ -f "$ALLOWLIST_FILE" ]]; then
  mapfile -t patterns < <(grep -vE '^[[:space:]]*(#|$)' "$ALLOWLIST_FILE" || true)
else
  patterns=()
fi

if [[ ${#patterns[@]} -eq 0 ]]; then
  filtered=("${matches[@]}")
else
  for m in "${matches[@]}"; do
    allowed=false
    for p in "${patterns[@]}"; do
      if [[ "$m" =~ $p ]]; then
        allowed=true
        break
      fi
    done
    if [[ "$allowed" == false ]]; then
      filtered+=("$m")
    fi
  done
fi

if [[ ${#filtered[@]} -eq 0 ]]; then
  echo "todo_guard: OK (matches are allowlisted)"
  exit 0
fi

echo "todo_guard: FAIL — found TODO/FIXME/HACK that must be resolved or allowlisted:"
printf '%s\n' "${filtered[@]}"
exit 1
