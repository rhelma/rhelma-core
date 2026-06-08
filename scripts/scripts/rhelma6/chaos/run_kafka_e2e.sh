#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"

COMPOSE_FILE="scripts/rhelma6/chaos/docker-compose.kafka.yml"

: "${RHELMA_KAFKA_BROKERS:=localhost:9092}"
: "${RHELMA_KAFKA_TOPIC_PREFIX:=rhelma.}"
: "${RHELMA_E2E_KAFKA_TIMEOUT_SEC:=30}"

cleanup() {
  if command -v docker >/dev/null 2>&1; then
    docker compose -f "$COMPOSE_FILE" down -v --remove-orphans >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

echo "[1/3] Starting Kafka (docker compose)…"
docker compose -f "$COMPOSE_FILE" up -d

echo "[2/3] Waiting for broker…"
for i in {1..30}; do
  if (echo > /dev/tcp/127.0.0.1/9092) >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

echo "[3/3] Running Kafka integration tests…"
export RHELMA_KAFKA_BROKERS
export RHELMA_KAFKA_TOPIC_PREFIX
export RHELMA_E2E_KAFKA_TIMEOUT_SEC

cargo test -p e2e-tests --features kafka-integration -- --ignored --nocapture
