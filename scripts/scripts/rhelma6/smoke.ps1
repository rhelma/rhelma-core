Param(
  [int]$TimeoutSec = 2,
  [string]$ApiGatewayUrl = "http://127.0.0.1:3000",
  [string]$AiOrchUrl = "http://127.0.0.1:4000",
  [string]$NodeRegistryUrl = "http://127.0.0.1:8090",
  [string]$SecurityGovUrl = "http://127.0.0.1:8091",
  [string]$GossipUrl = "http://127.0.0.1:8092",
  [string]$MniRagUrl = "http://127.0.0.1:8096"
)

$ErrorActionPreference = "Stop"

function Invoke-Smoke($name, $url) {
  Write-Host "[smoke] $name: $url"
  Invoke-WebRequest -UseBasicParsing -TimeoutSec $TimeoutSec -Uri $url | Out-Null
}

Invoke-Smoke "api-gateway health" "$ApiGatewayUrl/healthz"
Invoke-Smoke "ai-orchestrator health" "$AiOrchUrl/healthz"
Invoke-Smoke "node-registry health" "$NodeRegistryUrl/healthz"
Invoke-Smoke "security-governance health" "$SecurityGovUrl/healthz"
Invoke-Smoke "gossip-discovery peers" "$GossipUrl/v1/peers"
Invoke-Smoke "mni-rag health" "$MniRagUrl/healthz"

Write-Host "[smoke] OK"
