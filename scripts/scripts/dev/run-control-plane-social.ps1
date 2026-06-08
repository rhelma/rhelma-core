$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

# scripts/dev/run-control-plane-social.ps1
# Windows / PowerShell equivalent of run-control-plane-social.sh
# Bring up: Postgres+Redis (docker) + control-service + social-service + api-gateway
# Then register a local social node into realm=central and keep it online via heartbeat.

function Write-Step([string]$msg) { Write-Host "➤ $msg" -ForegroundColor Cyan }
function Write-Ok([string]$msg)   { Write-Host "✓ $msg" -ForegroundColor Green }
function Write-Warn([string]$msg) { Write-Host "⚠ $msg" -ForegroundColor Yellow }
function Write-Err([string]$msg)  { Write-Host "✗ $msg" -ForegroundColor Red }

function Set-EnvDefault([string]$name, [string]$value) {
  $current = [Environment]::GetEnvironmentVariable($name)
  if ([string]::IsNullOrWhiteSpace($current)) {
    [Environment]::SetEnvironmentVariable($name, $value)
    $env:$name = $value
  }
}

function Import-DotEnv([string]$path) {
  if (-not (Test-Path $path)) { return }
  Get-Content $path | ForEach-Object {
    $line = $_.Trim()
    if ($line.Length -eq 0) { return }
    if ($line.StartsWith('#')) { return }

    $idx = $line.IndexOf('=')
    if ($idx -lt 1) { return }

    $key = $line.Substring(0, $idx).Trim()
    $val = $line.Substring($idx + 1).Trim()

    if (($val.StartsWith('"') -and $val.EndsWith('"')) -or ($val.StartsWith("'") -and $val.EndsWith("'"))) {
      $val = $val.Substring(1, $val.Length - 2)
    }

    if (-not [string]::IsNullOrWhiteSpace($key)) {
      $existing = [Environment]::GetEnvironmentVariable($key)
      if ([string]::IsNullOrWhiteSpace($existing)) {
        [Environment]::SetEnvironmentVariable($key, $val)
        $env:$key = $val
      }
    }
  }
}

function Wait-HttpOk([string]$url, [int]$retries = 60, [int]$delaySeconds = 1, [string]$name = 'service') {
  for ($i = 1; $i -le $retries; $i++) {
    try {
      Invoke-RestMethod -Method Get -Uri $url -TimeoutSec 2 | Out-Null
      Write-Ok "$name is up"
      return
    } catch {
      Start-Sleep -Seconds $delaySeconds
    }
  }
  throw "$name didn't come up: $url"
}

function Invoke-DockerCompose {
  param(
    [Parameter(Mandatory=$true)][string[]]$Args
  )

  $useV2 = $false
  try {
    docker compose version | Out-Null
    $useV2 = $true
  } catch {
    $useV2 = $false
  }

  if ($useV2) {
    & docker compose @Args
  } else {
    & docker-compose @Args
  }
}

$RootDir = Resolve-Path (Join-Path $PSScriptRoot '..\..') | Select-Object -ExpandProperty Path
Set-Location $RootDir

Import-DotEnv (Join-Path $RootDir '.env')

if ([string]::IsNullOrWhiteSpace($env:RHELMA_ENVIRONMENT)) {
  if (-not [string]::IsNullOrWhiteSpace($env:RHELMA_ENV)) { $env:RHELMA_ENVIRONMENT = $env:RHELMA_ENV } else { $env:RHELMA_ENVIRONMENT = 'development' }
}
Set-EnvDefault 'RHELMA_ENV' $env:RHELMA_ENVIRONMENT
Set-EnvDefault 'RHELMA_REGION' 'local'
Set-EnvDefault 'RHELMA_SERVICE_VERSION' '0.0.0-dev'

if ([string]::IsNullOrWhiteSpace($env:RHELMA_DB__URL)) {
  if (-not [string]::IsNullOrWhiteSpace($env:DATABASE_URL)) {
    $env:RHELMA_DB__URL = $env:DATABASE_URL
  } else {
    $env:RHELMA_DB__URL = 'postgres://rhelma_user:password@127.0.0.1:5432/rhelma_platform'
  }
}
Set-EnvDefault 'DATABASE_URL' $env:RHELMA_DB__URL
Set-EnvDefault 'RHELMA_REDIS__URL' 'redis://127.0.0.1:6379/0'

Set-EnvDefault 'RHELMA_DB__AUTO_MIGRATE' '1'

Set-EnvDefault 'RHELMA_SOCIAL_SERVICE_URL' 'http://127.0.0.1:8085'
Set-EnvDefault 'RHELMA_CONTROL_SERVICE_URL' 'http://127.0.0.1:8086'
Set-EnvDefault 'RHELMA_GATEWAY_DISCOVERY_CACHE_TTL_SECONDS' '30'

Set-EnvDefault 'RHELMA_CONTROL_LISTEN_ADDR' '0.0.0.0:8086'
Set-EnvDefault 'RHELMA_CONTROL_ADMIN_TOKEN' 'dev-admin'
Set-EnvDefault 'RHELMA_CONTROL_NODE_REGISTRATION_TOKEN' 'dev-node-token'

Set-EnvDefault 'RHELMA_SOCIAL_LISTEN_ADDR' '0.0.0.0:8085'

Set-EnvDefault 'RHELMA_SERVICE_NAME' 'api-gateway'
Set-EnvDefault 'RHELMA_BIND_HOST' '0.0.0.0'
Set-EnvDefault 'RHELMA_BIND_PORT' '3000'

$LogDir = Join-Path $RootDir '.rhelma-logs'
New-Item -ItemType Directory -Force -Path $LogDir | Out-Null
$ControlLog = Join-Path $LogDir 'control-service.log'
$SocialLog  = Join-Path $LogDir 'social-service.log'
$GatewayLog = Join-Path $LogDir 'api-gateway.log'

$cs = $null
$ss = $null
$gw = $null
$hbJob = $null

function Cleanup() {
  Write-Warn 'Stopping background services...'
  try { if ($hbJob) { Stop-Job $hbJob -Force -ErrorAction SilentlyContinue; Remove-Job $hbJob -Force -ErrorAction SilentlyContinue } } catch {}
  try { if ($gw -and -not $gw.HasExited) { Stop-Process -Id $gw.Id -Force -ErrorAction SilentlyContinue } } catch {}
  try { if ($ss -and -not $ss.HasExited) { Stop-Process -Id $ss.Id -Force -ErrorAction SilentlyContinue } } catch {}
  try { if ($cs -and -not $cs.HasExited) { Stop-Process -Id $cs.Id -Force -ErrorAction SilentlyContinue } } catch {}
}

try {
  Write-Step 'Starting Postgres + Redis (docker-compose.dev.yml)...'
  Invoke-DockerCompose -Args @('-f','docker-compose.dev.yml','up','-d','postgres','redis') | Out-Null
  Write-Ok 'Docker services started'

  Write-Step 'Waiting for Postgres to become healthy...'
  $healthy = $false
  for ($i = 1; $i -le 40; $i++) {
    try {
      $status = docker inspect -f "{{.State.Health.Status}}" rhelma-postgres 2>$null
      if ($status -eq 'healthy') { $healthy = $true; break }
    } catch {}
    Start-Sleep -Seconds 1
  }
  if (-not $healthy) { throw 'Postgres did not become healthy in time' }
  Write-Ok 'Postgres is healthy'

  Write-Step ("Starting control-service on $env:RHELMA_CONTROL_LISTEN_ADDR ...")
  $cs = Start-Process -FilePath 'cargo' -ArgumentList @('run','-p','control-service') -WorkingDirectory $RootDir -NoNewWindow -PassThru -RedirectStandardOutput $ControlLog -RedirectStandardError $ControlLog
  Start-Sleep -Seconds 1

  Write-Step 'Waiting for control-service health...'
  Wait-HttpOk ("$($env:RHELMA_CONTROL_SERVICE_URL)/health") 60 1 'control-service'

  Write-Step ("Starting social-service on $env:RHELMA_SOCIAL_LISTEN_ADDR ...")
  $ss = Start-Process -FilePath 'cargo' -ArgumentList @('run','-p','social-service') -WorkingDirectory $RootDir -NoNewWindow -PassThru -RedirectStandardOutput $SocialLog -RedirectStandardError $SocialLog
  Start-Sleep -Seconds 1

  Write-Step 'Waiting for social-service health...'
  Wait-HttpOk ("$($env:RHELMA_SOCIAL_SERVICE_URL)/health") 60 1 'social-service'

  Write-Step "Registering local social node into realm 'central'..."
  $payload = @{
    name = 'local-social'
    region = $env:RHELMA_REGION
    public_base_url = $env:RHELMA_SOCIAL_SERVICE_URL
    realm_slug = 'central'
    capabilities = @{ 'social-service' = $true }
    version = $env:RHELMA_SERVICE_VERSION
  } | ConvertTo-Json -Depth 6

  $reg = Invoke-RestMethod -Method Post -Uri ("$($env:RHELMA_CONTROL_SERVICE_URL)/v1/nodes/register") -Headers @{ 'x-control-node-registration-token' = $env:RHELMA_CONTROL_NODE_REGISTRATION_TOKEN } -ContentType 'application/json' -Body $payload -TimeoutSec 10

  if (-not $reg.node_id -or -not $reg.api_key) {
    throw "Failed to register node. Response: $($reg | ConvertTo-Json -Depth 10)"
  }

  Write-Ok ("Registered node_id={0} (api_key_hint={1})" -f $reg.node_id, $reg.api_key_hint)

  Write-Step 'Starting heartbeat loop...'
  $controlUrl = $env:RHELMA_CONTROL_SERVICE_URL
  $nodeId = $reg.node_id
  $apiKey = $reg.api_key

  $hbJob = Start-Job -ScriptBlock {
    param($controlUrl, $nodeId, $apiKey)
    $ProgressPreference = 'SilentlyContinue'
    while ($true) {
      try {
        Invoke-RestMethod -Method Post -Uri "$controlUrl/v1/nodes/$nodeId/heartbeat" -Headers @{ Authorization = "Bearer $apiKey" } -ContentType 'application/json' -Body '{"checks":{"ok":true}}' -TimeoutSec 5 | Out-Null
      } catch {}
      Start-Sleep -Seconds 20
    }
  } -ArgumentList $controlUrl, $nodeId, $apiKey

  Write-Step ("Starting api-gateway on $env:RHELMA_BIND_HOST:$env:RHELMA_BIND_PORT ...")
  $gw = Start-Process -FilePath 'cargo' -ArgumentList @('run','-p','api-gateway') -WorkingDirectory $RootDir -NoNewWindow -PassThru -RedirectStandardOutput $GatewayLog -RedirectStandardError $GatewayLog

  Write-Step 'Waiting for api-gateway health...'
  try {
    Wait-HttpOk ("http://127.0.0.1:$($env:RHELMA_BIND_PORT)/health") 60 1 'api-gateway'
  } catch {
    Write-Warn ("api-gateway health not detected yet. See logs: $GatewayLog")
  }

  Write-Ok 'READY 🎉'
  Write-Host ''
  Write-Host 'Try:' -ForegroundColor Cyan
  Write-Host ("  curl -H ""x-tenant-id: central"" http://127.0.0.1:{0}/social/health" -f $env:RHELMA_BIND_PORT)
  Write-Host ("  curl -H ""x-tenant-id: central"" http://127.0.0.1:{0}/social/feed/latest" -f $env:RHELMA_BIND_PORT)
  Write-Host ''
  Write-Host 'Logs:' -ForegroundColor Cyan
  Write-Host ("  type {0}" -f $ControlLog)
  Write-Host ("  type {0}" -f $SocialLog)
  Write-Host ("  type {0}" -f $GatewayLog)
  Write-Host ''
  Write-Step 'Press Ctrl+C to stop (services will be stopped).'

  Wait-Process -Id $gw.Id
} finally {
  Cleanup
}
