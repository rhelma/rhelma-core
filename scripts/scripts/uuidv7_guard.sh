#!/usr/bin/env bash
set -euo pipefail
bash "$(dirname "$0")/guards/uuidv7_guard.sh" "$@"
