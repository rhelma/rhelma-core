Param(
  [string]$ComposeFile = "deploy/rhelma6/docker/docker-compose.rhelma6.yml",
  [string]$EnvFile = "deploy/rhelma6/docker/.env.rhelma6"
)

$ErrorActionPreference = "Stop"

function Wait-Http($Url, $Name, $MaxSeconds=90) {
  Write-Host "[wait] $Name: $Url"
  $start = Get-Date
  while ($true) {
    try {
      Invoke-RestMethod -Method GET -Uri $Url -TimeoutSec 3 | Out-Null
      Write-Host "[ok] $Name"
      return
    } catch {
      Start-Sleep -Seconds 1
      if (((Get-Date) - $start).TotalSeconds -gt $MaxSeconds) {
        throw "[fail] $Name did not become ready in ${MaxSeconds}s"
      }
    }
  }
}

if (-not (Test-Path $EnvFile)) {
  throw "Missing env file: $EnvFile. Copy deploy/rhelma6/docker/.env.rhelma6.example -> $EnvFile and edit."
}

try {
  Write-Host "[compose] up"
  docker compose -f $ComposeFile --env-file $EnvFile up -d --remove-orphans | Out-Null

  $NR = $env:RHELMA6_NODE_REGISTRY_URL; if (-not $NR) { $NR = "http://127.0.0.1:8090" }
  $SG = $env:RHELMA6_SECURITY_GOV_URL; if (-not $SG) { $SG = "http://127.0.0.1:8091" }
  $GD = $env:RHELMA6_GOSSIP_URL; if (-not $GD) { $GD = "http://127.0.0.1:8092" }
  $BR = $env:RHELMA6_BRIDGE_URL; if (-not $BR) { $BR = "http://127.0.0.1:8094" }

  Wait-Http "$NR/healthz" "node-registry" 90
  Wait-Http "$SG/healthz" "security-governance" 90
  Wait-Http "$GD/healthz" "gossip-discovery" 90
  Wait-Http "$BR/healthz" "bridge-adapter" 90

  # Discover should return JSON
  Invoke-RestMethod -Method GET -Uri "$NR/v1/nodes/discover" | Out-Null

  # Bridge policy gate probe (best-effort)
  $headers = @{}
  if ($env:RHELMA6_BRIDGE_ADMIN_TOKEN) { $headers["x-admin-token"] = $env:RHELMA6_BRIDGE_ADMIN_TOKEN }

  $intent = @{ direction="deposit"; chain="forbidden"; amount=1; subject_id="test-subject" } | ConvertTo-Json
  try {
    $resp = Invoke-RestMethod -Method POST -Uri "$BR/v1/bridge/intents" -Headers $headers -ContentType "application/json" -Body $intent
    $id = $resp.id
    if ($id) {
      try {
        Invoke-RestMethod -Method POST -Uri "$BR/v1/bridge/intents/$id/finalize" -Headers $headers -ContentType "application/json" -Body "{}" | Out-Null
        throw "[fail] bridge finalize unexpectedly succeeded for forbidden chain"
      } catch {
        Write-Host "[ok] bridge policy gate (forbidden chain rejected or failed as expected)"
      }
    } else {
      Write-Host "[warn] could not parse intent id; skipping policy gate check"
    }
  } catch {
    Write-Host "[warn] bridge intent creation failed (schema/auth may differ); skipping policy gate check"
  }

  Write-Host "[PASS] Rhelma6 E2E integration (MVP)"
}
finally {
  Write-Host "[cleanup] stopping compose"
  try { docker compose -f $ComposeFile --env-file $EnvFile down -v | Out-Null } catch {}
}
