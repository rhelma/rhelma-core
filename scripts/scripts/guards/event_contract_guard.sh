#!/usr/bin/env bash
set -euo pipefail

ROOT="${1:-.}"
echo "[event_contract_guard] scanning for unsafe event publishing patterns under: $ROOT"

# Focus on publish-boundary safety (not on how envelopes are constructed).
#
# Hard fail:
#   - Passing an EventEnvelope literal directly into publish() without a finalize_* call.
#
# Soft warning (heuristic):
#   - publish() call without a nearby finalize_* call (may be safe if bus enforces contracts).

ALLOW_PATH_RE='(tests/|examples/|\.md$|migrations/|target/|node_modules/|\.git/)'

fail() { echo "[event_contract_guard] FAIL: $1" >&2; exit 1; }

# Collect only Rust files that actually contain a publish(...) call.
# This keeps the guard fast and CI-friendly.
mapfile -t CAND_FILES < <(
  grep -RIl --exclude-dir=target --exclude-dir=.git --exclude-dir=node_modules     --include='*.rs' -E '\.publish[[:space:]]*\(' "$ROOT" 2>/dev/null || true
)

# 1) Hard fail: publish(EventEnvelope { .. }) without finalize_* on the same line.
HARD=0
for f in "${CAND_FILES[@]}"; do
  [[ "$f" =~ $ALLOW_PATH_RE ]] && continue
  grep -q 'EventEnvelope' "$f" 2>/dev/null || continue

  hits=$(grep -nE '\.publish[[:space:]]*\([[:space:]]*EventEnvelope[[:space:]]*\{' "$f" 2>/dev/null     | grep -Ev '\.finalize_[A-Za-z0-9_]*[[:space:]]*\(' || true)

  if [[ -n "$hits" ]]; then
    echo "----"
    echo "File: $f"
    echo "$hits"
    HARD=1
  fi

done

if [[ "$HARD" -eq 1 ]]; then
  fail "direct publish(EventEnvelope {..}) detected. Build envelope, then call finalize_*() before publish()."
fi

# 2) Soft warnings: publish(...) without nearby finalize_* in the preceding 10 lines.
WARN=0
for f in "${CAND_FILES[@]}"; do
  [[ "$f" =~ $ALLOW_PATH_RE ]] && continue
  grep -q 'EventEnvelope' "$f" 2>/dev/null || continue

  out=$(awk -v file="$f" '
    function buf_has_finalize(   i) {
      for (i=1; i<=10; i++) {
        if (buf[i] ~ /\.finalize_[A-Za-z0-9_]*[[:space:]]*\(/) return 1;
        if (buf[i] ~ /finalize_publish_boundary/) return 1;
        if (buf[i] ~ /publish_with_observability/) return 1;
      }
      return 0;
    }

    {
      for (i=10; i>1; i--) buf[i]=buf[i-1];
      buf[1]=$0;

      if ($0 ~ /\.publish[[:space:]]*\(/) {
        if ($0 ~ /\.finalize_[A-Za-z0-9_]*[[:space:]]*\(/) next;
        if ($0 ~ /finalize_publish_boundary/) next;
        if ($0 ~ /publish_with_observability/) next;

        if (!buf_has_finalize()) {
          printf("[event_contract_guard] WARN %s:%d: %s
", file, NR, $0);
          warn=1;
        }
      }
    }
  ' "$f" 2>/dev/null || true)

  if [[ -n "$out" ]]; then
    echo "$out"
    WARN=1
  fi

done

if [[ "$WARN" -eq 1 ]]; then
  echo "[event_contract_guard] NOTE: some publish() calls were found without a nearby finalize_* (heuristic warnings)."
  echo "[event_contract_guard] If safe (e.g., bus enforces contracts), consider switching to finalize_* before publish() or publish_with_observability()."
fi

echo "[event_contract_guard] OK"
