#!/usr/bin/env bash
set -euo pipefail

# Developer bootstrap for first-time setup.
#
# What it does:
# - Runs preflight (non-strict)
# - Creates .env from .env.example (if missing)
# - Generates local RSA keys (if missing)
#
# Usage:
#   bash scripts/setup/bootstrap.sh

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}" )/../.." && pwd)"
cd "$ROOT_DIR"

echo "== Rhelma bootstrap =="

bash scripts/setup/preflight.sh || true

if [[ ! -f ".env" ]]; then
  if [[ -f ".env.example" ]]; then
    cp ".env.example" ".env"
    echo "✅ Created .env from .env.example"
    echo "⚠️  Review .env and adjust secrets/ports as needed."
  else
    echo "⚠️  .env.example not found; skipping .env creation."
  fi
else
  echo "✅ .env already exists"
fi

if [[ ! -f "keys/private.pem" || ! -f "keys/public.pem" ]]; then
  echo "Generating RSA keys under ./keys ..."
  bash scripts/setup/generate-keys.sh "./keys" || true
else
  echo "✅ keys/private.pem + keys/public.pem already exist"
fi

echo ""
echo "Next steps:"
echo "- Run full verification:   bash scripts/verify_all.sh"
echo "- Start local world stack: bash scripts/run-world.sh"
echo "- Run local smoke checks:  bash scripts/smoke_local.sh"
