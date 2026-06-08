Param(
  [int]$TimeoutSec = $env:RHELMA_SMOKE_TIMEOUT_SEC
)

$ErrorActionPreference = "Stop"

if (-not $TimeoutSec -or $TimeoutSec -lt 1) { $TimeoutSec = 2 }

$ApiGatewayUrl = $env:RHELMA_SMOKE_API_GATEWAY_URL; if (-not $ApiGatewayUrl) { $ApiGatewayUrl = "http://127.0.0.1:3000" }
$AiOrchUrl = $env:RHELMA_SMOKE_AI_ORCH_URL; if (-not $AiOrchUrl) { $AiOrchUrl = "http://127.0.0.1:4000" }
$SearchUrl = $env:RHELMA_SMOKE_SEARCH_URL; if (-not $SearchUrl) { $SearchUrl = "http://127.0.0.1:8082" }
$FileStorageUrl = $env:RHELMA_SMOKE_FILE_STORAGE_URL; if (-not $FileStorageUrl) { $FileStorageUrl = "http://127.0.0.1:3005" }
$RealtimeUrl = $env:RHELMA_SMOKE_REALTIME_URL; if (-not $RealtimeUrl) { $RealtimeUrl = "http://127.0.0.1:9000" }
$NodeRegistryUrl = $env:RHELMA_SMOKE_NODE_REGISTRY_URL; if (-not $NodeRegistryUrl) { $NodeRegistryUrl = "http://127.0.0.1:8090" }
$LlmNodeUrl = $env:RHELMA_SMOKE_LLM_NODE_URL; if (-not $LlmNodeUrl) { $LlmNodeUrl = "http://127.0.0.1:8088" }

$SkipApiGateway = $env:RHELMA_SMOKE_SKIP_API_GATEWAY; if (-not $SkipApiGateway) { $SkipApiGateway = "0" }
$SkipAiOrch = $env:RHELMA_SMOKE_SKIP_AI_ORCH; if (-not $SkipAiOrch) { $SkipAiOrch = "0" }
$SkipSearch = $env:RHELMA_SMOKE_SKIP_SEARCH; if (-not $SkipSearch) { $SkipSearch = "0" }
$SkipFileStorage = $env:RHELMA_SMOKE_SKIP_FILE_STORAGE; if (-not $SkipFileStorage) { $SkipFileStorage = "0" }
$SkipRealtime = $env:RHELMA_SMOKE_SKIP_REALTIME; if (-not $SkipRealtime) { $SkipRealtime = "0" }
$SkipNodeRegistry = $env:RHELMA_SMOKE_SKIP_NODE_REGISTRY; if (-not $SkipNodeRegistry) { $SkipNodeRegistry = "0" }
$SkipLlmNode = $env:RHELMA_SMOKE_SKIP_LLM_NODE; if (-not $SkipLlmNode) { $SkipLlmNode = "0" }

function Join-Url([string]$Base, [string]$Path) {
  return ($Base.TrimEnd('/') + $Path)
}

function Check([string]$Name, [string]$Base, [string]$Path) {
  $url = Join-Url $Base $Path
  Write-Host "- $Name: GET $url"
  $r = Invoke-WebRequest -Uri $url -UseBasicParsing -TimeoutSec $TimeoutSec
  if ($r.StatusCode -lt 200 -or $r.StatusCode -ge 400) {
    throw "smoke_staging: $Name returned HTTP $($r.StatusCode)"
  }
}

function Hex-FromBytes([byte[]]$Bytes) {
  return ($Bytes | ForEach-Object { $_.ToString('x2') }) -join ''
}

function Bytes-FromHex([string]$Hex) {
  if (-not $Hex) { return $null }
  if ($Hex.Length % 2 -ne 0) { return $null }
  $out = New-Object byte[] ($Hex.Length/2)
  for ($i=0; $i -lt $out.Length; $i++) {
    $out[$i] = [Convert]::ToByte($Hex.Substring($i*2,2),16)
  }
  return $out
}

function LeadingZeroBits([byte[]]$Hash) {
  $z = 0
  foreach ($b in $Hash) {
    if ($b -eq 0) {
      $z += 8
      continue
    }
    # Count leading zeros in this byte.
    for ($i=7; $i -ge 0; $i--) {
      if (($b -band (1 -shl $i)) -eq 0) { $z++ } else { return $z }
    }
  }
  return $z
}

function Solve-Pow([string]$NonceHex, [int]$DifficultyBits, [int]$MaxIters) {
  $nonce = Bytes-FromHex $NonceHex
  if (-not $nonce -or $nonce.Length -ne 32) { return $null }
  $sha = [System.Security.Cryptography.SHA256]::Create()

  for ($i=0; $i -lt $MaxIters; $i++) {
    $sol = [BitConverter]::GetBytes([UInt64]$i) # little-endian
    $data = New-Object byte[] (32 + 8)
    [Array]::Copy($nonce, 0, $data, 0, 32)
    [Array]::Copy($sol, 0, $data, 32, 8)
    $h = $sha.ComputeHash($data)
    if ((LeadingZeroBits $h) -ge $DifficultyBits) {
      return (Hex-FromBytes $sol)
    }
  }
  return $null
}

function NodeRegistry-Flow() {
  # Generate a 32-byte lower-hex node_id.
  $bytes = New-Object byte[] 32
  [System.Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($bytes)
  $nodeId = Hex-FromBytes $bytes
  $issuedAt = (Get-Date).ToUniversalTime().ToString("o")

  $admission = $null
  $challengeUrl = (Join-Url $NodeRegistryUrl "/v1/admission/challenge") + "?node_id=$nodeId"
  try {
    $resp = Invoke-WebRequest -Uri $challengeUrl -UseBasicParsing -TimeoutSec $TimeoutSec
    if ($resp.StatusCode -ge 200 -and $resp.StatusCode -lt 300) {
      $obj = $resp.Content | ConvertFrom-Json
      $nonceHex = [string]$obj.nonce_hex
      $difficultyBits = [int]$obj.difficulty_bits
      if ($nonceHex -and $difficultyBits -ge 0) {
        $maxIters = 2000000
        if ($env:RHELMA_SMOKE_POW_MAX_ITERS) { $maxIters = [int]$env:RHELMA_SMOKE_POW_MAX_ITERS }
        Write-Host "- node-registry admission: solving PoW (difficulty_bits=$difficultyBits, max_iters=$maxIters)"
        $solutionHex = Solve-Pow $nonceHex $difficultyBits $maxIters
        if ($solutionHex) {
          $admission = @{ nonce_hex = $nonceHex; solution_hex = $solutionHex; difficulty_bits = $difficultyBits }
        } else {
          Write-Host "  (node-registry flow: PoW solve failed; skipping)" -ForegroundColor Yellow
          return
        }
      }
    }
  } catch {
    # PoW disabled or endpoint not reachable; proceed without admission.
  }

  $manifest = @{ 
    node_id = $nodeId;
    public_key_hex = $nodeId;
    display_name = "smoke-node";
    region = "local";
    allowed_residencies = @("local");
    capabilities = @("smoke");
    endpoints = @{ control_url = $null; data_url = $null };
    version = "0.0.0-smoke";
    issued_at = $issuedAt;
  }

  $req = @{ manifest = $manifest }
  if ($admission) { $req.admission = $admission }

  $body = $req | ConvertTo-Json -Depth 8
  Write-Host "- node-registry register: POST $(Join-Url $NodeRegistryUrl "/v1/nodes/register") (node_id=$nodeId)"
  Invoke-WebRequest -Uri (Join-Url $NodeRegistryUrl "/v1/nodes/register") -Method POST -UseBasicParsing -TimeoutSec $TimeoutSec -ContentType "application/json" -Body $body | Out-Null

  $hb = @{ node_id = $nodeId; observed_at = (Get-Date).ToUniversalTime().ToString("o"); load_avg_1m = $null; free_mem_mb = $null; notes = "smoke" }
  Invoke-WebRequest -Uri (Join-Url $NodeRegistryUrl "/v1/nodes/heartbeat") -Method POST -UseBasicParsing -TimeoutSec $TimeoutSec -ContentType "application/json" -Body ($hb | ConvertTo-Json) | Out-Null

  try {
    Invoke-WebRequest -Uri ((Join-Url $NodeRegistryUrl "/v1/nodes/discover") + "?capability=smoke&limit=1") -UseBasicParsing -TimeoutSec $TimeoutSec | Out-Null
    Write-Host "- node-registry discover: OK"
  } catch {
    Write-Host "  (node-registry discover: not reachable; skipping)" -ForegroundColor Yellow
  }
}

Write-Host "`nRHELMA smoke test (staging-ready endpoints)" -ForegroundColor Cyan
Write-Host "Timeout: ${TimeoutSec}s"

if ($SkipApiGateway -ne "1") {
  Check "api-gateway live" $ApiGatewayUrl "/health/"
  Check "api-gateway ready" $ApiGatewayUrl "/health/ready"
  Check "api-gateway metrics" $ApiGatewayUrl "/admin/metrics"
  Check "api-gateway auth health" $ApiGatewayUrl "/auth/health"

  if ($env:RHELMA_SMOKE_AUTH_FLOW -eq "1") {
    $tenant = if ($env:RHELMA_SMOKE_TENANT_ID) { $env:RHELMA_SMOKE_TENANT_ID } else { "local" }
    $email = "smoke_$([Guid]::NewGuid().ToString('N'))@example.local"
    $pass = if ($env:RHELMA_SMOKE_PASSWORD) { $env:RHELMA_SMOKE_PASSWORD } else { "SmokeTestPassw0rd!" }

    Write-Host "- api-gateway auth flow: register/login/refresh (tenant='$tenant' email='$email')"

    $regBody = @{ email=$email; password=$pass; name="smoke" } | ConvertTo-Json
    $reg = Invoke-WebRequest -Uri (Join-Url $ApiGatewayUrl "/auth/register") -Method POST -UseBasicParsing -TimeoutSec $TimeoutSec -Headers @{ "x-tenant-id" = $tenant } -ContentType "application/json" -Body $regBody
    $regObj = $reg.Content | ConvertFrom-Json
    $refresh = [string]$regObj.refresh_token
    if (-not $refresh) { throw "smoke_staging: could not extract refresh_token from register response" }

    $loginBody = @{ email=$email; password=$pass } | ConvertTo-Json
    Invoke-WebRequest -Uri (Join-Url $ApiGatewayUrl "/auth/login") -Method POST -UseBasicParsing -TimeoutSec $TimeoutSec -Headers @{ "x-tenant-id" = $tenant } -ContentType "application/json" -Body $loginBody | Out-Null

    $refreshBody = @{ refresh_token=$refresh } | ConvertTo-Json
    Invoke-WebRequest -Uri (Join-Url $ApiGatewayUrl "/auth/refresh") -Method POST -UseBasicParsing -TimeoutSec $TimeoutSec -Headers @{ "x-tenant-id" = $tenant } -ContentType "application/json" -Body $refreshBody | Out-Null
  }
}

if ($SkipAiOrch -ne "1") {
  Check "ai-orchestrator live" $AiOrchUrl "/live"
  Check "ai-orchestrator ready" $AiOrchUrl "/ready"
  Check "ai-orchestrator metrics" $AiOrchUrl "/metrics"
}

if ($SkipSearch -ne "1") {
  Check "search-service health" $SearchUrl "/admin/health"
  Check "search-service metrics" $SearchUrl "/metrics"
}

if ($SkipFileStorage -ne "1") {
  Check "file-storage health" $FileStorageUrl "/health"
  Check "file-storage deps" $FileStorageUrl "/health/deps"
  Check "file-storage metrics" $FileStorageUrl "/metrics"
}

if ($SkipRealtime -ne "1") {
  Check "realtime-service health" $RealtimeUrl "/healthz"
  Check "realtime-service ready" $RealtimeUrl "/readyz"
  Check "realtime-service metrics" $RealtimeUrl "/metrics"
}

if ($SkipNodeRegistry -ne "1") {
  Check "node-registry health" $NodeRegistryUrl "/healthz"
  Check "node-registry ready" $NodeRegistryUrl "/readyz"
  if ($env:RHELMA_SMOKE_NODE_REGISTRY_FLOW -eq "1") {
    NodeRegistry-Flow
  }
}

if ($SkipLlmNode -ne "1") {
  Check "llm-node health" $LlmNodeUrl "/health"
  try {
    Check "llm-node metrics" $LlmNodeUrl "/metrics"
  } catch {
    Write-Host "  (llm-node metrics: not reachable; skipping)" -ForegroundColor Yellow
  }
}

Write-Host "`nsmoke_staging: OK" -ForegroundColor Green
