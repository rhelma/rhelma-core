#!/usr/bin/env bash
set -euo pipefail

# Kill a docker compose service (game day helper)
# Usage: ./kill_service.sh <compose_file> <service>

COMPOSE_FILE=${1:?compose_file}
SERVICE=${2:?service}

docker compose -f "$COMPOSE_FILE" stop "$SERVICE"
