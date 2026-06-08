#!/usr/bin/env bash
set -euo pipefail
bash "$(dirname "$0")/guards/env_contract_guard.sh" "$@"
