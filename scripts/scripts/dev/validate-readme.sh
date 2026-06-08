#!/usr/bin/env bash
set -euo pipefail
# validate-readme.sh

REQUIRED_SECTIONS=(
    "## Overview"
    "## Ownership"
    "## Run"
    "## Configuration"
    "## Endpoints"
    "## Observability"
)

for service in apps/*/; do
  readme="$service/README.md"

  if [[ ! -f "$readme" ]]; then
    echo "❌ $service missing: README.md"
    continue
  fi

  for section in "${REQUIRED_SECTIONS[@]}"; do
    if ! grep -q "$section" "$readme"; then
      echo "❌ $service missing: $section"
    fi
  done
done
