#!/usr/bin/env bash
set -euo pipefail

# Small helper utilities shared across scripts.

require_cmd() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "ERROR: Required command '$cmd' was not found in PATH." >&2
    return 127
  fi
}

require_rust_toolchain() {
  require_cmd cargo || {
    cat >&2 <<'MSG'
Hint: Install Rust (cargo) and re-run.
- Recommended: rustup (https://rustup.rs)
- Or install via your OS package manager.
MSG
    return 127
  }
}
