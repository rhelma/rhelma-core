#!/usr/bin/env bash
set -euo pipefail
bash "$(dirname "$0")/guards/todo_guard.sh" "$@"
