#!/usr/bin/env bash
set -euo pipefail

# Generate an Ed25519 keypair in **DER** format and print base64 values
# suitable for rhelma-auth env vars:
#   RHELMA_AUTH_JWT_PRIVATE_KEY_B64
#   RHELMA_AUTH_JWT_PUBLIC_KEY_B64

tmpdir="${TMPDIR:-/tmp}/rhelma_ed25519_$$"
mkdir -p "$tmpdir"
trap 'rm -rf "$tmpdir"' EXIT

priv_der="$tmpdir/ed25519_private.der"
pub_der="$tmpdir/ed25519_public.der"

openssl genpkey -algorithm ED25519 -outform DER -out "$priv_der" >/dev/null 2>&1
openssl pkey -in "$priv_der" -inform DER -pubout -outform DER -out "$pub_der" >/dev/null 2>&1

priv_b64=$(base64 -w0 "$priv_der")
pub_b64=$(base64 -w0 "$pub_der")

echo "RHELMA_AUTH_JWT_PRIVATE_KEY_B64=$priv_b64"
echo "RHELMA_AUTH_JWT_PUBLIC_KEY_B64=$pub_b64"
