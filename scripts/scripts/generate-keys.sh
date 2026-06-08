#!/usr/bin/env bash
set -euo pipefail
bash "$(dirname "$0")/setup/generate-keys.sh" "$@"
