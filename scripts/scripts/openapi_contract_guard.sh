#!/usr/bin/env bash
set -euo pipefail
bash "$(dirname "$0")/guards/openapi_contract_guard.sh" "$@"
