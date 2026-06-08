#!/usr/bin/env bash
set -euo pipefail
# Simulate federation lag by blocking outgoing traffic to a peer for a short window.
# Usage: PEER_HOST=10.0.0.2 DURATION=60 ./federation_lag_sim.sh
PEER_HOST="${PEER_HOST:-10.0.0.2}"
DURATION="${DURATION:-60}"
echo "Blocking traffic to ${PEER_HOST} for ${DURATION}s (requires sudo)..."
sudo iptables -A OUTPUT -d "${PEER_HOST}" -j DROP
sleep "${DURATION}"
sudo iptables -D OUTPUT -d "${PEER_HOST}" -j DROP
echo "Unblocked."
