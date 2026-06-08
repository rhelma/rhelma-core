#!/usr/bin/env bash
set -euo pipefail
bash "$(dirname "$0")/guards/scrapeability_guard.sh" "$@"
