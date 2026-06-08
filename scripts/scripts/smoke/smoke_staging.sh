#!/usr/bin/env bash
set -euo pipefail

TIMEOUT_SEC="${RHELMA_SMOKE_TIMEOUT_SEC:-2}"

API_GATEWAY_URL="${RHELMA_SMOKE_API_GATEWAY_URL:-http://127.0.0.1:3000}"
AI_ORCH_URL="${RHELMA_SMOKE_AI_ORCH_URL:-http://127.0.0.1:4000}"
SEARCH_URL="${RHELMA_SMOKE_SEARCH_URL:-http://127.0.0.1:8082}"
FILE_STORAGE_URL="${RHELMA_SMOKE_FILE_STORAGE_URL:-http://127.0.0.1:3005}"
REALTIME_URL="${RHELMA_SMOKE_REALTIME_URL:-http://127.0.0.1:9000}"
NODE_REGISTRY_URL="${RHELMA_SMOKE_NODE_REGISTRY_URL:-http://127.0.0.1:8090}"
LLM_NODE_URL="${RHELMA_SMOKE_LLM_NODE_URL:-http://127.0.0.1:8088}"

# Optional: multi-region region-health-aggregator checks.
REGION_AGGREGATOR_URL="${RHELMA_SMOKE_REGION_AGGREGATOR_URL:-}"

# Optional: Kafka connectivity check (TCP connect) for critical dependencies.
KAFKA_BROKERS="${RHELMA_SMOKE_KAFKA_BROKERS:-${RHELMA_KAFKA_BROKERS:-}}"

SKIP_API_GATEWAY="${RHELMA_SMOKE_SKIP_API_GATEWAY:-0}"
SKIP_AI_ORCH="${RHELMA_SMOKE_SKIP_AI_ORCH:-0}"
SKIP_SEARCH="${RHELMA_SMOKE_SKIP_SEARCH:-0}"
SKIP_FILE_STORAGE="${RHELMA_SMOKE_SKIP_FILE_STORAGE:-0}"
SKIP_REALTIME="${RHELMA_SMOKE_SKIP_REALTIME:-0}"
SKIP_NODE_REGISTRY="${RHELMA_SMOKE_SKIP_NODE_REGISTRY:-0}"
SKIP_LLM_NODE="${RHELMA_SMOKE_SKIP_LLM_NODE:-0}"
SKIP_KAFKA="${RHELMA_SMOKE_SKIP_KAFKA:-0}"
SKIP_REGION_AGGREGATOR="${RHELMA_SMOKE_SKIP_REGION_AGGREGATOR:-0}"

need() {
  command -v "$1" >/dev/null 2>&1 || { echo "smoke_staging: missing dependency: $1"; exit 2; }
}

need curl

kafka_check() {
  if [[ -z "$KAFKA_BROKERS" ]]; then
    echo "  (kafka check: RHELMA_SMOKE_KAFKA_BROKERS not set; skipping)"
    return 0
  fi

  if ! command -v python3 >/dev/null 2>&1; then
    echo "  (kafka check: python3 not found; skipping)" >&2
    return 0
  fi

  echo "- kafka brokers: tcp connect ($KAFKA_BROKERS)"
  TIMEOUT_SEC_LOCAL="$TIMEOUT_SEC" KAFKA_BROKERS_LOCAL="$KAFKA_BROKERS" python3 - <<'PY'
import os, socket

timeout = float(os.environ.get("TIMEOUT_SEC_LOCAL", "2"))
brokers = os.environ.get("KAFKA_BROKERS_LOCAL", "").strip()

def parse(b: str):
  b = b.strip()
  if not b:
    return None
  if ":" in b:
    host, port = b.rsplit(":", 1)
    try:
      return host, int(port)
    except Exception:
      return host, 9092
  return b, 9092

for raw in [p for p in brokers.split(",") if p.strip()]:
  hp = parse(raw)
  if not hp:
    continue
  host, port = hp
  s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
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
print("kafka check: OK")
PY
}

check() {
  local name="$1"; local base="$2"; local path="$3";
  local url="${base%/}$path"
  echo "- $name: GET $url"
  curl -fsS --max-time "$TIMEOUT_SEC" "$url" >/dev/null
}

region_aggregator_check() {
  if [[ -z "$REGION_AGGREGATOR_URL" ]]; then
    echo "  (region-health-aggregator: RHELMA_SMOKE_REGION_AGGREGATOR_URL not set; skipping)"
    return 0
  fi

  check "region-health-aggregator live" "$REGION_AGGREGATOR_URL" "/healthz"
  check "region-health-aggregator regions" "$REGION_AGGREGATOR_URL" "/v1/regions/health"
}

json_get() {
  # Reads JSON from stdin and prints the field.
  local field="$1"
  python3 - <<PY
import json,sys
try:
  obj=json.load(sys.stdin)
  v=obj.get("$field","")
  print(v if v is not None else "")
except Exception:
  print("")
PY
}

pow_solve() {
  local nonce_hex="$1"; local difficulty_bits="$2"; local max_iters="$3"
  python3 - "$nonce_hex" "$difficulty_bits" "$max_iters" <<'PY'
import hashlib,sys
nonce_hex=sys.argv[1]
difficulty=int(sys.argv[2])
max_iters=int(sys.argv[3])
nonce=bytes.fromhex(nonce_hex)
if len(nonce)!=32:
  print("")
  raise SystemExit(0)

def leading_zero_bits(d: bytes) -> int:
  z=0
  for b in d:
    if b==0:
      z += 8
      continue
    z += (8 - b.bit_length())
    break
  return z

for i in range(max_iters):
  sol=i.to_bytes(8,'little',signed=False)
  h=hashlib.sha256(nonce+sol).digest()
  if leading_zero_bits(h) >= difficulty:
    print(sol.hex())
    raise SystemExit(0)

print("")
PY
}

node_registry_flow() {
  if ! command -v python3 >/dev/null 2>&1; then
    echo "  (node-registry flow: python3 not found; skipping)" >&2
    return 0
  fi

  local node_id issued_at
  node_id=$(python3 - <<'PY'
import os
print(os.urandom(32).hex())
PY
)
  issued_at=$(python3 - <<'PY'
from datetime import datetime, timezone
print(datetime.now(timezone.utc).isoformat().replace('+00:00','Z'))
PY
)

  local admission_json=""

  # If PoW is enabled, the challenge endpoint will exist.
  local challenge_url="${NODE_REGISTRY_URL%/}/v1/admission/challenge?node_id=${node_id}"
  if challenge_resp=$(curl -fsS --max-time "$TIMEOUT_SEC" "$challenge_url" 2>/dev/null); then
    local nonce_hex difficulty_bits
    nonce_hex=$(echo "$challenge_resp" | json_get nonce_hex)
    difficulty_bits=$(echo "$challenge_resp" | json_get difficulty_bits)
    if [[ -n "$nonce_hex" && -n "$difficulty_bits" ]]; then
      local max_iters="${RHELMA_SMOKE_POW_MAX_ITERS:-2000000}"
      echo "- node-registry admission: solving PoW (difficulty_bits=$difficulty_bits, max_iters=$max_iters)"
      local solution_hex
      solution_hex=$(pow_solve "$nonce_hex" "$difficulty_bits" "$max_iters")
      if [[ -z "$solution_hex" ]]; then
        echo "  (node-registry flow: PoW solve failed within max_iters=$max_iters; skipping flow)" >&2
        return 0
      fi
      admission_json=$(python3 - <<PY
import json
print(json.dumps({"nonce_hex":"$nonce_hex","solution_hex":"$solution_hex","difficulty_bits":int($difficulty_bits)}))
PY
)
    fi
  fi

  # Build register request JSON.
  NODE_ID="$node_id" ISSUED_AT="$issued_at" ADMISSION_JSON="$admission_json" \
    python3 - <<'PY' >"${TMPDIR:-/tmp}/rhelma_node_register.json"
import json,os
node_id=os.environ['NODE_ID']
issued_at=os.environ['ISSUED_AT']
adm=os.environ.get('ADMISSION_JSON','').strip()
manifest={
  "node_id": node_id,
  "public_key_hex": node_id,
  "display_name": "smoke-node",
  "region": "local",
  "allowed_residencies": ["local"],
  "capabilities": ["smoke"],
  "endpoints": {"control_url": None, "data_url": None},
  "version": "0.0.0-smoke",
  "issued_at": issued_at,
}
req={"manifest": manifest}
if adm:
  req["admission"]=json.loads(adm)
print(json.dumps(req))
PY

  echo "- node-registry register: POST ${NODE_REGISTRY_URL%/}/v1/nodes/register (node_id=$node_id)"
  curl -fsS --max-time "$TIMEOUT_SEC" \
    -H 'content-type: application/json' \
    -d @"${TMPDIR:-/tmp}/rhelma_node_register.json" \
    "${NODE_REGISTRY_URL%/}/v1/nodes/register" >/dev/null

  # Heartbeat.
  NODE_ID="$node_id" \
    python3 - <<'PY' >"${TMPDIR:-/tmp}/rhelma_node_heartbeat.json"
import json,os
from datetime import datetime, timezone
node_id=os.environ['NODE_ID']
obs=datetime.now(timezone.utc).isoformat().replace('+00:00','Z')
print(json.dumps({"node_id":node_id,"observed_at":obs,"load_avg_1m":None,"free_mem_mb":None,"notes":"smoke"}))
PY

  echo "- node-registry heartbeat: POST ${NODE_REGISTRY_URL%/}/v1/nodes/heartbeat"
  curl -fsS --max-time "$TIMEOUT_SEC" \
    -H 'content-type: application/json' \
    -d @"${TMPDIR:-/tmp}/rhelma_node_heartbeat.json" \
    "${NODE_REGISTRY_URL%/}/v1/nodes/heartbeat" >/dev/null

  # Discover (best-effort).
  if ! curl -fsS --max-time "$TIMEOUT_SEC" "${NODE_REGISTRY_URL%/}/v1/nodes/discover?capability=smoke&limit=1" >/dev/null 2>&1; then
    echo "  (node-registry discover: not reachable; skipping)"
  else
    echo "- node-registry discover: OK"
  fi
}

echo -e "\nRHELMA smoke test (staging-ready endpoints)"
echo "Timeout: ${TIMEOUT_SEC}s"

# api-gateway
if [[ "$SKIP_API_GATEWAY" != "1" ]]; then
  check "api-gateway live" "$API_GATEWAY_URL" "/health/"
  check "api-gateway ready" "$API_GATEWAY_URL" "/health/ready"
  check "api-gateway metrics" "$API_GATEWAY_URL" "/admin/metrics"
  check "api-gateway auth health" "$API_GATEWAY_URL" "/auth/health"

  # Optional: auth flow (requires Postgres migrations + working DB config)
  if [[ "${RHELMA_SMOKE_AUTH_FLOW:-0}" == "1" ]]; then
    TENANT_ID="${RHELMA_SMOKE_TENANT_ID:-local}"
    EMAIL="smoke_${RANDOM}_$(date +%s)@example.local"
    PASS="${RHELMA_SMOKE_PASSWORD:-SmokeTestPassw0rd!}"
    echo "- api-gateway auth flow: register/login/refresh (tenant='$TENANT_ID' email='$EMAIL')"

    register_json=$(curl -fsS --max-time "$TIMEOUT_SEC" \
      -H 'content-type: application/json' \
      -H "x-tenant-id: $TENANT_ID" \
      -d "{\"email\":\"$EMAIL\",\"password\":\"$PASS\",\"name\":\"smoke\"}" \
      "${API_GATEWAY_URL%/}/auth/register")

    refresh_token=$(echo "$register_json" | json_get refresh_token)
    if [[ -z "$refresh_token" ]]; then
      echo "  (auth flow: could not extract refresh_token; response was: $register_json)" >&2
      exit 1
    fi

    # login
    curl -fsS --max-time "$TIMEOUT_SEC" \
      -H 'content-type: application/json' \
      -H "x-tenant-id: $TENANT_ID" \
      -d "{\"email\":\"$EMAIL\",\"password\":\"$PASS\"}" \
      "${API_GATEWAY_URL%/}/auth/login" >/dev/null

    # refresh
    curl -fsS --max-time "$TIMEOUT_SEC" \
      -H 'content-type: application/json' \
      -H "x-tenant-id: $TENANT_ID" \
      -d "{\"refresh_token\":\"$refresh_token\"}" \
      "${API_GATEWAY_URL%/}/auth/refresh" >/dev/null
  fi
fi

# ai-orchestrator
if [[ "$SKIP_AI_ORCH" != "1" ]]; then
  check "ai-orchestrator live" "$AI_ORCH_URL" "/live"
  check "ai-orchestrator ready" "$AI_ORCH_URL" "/ready"
  check "ai-orchestrator metrics" "$AI_ORCH_URL" "/metrics"
fi

# search-service
if [[ "$SKIP_SEARCH" != "1" ]]; then
  check "search-service health" "$SEARCH_URL" "/admin/health"
  check "search-service metrics" "$SEARCH_URL" "/metrics"
fi

# file-storage
if [[ "$SKIP_FILE_STORAGE" != "1" ]]; then
  check "file-storage health" "$FILE_STORAGE_URL" "/health"
  check "file-storage deps" "$FILE_STORAGE_URL" "/health/deps"
  check "file-storage metrics" "$FILE_STORAGE_URL" "/metrics"
fi

# realtime-service
if [[ "$SKIP_REALTIME" != "1" ]]; then
  check "realtime-service health" "$REALTIME_URL" "/healthz"
  check "realtime-service ready" "$REALTIME_URL" "/readyz"
  check "realtime-service metrics" "$REALTIME_URL" "/metrics"
fi

# node-registry
if [[ "$SKIP_NODE_REGISTRY" != "1" ]]; then
  check "node-registry health" "$NODE_REGISTRY_URL" "/healthz"
  check "node-registry ready" "$NODE_REGISTRY_URL" "/readyz"

  if [[ "${RHELMA_SMOKE_NODE_REGISTRY_FLOW:-0}" == "1" ]]; then
    node_registry_flow
  fi
fi

# region-health-aggregator (optional)
if [[ "$SKIP_REGION_AGGREGATOR" != "1" ]]; then
  region_aggregator_check
fi

# kafka (optional)
if [[ "$SKIP_KAFKA" != "1" ]]; then
  kafka_check
fi

# llm-node (optional)
if [[ "$SKIP_LLM_NODE" != "1" ]]; then
  check "llm-node health" "$LLM_NODE_URL" "/health"
  # metrics endpoint is optional (depends on implementation)
  if ! curl -fsS --max-time "$TIMEOUT_SEC" "${LLM_NODE_URL%/}/metrics" >/dev/null 2>&1; then
    echo "  (llm-node metrics: not reachable; skipping)"
  else
    echo "- llm-node metrics: OK"
  fi
fi

echo -e "\nsmoke_staging: OK"
