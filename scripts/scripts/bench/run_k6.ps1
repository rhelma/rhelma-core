Param(
  [Parameter(Position=0)]
  [ValidateSet('api-gateway','node-registry')]
  [string]$Scenario = 'api-gateway'
)

$RootDir = Resolve-Path (Join-Path $PSScriptRoot '..\..')

switch ($Scenario) {
  'api-gateway' {
    $Script = 'benchmarks/k6/api_gateway_load.js'
    $EnvVar = 'RHELMA_API_URL'
    $DefaultUrl = 'http://localhost:3000'
  }
  'node-registry' {
    $Script = 'benchmarks/k6/node_registry_load.js'
    $EnvVar = 'RHELMA_NODE_REGISTRY_URL'
    $DefaultUrl = 'http://localhost:3001'
  }
}

$BaseUrl = (Get-Item -Path "Env:$EnvVar" -ErrorAction SilentlyContinue).Value
if ([string]::IsNullOrWhiteSpace($BaseUrl)) { $BaseUrl = $DefaultUrl }

Write-Host "==> Running k6: $Scenario"
Write-Host "    script: $Script"
Write-Host "    base url: $BaseUrl"

# Uses the official k6 container so you don't need k6 installed locally.
# Requirements: Docker.
docker run --rm -i `
  -e "$EnvVar=$BaseUrl" `
  -v "${RootDir}:/work" `
  -w /work `
  grafana/k6 run $Script
