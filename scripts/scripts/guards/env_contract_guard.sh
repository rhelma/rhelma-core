#!/usr/bin/env bash
set -euo pipefail

ROOT="${1:-.}"

fail() { echo "❌ $1" >&2; exit 1; }

# Enforce env/region contract primarily at *app surfaces*.
# (Crates may offer helpers, but apps should rely on rhelma-config CentralEnv.)
SCAN_ROOTS=(
  'apps'
  'observability'
)

# Forbidden: direct reads of env/region from process env.
DISALLOWED_CALL_PATTERNS=(
  # Direct calls with string literal.
  'env::var\("RHELMA_ENV"\)'
  'env::var\("RHELMA_ENVIRONMENT"\)'
  'env::var\("RHELMA_REGION"\)'
  'env::var\("RHELMA_ENV_NAME"\)'
  'std::env::var\("RHELMA_ENV"\)'
  'std::env::var\("RHELMA_ENVIRONMENT"\)'
  'std::env::var\("RHELMA_REGION"\)'
  'std::env::var\("RHELMA_ENV_NAME"\)'
  'env::var_os\("RHELMA_ENV"\)'
  'env::var_os\("RHELMA_ENVIRONMENT"\)'
  'env::var_os\("RHELMA_REGION"\)'
  'env::var_os\("RHELMA_ENV_NAME"\)'
  'std::env::var_os\("RHELMA_ENV"\)'
  'std::env::var_os\("RHELMA_ENVIRONMENT"\)'
  'std::env::var_os\("RHELMA_REGION"\)'
  'std::env::var_os\("RHELMA_ENV_NAME"\)'

  # Common alternative wrappers.
  '\.var\("RHELMA_ENV"\)'
  '\.var\("RHELMA_ENVIRONMENT"\)'
  '\.var\("RHELMA_REGION"\)'
)

# Close const-key loophole: forbid defining these keys as constants in app code.
DISALLOWED_CONST_PATTERNS=(
  'const[[:space:]]+[A-Za-z0-9_]+[[:space:]]*:[[:space:]]*&?[[:space:]]*str[[:space:]]*=[[:space:]]*"RHELMA_ENV"'
  'const[[:space:]]+[A-Za-z0-9_]+[[:space:]]*:[[:space:]]*&?[[:space:]]*str[[:space:]]*=[[:space:]]*"RHELMA_ENVIRONMENT"'
  'const[[:space:]]+[A-Za-z0-9_]+[[:space:]]*:[[:space:]]*&?[[:space:]]*str[[:space:]]*=[[:space:]]*"RHELMA_REGION"'
  'const[[:space:]]+[A-Za-z0-9_]+[[:space:]]*:[[:space:]]*&?[[:space:]]*str[[:space:]]*=[[:space:]]*"RHELMA_ENV_NAME"'
  'static[[:space:]]+[A-Za-z0-9_]+[[:space:]]*:[[:space:]]*&?[[:space:]]*str[[:space:]]*=[[:space:]]*"RHELMA_ENV"'
  'static[[:space:]]+[A-Za-z0-9_]+[[:space:]]*:[[:space:]]*&?[[:space:]]*str[[:space:]]*=[[:space:]]*"RHELMA_ENVIRONMENT"'
  'static[[:space:]]+[A-Za-z0-9_]+[[:space:]]*:[[:space:]]*&?[[:space:]]*str[[:space:]]*=[[:space:]]*"RHELMA_REGION"'
  'static[[:space:]]+[A-Za-z0-9_]+[[:space:]]*:[[:space:]]*&?[[:space:]]*str[[:space:]]*=[[:space:]]*"RHELMA_ENV_NAME"'
)

# Allowed hint: central env loader (strict)
ALLOWED_HINTS=(
  'CentralEnv::from_env_strict'
)

# Skip docs/tests/examples/migrations and build artifacts
ALLOW_PATH_RE='(tests/|examples/|\\.md$|migrations/|target/|node_modules/|\\.git/)'

cd "$ROOT"

ALLOWLIST_FILE="scripts/guards/env_contract_guard_allowlist.txt"

filter_hits() {
  local in="$1"
  local out="$in"
  if [[ -n "$out" ]]; then
    out=$(printf "%s\n" "$out" | grep -Ev "$ALLOW_PATH_RE" || true)
  fi
  if [[ -n "$out" && -f "$ALLOWLIST_FILE" ]]; then
    out=$(printf "%s\n" "$out" | grep -vFf "$ALLOWLIST_FILE" || true)
  fi
  printf "%s" "$out"
}

scan() {
  local ptn="$1"
  local raw=""
  if command -v rg >/dev/null 2>&1; then
    raw=$(rg -n --hidden --no-ignore-vcs --glob '!target/**' --glob '!node_modules/**' --glob '!**/*.md' --glob '*.rs' "$ptn" "${SCAN_ROOTS[@]}" 2>/dev/null || true)
  else
    raw=$(grep -RIn --exclude-dir=target --exclude-dir=.git --exclude-dir=node_modules --include='*.rs' -E "$ptn" "${SCAN_ROOTS[@]}" 2>/dev/null || true)
  fi
  filter_hits "$raw"
}

HITS=0

for ptn in "${DISALLOWED_CALL_PATTERNS[@]}"; do
  hits=$(scan "$ptn")
  if [[ -n "$hits" ]]; then
    echo "----"
    echo "Found disallowed env access pattern: $ptn"
    echo "$hits"
    HITS=1
  fi
done

for ptn in "${DISALLOWED_CONST_PATTERNS[@]}"; do
  hits=$(scan "$ptn")
  if [[ -n "$hits" ]]; then
    echo "----"
    echo "Found disallowed env key const/static (const-key loophole): $ptn"
    echo "$hits"
    HITS=1
  fi
done

# Optional sanity: if no trace of CentralEnv, warn (not fail)
FOUND_ALLOWED=0
for a in "${ALLOWED_HINTS[@]}"; do
  if command -v rg >/dev/null 2>&1; then
    raw=$(rg -n --hidden --no-ignore-vcs --glob '!target/**' --glob '!node_modules/**' --glob '!**/*.md' --glob '*.rs' "$a" "${SCAN_ROOTS[@]}" 2>/dev/null || true)
    raw=$(filter_hits "$raw")
    if [[ -n "$raw" ]]; then FOUND_ALLOWED=1; break; fi
  else
    raw=$(grep -RIn --exclude-dir=target --exclude-dir=.git --exclude-dir=node_modules --include='*.rs' -F "$a" "${SCAN_ROOTS[@]}" 2>/dev/null || true)
    raw=$(filter_hits "$raw")
    if [[ -n "$raw" ]]; then FOUND_ALLOWED=1; break; fi
  fi
done

if [[ "$HITS" -eq 1 ]]; then
  fail "Direct/indirect access to RHELMA_ENV/RHELMA_REGION is forbidden in app surfaces. Use CentralEnv::from_env_strict()."
fi

if [[ "$FOUND_ALLOWED" -eq 0 ]]; then
  echo "⚠️  Warning: No CentralEnv::from_env_strict found in app surfaces. Are configs migrated?"
fi

echo "✅ env_contract_guard: OK"
