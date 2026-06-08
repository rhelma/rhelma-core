#!/usr/bin/env bash
set -euo pipefail

# Restart a docker compose service (game day helper)
#
# Designed for use with `crates/e2e-tests` chaos tests.
#
# Environment:
#   RHELMA_CHAOS_COMPOSE_FILE  Path to compose file (defaults to docker-compose.dev.yml)
#
# Usage:
#   ./restart_service.sh restart <service>

ACTION=${1:?action}
SERVICE=${2:?service}

if [[ "$ACTION" != "restart" ]]; then
  echo "unsupported action: $ACTION" >&2
  exit 2
fi

COMPOSE_FILE=${RHELMA_CHAOS_COMPOSE_FILE:-docker-compose.dev.yml}

echo "[chaos] restarting service '$SERVICE' using compose file '$COMPOSE_FILE'" >&2

# Prefer docker compose restart for minimal disruption.
if docker compose -f "$COMPOSE_FILE" restart "$SERVICE"; then
  exit 0
fi

# Fallback: stop + up (handles older compose variants and some edge cases).
docker compose -f "$COMPOSE_FILE" stop "$SERVICE"
docker compose -f "$COMPOSE_FILE" up -d "$SERVICE"
