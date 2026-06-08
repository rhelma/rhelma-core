#!/usr/bin/env bash
set -euo pipefail

# Optional OTEL verification gate.
#
# Enable by setting:
#   RHELMA_VERIFY_OTEL=1
#
# Kept separate so local `./scripts/verify.sh` stays fast by default.

if [[ "${RHELMA_VERIFY_OTEL:-0}" != "1" ]]; then
  echo "RHELMA_VERIFY_OTEL is not set to 1; skipping OTEL verification"
  exit 0
fi

echo "Running OTEL verification (rhelma-event-kafka --features otel)"

# Run OTEL-specific tests for Kafka trace/baggage propagation.
cargo test -p rhelma-event-kafka --features otel --tests

echo "OTEL verification OK"
