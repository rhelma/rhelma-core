#!/usr/bin/env bash
set -euo pipefail

if [ ! -d "apps/admin-web" ]; then
  echo "apps/admin-web not found" >&2
  exit 1
fi

cd apps/admin-web

if command -v pnpm >/dev/null 2>&1; then
  PM=pnpm
elif command -v npm >/dev/null 2>&1; then
  PM=npm
elif command -v yarn >/dev/null 2>&1; then
  PM=yarn
else
  echo "Install pnpm/npm/yarn to run admin-web" >&2
  exit 1
fi

$PM install
$PM run dev
