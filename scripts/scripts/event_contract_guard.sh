#!/usr/bin/env bash
set -euo pipefail
bash "$(dirname "$0")/guards/event_contract_guard.sh" "$@"
