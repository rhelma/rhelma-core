#!/usr/bin/env bash
set -euo pipefail
bash "$(dirname "$0")/smoke/smoke_local.sh" "$@"
