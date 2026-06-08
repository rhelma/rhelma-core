#!/usr/bin/env bash
set -euo pipefail

# Network partition helper for chaos tests.
#
# Usage:
#   network_partition.sh partition <endpoint-url>
#   network_partition.sh heal      <endpoint-url>
#
# The script locates the container that exposes the given endpoint port and
# disconnects/reconnects it from the Docker network.

ACTION=${1:-}
ENDPOINT=${2:-}

if [[ -z "$ACTION" || -z "$ENDPOINT" ]]; then
  echo "usage: $0 <partition|heal> <endpoint-url>" >&2
  exit 2
fi

PORT=$(echo "$ENDPOINT" | sed -E 's#^https?://[^:/]+:([0-9]+).*#\1#')
if [[ -z "$PORT" || "$PORT" == "$ENDPOINT" ]]; then
  echo "could not parse port from endpoint: $ENDPOINT" >&2
  exit 2
fi

# Find container name by published host port.
CONTAINER=$(docker ps --format '{{.Names}} {{.Ports}}' | awk -v p=":${PORT}->" '$0 ~ p {print $1; exit}')
if [[ -z "$CONTAINER" ]]; then
  # Fallback: sometimes ports may be shown without the 0.0.0.0 prefix.
  CONTAINER=$(docker ps --format '{{.Names}} {{.Ports}}' | awk -v p="${PORT}->" '$0 ~ p {print $1; exit}')
fi

if [[ -z "$CONTAINER" ]]; then
  echo "could not locate container publishing port ${PORT}" >&2
  docker ps --format '{{.Names}} {{.Ports}}' >&2 || true
  exit 3
fi

NETWORK=${RHELMA_CHAOS_DOCKER_NETWORK:-}
if [[ -z "$NETWORK" ]]; then
  # Pick the first network the container is connected to.
  NETWORK=$(docker inspect -f '{{range $k, $v := .NetworkSettings.Networks}}{{printf "%s\n" $k}}{{end}}' "$CONTAINER" | head -n 1)
fi

if [[ -z "$NETWORK" ]]; then
  echo "could not determine docker network for container: $CONTAINER" >&2
  exit 4
fi

case "$ACTION" in
  partition)
    echo "[chaos] partition: disconnecting ${CONTAINER} from ${NETWORK}" >&2
    docker network disconnect -f "$NETWORK" "$CONTAINER"
    ;;
  heal)
    echo "[chaos] heal: reconnecting ${CONTAINER} to ${NETWORK}" >&2
    # If already connected, docker network connect returns non-zero.
    docker network connect "$NETWORK" "$CONTAINER" 2>/dev/null || true
    ;;
  *)
    echo "unknown action: $ACTION (expected: partition|heal)" >&2
    exit 2
    ;;
esac

exit 0
