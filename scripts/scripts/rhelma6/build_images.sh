#!/usr/bin/env bash
set -euo pipefail

# Build Rhelma6 production images locally.
# Usage:
#   bash scripts/rhelma6/build_images.sh v6.0.0-dev

TAG="${1:-dev}"
REGISTRY="${RHELMA_DOCKER_REGISTRY:-}" # e.g. ghcr.io/ORG/REPO

img() {
  local name="$1"
  local dockerfile="$2"
  local target="${REGISTRY:+$REGISTRY/}$name:$TAG"
  echo "==> Building $target"
  docker build -f "$dockerfile" -t "$target" .
}

img node-registry deploy/rhelma6/docker/Dockerfile.node-registry
img bridge-adapter deploy/rhelma6/docker/Dockerfile.bridge-adapter
img rhelma-node deploy/rhelma6/docker/Dockerfile.rhelma-node

echo "Done."
