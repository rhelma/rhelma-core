#!/usr/bin/env bash
set -euo pipefail

BROKERS="${RHELMA_KAFKA_BROKERS:-localhost:9092}"
PREFIX="${RHELMA_KAFKA_TOPIC_PREFIX:-rhelma.}"
TOPIC="${1:?topic required (obs.region_health|obs.region_failover)}"
FILE="${2:?json file required}"

REAL_TOPIC="${PREFIX}${TOPIC}"

if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required" >&2
  exit 1
fi

# Use the Kafka image's console producer so host tools are not required.
docker run --rm -i --network host confluentinc/cp-kafka:7.6.1 \
  kafka-console-producer --bootstrap-server "${BROKERS}" --topic "${REAL_TOPIC}" < "${FILE}"

echo "published to ${REAL_TOPIC}"
