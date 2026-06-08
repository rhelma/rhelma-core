#!/usr/bin/env bash
set -euo pipefail

# Adds/removes latency using Linux netem inside a docker container.
#
# Usage:
#   netem_latency.sh add <service_or_container> <delay_ms> [jitter_ms]
#   netem_latency.sh remove <service_or_container>
#
# Notes:
# - Container must run with NET_ADMIN capability.
# - For the Rhelma chaos compose bundle, services are named like `node-registry-2`.

action=${1:-}
target=${2:-}

if [[ -z "$action" || -z "$target" ]]; then
  echo "usage: $0 add|remove <service_or_container> [delay_ms] [jitter_ms]" >&2
  exit 2
fi

delay_ms=${3:-}
jitter_ms=${4:-0}

resolve_container() {
  local t=${1:?target}
  # If it's already a container id/name, this will work.
  if docker inspect "$t" >/dev/null 2>&1; then
    echo "$t"
    return 0
  fi

  # Try to resolve docker-compose service name.
  local project=${COMPOSE_PROJECT_NAME:-rhelma6chaos}
  local cid
  cid=$(docker ps --filter "name=${project}-${t}-" --format '{{.ID}}' | head -n 1 || true)
  if [[ -n "$cid" ]]; then
    echo "$cid"
    return 0
  fi

  # Try plain service name match.
  cid=$(docker ps --filter "name=${t}" --format '{{.ID}}' | head -n 1 || true)
  if [[ -n "$cid" ]]; then
    echo "$cid"
    return 0
  fi

  echo "could not resolve container for target: $t" >&2
  return 1
}

cid=$(resolve_container "$target")

ensure_tc() {
  docker exec "$cid" bash -lc 'command -v tc >/dev/null 2>&1 || (apt-get update -y >/dev/null && apt-get install -y iproute2 >/dev/null)'
}

if [[ "$action" == "add" ]]; then
  if [[ -z "$delay_ms" ]]; then
    echo "usage: $0 add <service_or_container> <delay_ms> [jitter_ms]" >&2
    exit 2
  fi
  ensure_tc
  # Use replace to be idempotent.
  docker exec "$cid" bash -lc "tc qdisc replace dev eth0 root netem delay ${delay_ms}ms ${jitter_ms}ms"
  echo "[chaos] netem latency enabled on $target ($cid): delay=${delay_ms}ms jitter=${jitter_ms}ms" >&2
  exit 0
fi

if [[ "$action" == "remove" ]]; then
  ensure_tc
  docker exec "$cid" bash -lc 'tc qdisc del dev eth0 root 2>/dev/null || true'
  echo "[chaos] netem latency removed on $target ($cid)" >&2
  exit 0
fi

echo "unknown action: $action (expected add/remove)" >&2
exit 2
