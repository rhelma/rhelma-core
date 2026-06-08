#!/usr/bin/env bash
set -euo pipefail
ROOT="${1:-.}"
python3 scripts/guards/openapi_drift_guard.py "$ROOT" --service all
