#!/usr/bin/env bash
set -euo pipefail
bash "$(dirname "$0")/guards/header_contract_guard.sh" "$@"
