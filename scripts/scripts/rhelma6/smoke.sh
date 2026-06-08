#!/usr/bin/env bash
set -euo pipefail

# Rhelma 6 Phase 41 Smoke Test
# Usage:
#   RHELMA_SMOKE_API_GATEWAY_URL=http://127.0.0.1:3000 \
#   RHELMA_SMOKE_AI_ORCH_URL=http://127.0.0.1:4000 \
#   RHELMA_SMOKE_NODE_REGISTRY_URL=http://127.0.0.1:8090 \
#   RHELMA_SMOKE_SECURITY_GOV_URL=http://127.0.0.1:8091 \
#   RHELMA_SMOKE_GOSSIP_URL=http://127.0.0.1:8092 \
#   RHELMA_SMOKE_MNI_RAG_URL=http://127.0.0.1:8096 \
#   ./scripts/rhelma6/smoke.sh

TIMEOUT_SEC="${RHELMA_SMOKE_TIMEOUT_SEC:-2}"

KAFKA_BROKERS="${RHELMA_SMOKE_KAFKA_BROKERS:-${RHELMA_KAFKA_BROKERS:-}}"

curl_t() {
  local url="$1"
  curl -fsS --max-time "$TIMEOUT_SEC" "$url" >/dev/null
}

must_ok() {
  local name="$1"
  local url="$2"
  echo "[smoke] ${name}: ${url}"
  curl_t "$url"
}

must_ok "api-gateway health" "${RHELMA_SMOKE_API_GATEWAY_URL:-http://127.0.0.1:3000}/healthz"
must_ok "ai-orchestrator health" "${RHELMA_SMOKE_AI_ORCH_URL:-http://127.0.0.1:4000}/healthz"
must_ok "node-registry health" "${RHELMA_SMOKE_NODE_REGISTRY_URL:-http://127.0.0.1:8090}/healthz"
must_ok "security-governance health" "${RHELMA_SMOKE_SECURITY_GOV_URL:-http://127.0.0.1:8091}/healthz"
must_ok "gossip-discovery peers" "${RHELMA_SMOKE_GOSSIP_URL:-http://127.0.0.1:8092}/v1/peers"
must_ok "mni-rag health" "${RHELMA_SMOKE_MNI_RAG_URL:-http://127.0.0.1:8096}/healthz"
must_ok "region-health-aggregator health" "${RHELMA_SMOKE_RHA_URL:-http://127.0.0.1:8097}/healthz"

if [[ -n "$KAFKA_BROKERS" ]]; then
  if command -v python3 >/dev/null 2>&1; then
    echo "[smoke] kafka brokers: tcp connect ($KAFKA_BROKERS)"
    TIMEOUT_SEC_LOCAL="$TIMEOUT_SEC" KAFKA_BROKERS_LOCAL="$KAFKA_BROKERS" python3 - <<'PY'
import os, socket
timeout=float(os.environ.get('TIMEOUT_SEC_LOCAL','2'))
brokers=os.environ.get('KAFKA_BROKERS_LOCAL','').strip()

def parse(b: str):
  b=b.strip()
  if not b:
    return None
  if ':' in b:
    host, port = b.rsplit(':', 1)
    try:
      return host, int(port)
    except Exception:
      return host, 9092
  return b, 9092

for raw in [p for p in brokers.split(',') if p.strip()]:
  hp=parse(raw)
  if not hp:
    continue
  host, port = hp
  s=socket.socket(socket.AF_INET, socket.SOCK_STREAM)
  s.settimeout(timeout)
  try:
    s.connect((host, port))
  except Exception as e:
    raise SystemExit(f"kafka check failed: {host}:{port} ({e})")
  finally:
    try:
      s.close()
    except Exception:
      pass
print('kafka check: OK')
PY
  else
    echo "[smoke] kafka brokers: python3 not found; skipping" >&2
  fi
fi

echo "[smoke] OK"
