#!/usr/bin/env bash
set -euo pipefail

# Simple developer report for discovering phase-scoped stubs.
# It searches for common keywords in Rust sources and prints a short list.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}" )/../.." && pwd)"
cd "$ROOT_DIR"

have() { command -v "$1" >/dev/null 2>&1; }

echo "== Rhelma stub report =="

PATTERN='\bstub\b|TODO:.*stub|intentionally a stub|stubs / opt-in wiring'

if have rg; then
  rg -n --hidden --glob '!target/**' --glob '!**/node_modules/**' -S "$PATTERN" apps crates observability || true
else
  # grep -E does not support \b, so we approximate boundaries using non-word chars.
  grep -RInE --exclude-dir=target --exclude-dir=node_modules --exclude-dir=.git \
    '(^|[^A-Za-z0-9_])stub([^A-Za-z0-9_]|$)|TODO:.*stub|intentionally a stub|stubs / opt-in wiring' \
    apps crates observability || true
fi

echo ""
echo "Tip: see docs/reference/KNOWN_STUBS_AND_PHASED_WIRING.md"
