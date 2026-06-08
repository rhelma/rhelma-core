#!/usr/bin/env bash
set -euo pipefail
repo_root="${1:-.}"
if command -v python3 >/dev/null 2>&1; then
  python3 "$(dirname "$0")/outbound_http_context_guard.py" "$repo_root"
elif command -v python >/dev/null 2>&1; then
  python "$(dirname "$0")/outbound_http_context_guard.py" "$repo_root"
else
  echo "[outbound_http_context_guard] SKIP: python/python3 not found"
  exit 0
fi
