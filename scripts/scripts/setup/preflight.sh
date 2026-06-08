#!/usr/bin/env bash
set -euo pipefail

# Preflight environment check.
#
# هدف: نصب/راه‌اندازی سریع‌تر روی سیستم‌های مختلف (Linux/macOS/WSL).
# این اسکریپت فقط **هشدار** می‌دهد مگر اینکه با --strict اجرا شود.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}" )/../.." && pwd)"
source "$ROOT_DIR/_lib.sh"

STRICT=0
if [[ "${1:-}" == "--strict" ]]; then
  STRICT=1
fi

warn() { echo "⚠️  $*" >&2; }
ok() { echo "✅ $*"; }
die() { echo "❌ $*" >&2; exit 127; }

must() {
  local cmd="$1"; shift
  if ! command -v "$cmd" >/dev/null 2>&1; then
    die "Required command '$cmd' not found. $*"
  fi
  ok "$cmd"
}

maybe() {
  local cmd="$1"; shift
  if command -v "$cmd" >/dev/null 2>&1; then
    ok "$cmd"
  else
    if [[ "$STRICT" -eq 1 ]]; then
      die "Missing recommended command '$cmd'. $*"
    fi
    warn "Missing recommended command '$cmd'. $*"
  fi
}

echo "== Rhelma preflight =="

must git "Install Git and retry."
must cargo "Install Rust toolchain (rustup recommended) and retry."

# Rust components (recommended)
maybe rustfmt "Run: rustup component add rustfmt"
maybe clippy "Run: rustup component add clippy"

# Optional tooling
maybe docker "Needed for docker-compose based dev stacks."
maybe openssl "Needed for scripts/setup/generate-keys.(sh|ps1)."
maybe node "Needed for Svelte frontends under apps/(admin-web|web)."
maybe npm "Needed for Svelte frontends under apps/(admin-web|web)."

# ripgrep is optional; scripts fall back to grep.
maybe rg "Optional (faster scans)."

echo "preflight: OK"
