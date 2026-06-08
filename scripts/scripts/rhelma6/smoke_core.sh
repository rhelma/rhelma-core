#!/usr/bin/env bash
set -euo pipefail

# Minimal smoke checks for the core control plane.
# Targets:
# - api-gateway
# - ai-orchestrator
# - node-registry
# - kafka brokers (optional)
#
# Usage:
#   RHELMA_SMOKE_API_GATEWAY_URL=http://127.0.0.1:3000 \
#   RHELMA_SMOKE_AI_ORCH_URL=http://127.0.0.1:4000 \
#   RHELMA_SMOKE_NODE_REGISTRY_URL=http://127.0.0.1:8090 \
#   RHELMA_SMOKE_KAFKA_BROKERS="kafka-0:9092,kafka-1:9092" \
#   ./scripts/rhelma6/smoke_core.sh

export RHELMA_SMOKE_SKIP_SEARCH=1
export RHELMA_SMOKE_SKIP_FILE_STORAGE=1
export RHELMA_SMOKE_SKIP_REALTIME=1
export RHELMA_SMOKE_SKIP_LLM_NODE=1

# Keep node-registry flow off by default in prod.
export RHELMA_SMOKE_NODE_REGISTRY_FLOW="${RHELMA_SMOKE_NODE_REGISTRY_FLOW:-0}"

bash scripts/smoke_staging.sh
