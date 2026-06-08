#!/usr/bin/env bash
set -euo pipefail

# Metrics cardinality guard (Rhelma v5.2)
#
# Fails if any Rust code uses a raw HTTP path (e.g. `req.uri().path()`) as the
# metrics endpoint label. This is a best-effort static scan.

repo_root="${1:-.}"

# Find Rust files that record HTTP metrics.
files=$(grep -R --line-number --include='*.rs' --exclude-dir=target --exclude-dir=.git   -e 'record_http_request' "$repo_root" 2>/dev/null | cut -d: -f1 | sort -u || true)

fail=0

for f in $files; do
  # 1) Direct raw-path usage as endpoint argument.
  if grep -nE 'record_http_request(_with_bytes|_with_labels)?\([^,]*,[^,]*(req\.)?uri\(\)\.path\(\)' "$f" >/dev/null; then
    echo "[FAIL] raw uri().path() used as metrics endpoint label: $f"
    grep -nE 'record_http_request(_with_bytes|_with_labels)?\([^,]*,[^,]*(req\.)?uri\(\)\.path\(\)' "$f" || true
    fail=1
  fi

  # 2) Common pattern: `let path = req.uri().path(); ... record_http_request(..., path, ...)`
  # NOTE: Use POSIX character classes for portability. (grep -E does NOT support \\s/\\b)
  if grep -nE 'let[[:space:]]+path[[:space:]]*=[[:space:]]*.*uri\(\)\.path\(\)' "$f" >/dev/null \
      && grep -nE 'record_http_request(_with_bytes|_with_labels)?\([^,]*,[^,]*([^[:alnum:]_]|^)path([^[:alnum:]_]|$)' "$f" >/dev/null; then
    echo "[FAIL] variable `path` derived from uri().path() used as metrics endpoint label: $f"
    fail=1
  fi
done

if [[ $fail -ne 0 ]]; then
  echo "[metrics_cardinality_guard] FAILED"
  exit 1
fi

echo "[metrics_cardinality_guard] OK"
