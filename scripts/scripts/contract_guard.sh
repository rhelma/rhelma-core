#!/usr/bin/env bash
set -euo pipefail
bash "$(dirname "$0")/guards/contract_guard.sh" "$@"
