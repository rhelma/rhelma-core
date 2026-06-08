$ErrorActionPreference = "Stop"

param(
  [string]$Root = "."
)

Write-Host "[openapi_contract_guard] validating OpenAPI specs under: $Root"

$docs = Join-Path $Root "docs/openapi"
if (!(Test-Path $docs)) {
  Write-Error "[openapi_contract_guard] FAIL: missing docs/openapi directory"
  exit 1
}

# Prefer deep validation with python if available
$py = Get-Command python3 -ErrorAction SilentlyContinue
if ($null -ne $py) {
  & python3 (Join-Path $Root "scripts/guards/openapi_contract_check.py") $Root
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  exit 0
}

$req = @("api-gateway.yaml", "region-health-aggregator.yaml")
foreach ($f in $req) {
  $p = Join-Path $docs $f
  if (!(Test-Path $p)) {
    Write-Error "[openapi_contract_guard] FAIL: missing required spec: $p"
    exit 1
  }
  $txt = Get-Content $p -Raw
  if ($txt -notmatch "(?m)^openapi:\s*3\.0") {
    Write-Error "[openapi_contract_guard] FAIL: $f missing openapi: 3.0.x"
    exit 1
  }
  if ($txt -notmatch "(?m)^\s*version:\s*6\.0\.0\s*$") {
    Write-Error "[openapi_contract_guard] FAIL: $f missing info.version: 6.0.0"
    exit 1
  }
  if ($txt -notmatch "(?m)^x-rhelma-contract-version:\s*v6\.0\s*$") {
    Write-Error "[openapi_contract_guard] FAIL: $f missing x-rhelma-contract-version: v6.0"
    exit 1
  }
}

$gw = Get-Content (Join-Path $docs "api-gateway.yaml") -Raw
if ($gw -notmatch "/admin/region-routing/snapshot") {
  Write-Error "[openapi_contract_guard] FAIL: api-gateway.yaml missing /admin/region-routing/snapshot"
  exit 1
}
$rha = Get-Content (Join-Path $docs "region-health-aggregator.yaml") -Raw
if ($rha -notmatch "/v1/regions/health") {
  Write-Error "[openapi_contract_guard] FAIL: region-health-aggregator.yaml missing /v1/regions/health"
  exit 1
}

Write-Host "[openapi_contract_guard] OK (fallback mode; install python3 + PyYAML for full validation)"
