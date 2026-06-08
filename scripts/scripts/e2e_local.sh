#!/usr/bin/env bash
set -euo pipefail
bash "$(dirname "$0")/e2e/e2e_local.sh" "$@"
