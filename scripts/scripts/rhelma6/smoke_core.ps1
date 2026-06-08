Param(
  [int]$TimeoutSec = 2,
  [string]$ApiGatewayUrl = "http://127.0.0.1:3000",
  [string]$AiOrchUrl = "http://127.0.0.1:4000",
  [string]$NodeRegistryUrl = "http://127.0.0.1:8090"
)

$ErrorActionPreference = "Stop"

function Invoke-Smoke($name, $url) {
  Write-Host "[smoke_core] $name: $url"
  Invoke-WebRequest -UseBasicParsing -TimeoutSec $TimeoutSec -Uri $url | Out-Null
}

Invoke-Smoke "api-gateway health" "$ApiGatewayUrl/healthz"
Invoke-Smoke "ai-orchestrator health" "$AiOrchUrl/healthz"
Invoke-Smoke "node-registry health" "$NodeRegistryUrl/healthz"

Write-Host "[smoke_core] OK"
