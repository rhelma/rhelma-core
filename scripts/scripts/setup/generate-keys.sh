#!/usr/bin/env bash
set -euo pipefail

# Generates local RSA keys for dev.
# Usage:
#   bash scripts/setup/generate-keys.sh            # writes to ./keys
#   bash scripts/setup/generate-keys.sh ./secrets  # writes to custom dir

KEYS_DIR="${1:-./keys}"

if ! command -v openssl >/dev/null 2>&1; then
  echo "❌ openssl not found. Install OpenSSL and retry." >&2
  exit 127
fi

mkdir -p "$KEYS_DIR"

PRIV="$KEYS_DIR/private.pem"
PUB="$KEYS_DIR/public.pem"

if [[ -f "$PRIV" || -f "$PUB" ]]; then
  echo "⚠️  Keys already exist in '$KEYS_DIR'. Refusing to overwrite." >&2
  echo "    Remove '$PRIV'/'$PUB' and re-run if you want to regenerate." >&2
  exit 2
fi

openssl genrsa -out "$PRIV" 4096 >/dev/null 2>&1
openssl rsa -in "$PRIV" -pubout -out "$PUB" >/dev/null 2>&1

echo "✅ RSA keys generated:"
echo "  - $PRIV"
echo "  - $PUB"
echo "🔒 Keep keys out of git (keys/ is in .gitignore)."