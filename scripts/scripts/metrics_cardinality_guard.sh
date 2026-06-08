#!/usr/bin/env bash
set -euo pipefail
bash "$(dirname "$0")/guards/metrics_cardinality_guard.sh" "$@"
