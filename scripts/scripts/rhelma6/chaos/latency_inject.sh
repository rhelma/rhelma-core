#!/usr/bin/env bash
set -euo pipefail

# Placeholder for latency injection.
# In prod, use tc/netem or a service-mesh fault injection.
# Usage: ./latency_inject.sh <iface> <ms>

IFACE=${1:-eth0}
MS=${2:-200}

echo "[placeholder] Inject latency ${MS}ms on ${IFACE}. Implement using: tc qdisc add dev ... netem delay ..."
