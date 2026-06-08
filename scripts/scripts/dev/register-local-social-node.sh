#!/usr/bin/env bash
# scripts/dev/register-local-social-node.sh
# Registers social-service as a node for realm=central in control-service.

set -euo pipefail

CONTROL_URL="${RHELMA_CONTROL_SERVICE_URL:-http://127.0.0.1:8086}"
SOCIAL_URL="${RHELMA_SOCIAL_SERVICE_URL:-http://127.0.0.1:8085}"
REGION="${RHELMA_REGION:-local}"
TOKEN="${RHELMA_CONTROL_NODE_REGISTRATION_TOKEN:-dev-node-token}"
VERSION="${RHELMA_SERVICE_VERSION:-0.0.0-dev}"

curl -fsS -X POST "${CONTROL_URL}/v1/nodes/register" \
  -H "content-type: application/json" \
  -H "x-control-node-registration-token: ${TOKEN}" \
  -d "{
    \"name\": \"local-social\",
    \"region\": \"${REGION}\",
    \"public_base_url\": \"${SOCIAL_URL}\",
    \"realm_slug\": \"central\",
    \"capabilities\": { \"social-service\": true },
    \"version\": \"${VERSION}\"
  }"
