#!/usr/bin/env bash
set -euo pipefail

ROOT="${1:-.}"

echo "[openapi_contract_guard] verifying OpenAPI multi-region surfaces under: $ROOT"

fail() {
  echo "[openapi_contract_guard] FAIL: $1" >&2
  exit 1
}

rha="$ROOT/docs/openapi/region-health-aggregator.yaml"
gw="$ROOT/docs/openapi/api-gateway.yaml"

[ -f "$rha" ] || fail "missing: docs/openapi/region-health-aggregator.yaml"
[ -f "$gw" ] || fail "missing: docs/openapi/api-gateway.yaml"

# Contract tagging
grep -q "version: 6.0.0" "$rha" || fail "region-health-aggregator: info.version must be 6.0.0"
grep -q "x-rhelma-contract-version: v6.0" "$rha" || fail "region-health-aggregator: missing x-rhelma-contract-version: v6.0"
grep -q "/v1/regions/health:" "$rha" || fail "region-health-aggregator: missing /v1/regions/health path"
grep -q "/v1/route:" "$rha" || fail "region-health-aggregator: missing /v1/route path"
grep -q "HealthSnapshot:" "$rha" || fail "region-health-aggregator: missing HealthSnapshot schema"
grep -q "RouteResponse:" "$rha" || fail "region-health-aggregator: missing RouteResponse schema"

grep -q "version: 6.0.0" "$gw" || fail "api-gateway: info.version must be 6.0.0"
grep -q "x-rhelma-contract-version: v6.0" "$gw" || fail "api-gateway: missing x-rhelma-contract-version: v6.0"
grep -q "/health/region/{region_id}:" "$gw" || fail "api-gateway: missing /health/region/{region_id} path"
grep -q "/admin/region-routing/snapshot:" "$gw" || fail "api-gateway: missing /admin/region-routing/snapshot path"
grep -q "/admin/region-routing/simulate-failover:" "$gw" || fail "api-gateway: missing /admin/region-routing/simulate-failover path"
grep -q "FailoverOverride:" "$gw" || fail "api-gateway: missing FailoverOverride schema"

echo "[openapi_contract_guard] OK"
