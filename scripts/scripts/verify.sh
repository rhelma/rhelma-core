#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/_lib.sh"
require_rust_toolchain

# --- Resource tuning --------------------------------------------------------
#
# By default, `cargo` will parallelize compilation and tests aggressively, which
# can overwhelm laptops / low-RAM machines.
#
# You can override the defaults via:
#   - RHELMA_VERIFY_JOBS
#   - RHELMA_VERIFY_TEST_THREADS
#   - RHELMA_VERIFY_LOW_RESOURCE=1   (forces jobs=1, threads=1)
#
# Or by setting the standard Cargo/Rust vars yourself:
#   - CARGO_BUILD_JOBS
#   - RUST_TEST_THREADS

detect_cores() {
  if command -v nproc >/dev/null 2>&1; then
    nproc
  elif command -v getconf >/dev/null 2>&1; then
    getconf _NPROCESSORS_ONLN
  elif command -v sysctl >/dev/null 2>&1; then
    sysctl -n hw.ncpu 2>/dev/null || echo 1
  else
    echo 1
  fi
}

detect_mem_gb() {
  # Best-effort; returns empty string if unknown.
  if [[ -r /proc/meminfo ]]; then
    awk '/MemTotal/ { printf("%.0f", $2/1024/1024) }' /proc/meminfo
  elif command -v sysctl >/dev/null 2>&1; then
    # macOS
    local bytes
    bytes="$(sysctl -n hw.memsize 2>/dev/null || true)"
    if [[ -n "$bytes" ]]; then
      awk -v b="$bytes" 'BEGIN { printf("%.0f", b/1024/1024/1024) }'
    fi
  fi
}

resolve_verify_tuning() {
  local cores mem_gb jobs threads
  cores="$(detect_cores)"
  mem_gb="$(detect_mem_gb || true)"

  local ci="${CI:-}"

  if [[ "${RHELMA_VERIFY_LOW_RESOURCE:-}" == "1" ]]; then
    jobs=1
    threads=1
  else
    jobs="${RHELMA_VERIFY_JOBS:-${CARGO_BUILD_JOBS:-}}"
    if [[ -z "$jobs" ]]; then
      if [[ -n "$ci" ]]; then
        # In CI we prefer speed over interactivity.
        jobs="$cores"
        if [[ "$jobs" -gt 16 ]]; then jobs=16; fi
      elif [[ -n "$mem_gb" && "$mem_gb" -lt 8 ]]; then
        jobs=1
      elif [[ -n "$mem_gb" && "$mem_gb" -lt 16 ]]; then
        jobs=2
      else
        # leave one core free, cap at 4 to avoid surprise fan+RAM spikes
        if [[ "$cores" -gt 1 ]]; then
          jobs=$((cores - 1))
        else
          jobs=1
        fi
        if [[ "$jobs" -gt 4 ]]; then jobs=4; fi
      fi
    fi

    threads="${RHELMA_VERIFY_TEST_THREADS:-${RUST_TEST_THREADS:-}}"
    if [[ -z "$threads" ]]; then
      if [[ -n "$ci" ]]; then
        threads="$jobs"
        if [[ "$threads" -gt 16 ]]; then threads=16; fi
      elif [[ -n "$mem_gb" && "$mem_gb" -lt 8 ]]; then
        threads=1
      else
        threads=2
      fi
    fi
  fi

  # sanitize
  if [[ "$jobs" -lt 1 ]]; then jobs=1; fi
  if [[ "$threads" -lt 1 ]]; then threads=1; fi

  # Apply defaults if the caller didn't already pin them.
  export CARGO_BUILD_JOBS="$jobs"
  export RUST_TEST_THREADS="$threads"
  export RAYON_NUM_THREADS="${RAYON_NUM_THREADS:-$threads}"

  local mem_msg="${mem_gb:-unknown}GB"
  echo "verify: tuning -> CARGO_BUILD_JOBS=$jobs, RUST_TEST_THREADS=$threads (CPU=$cores, RAM=$mem_msg)"
}

resolve_verify_tuning

cargo fmt --all -- --check
cargo clippy -j "${CARGO_BUILD_JOBS}" --workspace --all-targets -- -D warnings
cargo test -j "${CARGO_BUILD_JOBS}" --workspace -- --test-threads "${RUST_TEST_THREADS}"

# Optional OTEL propagation/regression tests (opt-in via RHELMA_VERIFY_OTEL=1)
if [[ -f "scripts/verify_otel.sh" ]]; then
  bash scripts/verify_otel.sh
fi

# Contract & env/event anti-drift gates (best-effort)
if [[ -f "scripts/contract_guard.sh" ]]; then
  bash scripts/contract_guard.sh
fi
if [[ -f "scripts/env_contract_guard.sh" ]]; then
  bash scripts/env_contract_guard.sh
fi
if [[ -f "scripts/event_contract_guard.sh" ]]; then
  bash scripts/event_contract_guard.sh
fi

if [[ -f "scripts/header_contract_guard.sh" ]]; then
  bash scripts/header_contract_guard.sh
fi

# OpenAPI contract checks (best-effort)
if [[ -f "scripts/openapi_contract_guard.sh" ]]; then
  bash scripts/openapi_contract_guard.sh
fi

# OpenAPI drift/examples gates (best-effort)
if [[ -f "scripts/openapi_drift_guard.sh" ]]; then
  bash scripts/openapi_drift_guard.sh .
fi
if [[ -f "scripts/openapi_examples_guard.sh" ]]; then
  bash scripts/openapi_examples_guard.sh .
fi

# Observability verification (best-effort; skips missing tests/scripts)
if [[ -f "scripts/verify_observability.sh" ]]; then
  bash scripts/verify_observability.sh .
fi

if command -v cargo-deny >/dev/null 2>&1; then
  cargo deny check
else
  echo "cargo-deny not installed; skipping (install with: cargo install --locked cargo-deny)"
fi

# Completeness report (non-blocking by default; gate with RHELMA_VERIFY_COMPLETENESS=1)
if [[ -f "scripts/dev/completeness-report.sh" ]]; then
  bash scripts/dev/completeness-report.sh
fi
