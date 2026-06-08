#!/usr/bin/env bash
set -euo pipefail

mkdir -p .githooks

git config core.hooksPath .githooks

if [[ -f .githooks/pre-commit ]]; then
  chmod +x .githooks/pre-commit || true
fi

echo "Installed git hooks path: .githooks/"
