#!/usr/bin/env bash
set -euo pipefail
bash "$(dirname "$0")/dev/run-multi-frontend.sh" "$@"
