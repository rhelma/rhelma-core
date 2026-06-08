$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

# scripts/dev/run-social-mvp.ps1
# Windows / PowerShell "social-mvp" stack:
#   - docker: Postgres + Redis + Qdrant + Meilisearch (and MinIO if file-storage provider=s3)
#   - services: control-service, search-service, file-storage-service, realtime-service, social-service, api-gateway
#   - registers local social node into realm=central and keeps it online via heartbeat.

function Write-Step([string]$msg) { Write-Host "➤ $msg" -ForegroundColor Cyan }
function Write-Ok([string]$msg)   { Write-Host "✓ $msg" -ForegroundColor Green }
function Write-Warn([string]$msg) { Write-Host "⚠ $msg" -ForegroundColor Yellow }
function Write-Err([string]$msg)  { Write-Host "✗ $msg" -ForegroundColor Red }

function Set-EnvDefault([string]$name, [string]$value) {
  $current = [Environment]::GetEnvironmentVariable($name)
  if ([string]::IsNullOrWhiteSpace($current)) {
    [Environment]::SetEnvironmentVariable($name, $value)
    $env:$name = $value
  } else {
    $env:$name = $current
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
      } else {
        $env:$key = $existing
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
  throw "$name health not detected: $url"
}

$RootDir = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
Set-Location $RootDir

Import-DotEnv (Join-Path $RootDir ".env")

# CentralEnv strict fields
if ([string]::IsNullOrWhiteSpace($env:RHELMA_ENVIRONMENT)) { $env:RHELMA_ENVIRONMENT = $env:RHELMA_ENV }
Set-EnvDefault "RHELMA_ENVIRONMENT" ($env:RHELMA_ENVIRONMENT)
Set-EnvDefault "RHELMA_ENV" "development"  # if RHELMA_ENV is empty, Set-EnvDefault below will set it

if ([string]::IsNullOrWhiteSpace($env:RHELMA_ENV)) { $env:RHELMA_ENV = "development" }
Set-EnvDefault "RHELMA_ENV" ($env:RHELMA_ENV)
Set-EnvDefault "RHELMA_REGION" ($env:RHELMA_REGION)
Set-EnvDefault "RHELMA_SERVICE_VERSION" ($env:RHELMA_SERVICE_VERSION)

# Provide fallbacks if still empty
Set-EnvDefault "RHELMA_REGION" "local"
Set-EnvDefault "RHELMA_SERVICE_VERSION" "0.0.0-dev"

# Core infra
if ([string]::IsNullOrWhiteSpace($env:RHELMA_DB__URL)) { $env:RHELMA_DB__URL = $env:DATABASE_URL }
Set-EnvDefault "RHELMA_DB__URL" ($env:RHELMA_DB__URL)
Set-EnvDefault "DATABASE_URL" ($env:DATABASE_URL)
Set-EnvDefault "RHELMA_REDIS__URL" ($env:RHELMA_REDIS__URL)
Set-EnvDefault "RHELMA_DB__AUTO_MIGRATE" ($env:RHELMA_DB__AUTO_MIGRATE)

Set-EnvDefault "RHELMA_DB__URL" "postgres://rhelma_user:password@127.0.0.1:5432/rhelma_platform"
Set-EnvDefault "DATABASE_URL" $env:RHELMA_DB__URL
Set-EnvDefault "RHELMA_REDIS__URL" "redis://127.0.0.1:6379/0"
Set-EnvDefault "RHELMA_DB__AUTO_MIGRATE" "1"

# URLs
Set-EnvDefault "RHELMA_SEARCH_SERVICE_URL" ($env:RHELMA_SEARCH_SERVICE_URL)
Set-EnvDefault "FILE_STORAGE_URL" ($env:FILE_STORAGE_URL)
Set-EnvDefault "RHELMA_SOCIAL_SERVICE_URL" ($env:RHELMA_SOCIAL_SERVICE_URL)
Set-EnvDefault "RHELMA_CONTROL_SERVICE_URL" ($env:RHELMA_CONTROL_SERVICE_URL)
Set-EnvDefault "RHELMA_SMOKE_REALTIME_URL" ($env:RHELMA_SMOKE_REALTIME_URL)

Set-EnvDefault "RHELMA_SEARCH_SERVICE_URL" "http://127.0.0.1:8082"
Set-EnvDefault "FILE_STORAGE_URL" "http://127.0.0.1:3005"
Set-EnvDefault "RHELMA_SOCIAL_SERVICE_URL" "http://127.0.0.1:8085"
Set-EnvDefault "RHELMA_CONTROL_SERVICE_URL" "http://127.0.0.1:8086"
Set-EnvDefault "RHELMA_SMOKE_REALTIME_URL" "http://127.0.0.1:9000"

# Search backends
Set-EnvDefault "RHELMA_SEARCH_QDRANT_URL" ($env:RHELMA_SEARCH_QDRANT_URL)
Set-EnvDefault "RHELMA_SEARCH_MEILI_URL" ($env:RHELMA_SEARCH_MEILI_URL)
Set-EnvDefault "RHELMA_SEARCH_QDRANT_URL" "http://127.0.0.1:6333"
Set-EnvDefault "RHELMA_SEARCH_MEILI_URL" "http://127.0.0.1:7700"

# Listeners
Set-EnvDefault "RHELMA_SEARCH_LISTEN_ADDR" ($env:RHELMA_SEARCH_LISTEN_ADDR)
Set-EnvDefault "RHELMA_FILE_STORAGE__LISTEN_ADDR" ($env:RHELMA_FILE_STORAGE__LISTEN_ADDR)
Set-EnvDefault "RHELMA_FILE_STORAGE__DATABASE_URL" ($env:RHELMA_FILE_STORAGE__DATABASE_URL)
Set-EnvDefault "RHELMA_FILE_STORAGE__PROVIDER" ($env:RHELMA_FILE_STORAGE__PROVIDER)
Set-EnvDefault "RHELMA_FILE_STORAGE__LOCAL_ROOT" ($env:RHELMA_FILE_STORAGE__LOCAL_ROOT)
Set-EnvDefault "RHELMA_RT_LISTEN_ADDR" ($env:RHELMA_RT_LISTEN_ADDR)
Set-EnvDefault "REALTIME_ALLOW_ANONYMOUS" ($env:REALTIME_ALLOW_ANONYMOUS)

Set-EnvDefault "RHELMA_SEARCH_LISTEN_ADDR" "0.0.0.0:8082"
Set-EnvDefault "RHELMA_FILE_STORAGE__LISTEN_ADDR" "0.0.0.0:3005"
Set-EnvDefault "RHELMA_FILE_STORAGE__DATABASE_URL" $env:DATABASE_URL
Set-EnvDefault "RHELMA_FILE_STORAGE__PROVIDER" "local"
Set-EnvDefault "RHELMA_FILE_STORAGE__LOCAL_ROOT" ".\data\files"
Set-EnvDefault "RHELMA_RT_LISTEN_ADDR" "0.0.0.0:9000"
Set-EnvDefault "REALTIME_ALLOW_ANONYMOUS" "true"

# Control + social + gateway
Set-EnvDefault "RHELMA_CONTROL_LISTEN_ADDR" ($env:RHELMA_CONTROL_LISTEN_ADDR)
Set-EnvDefault "RHELMA_CONTROL_ADMIN_TOKEN" ($env:RHELMA_CONTROL_ADMIN_TOKEN)
Set-EnvDefault "RHELMA_CONTROL_NODE_REGISTRATION_TOKEN" ($env:RHELMA_CONTROL_NODE_REGISTRATION_TOKEN)
Set-EnvDefault "RHELMA_SOCIAL_LISTEN_ADDR" ($env:RHELMA_SOCIAL_LISTEN_ADDR)
Set-EnvDefault "RHELMA_GATEWAY_DISCOVERY_CACHE_TTL_SECONDS" ($env:RHELMA_GATEWAY_DISCOVERY_CACHE_TTL_SECONDS)
Set-EnvDefault "RHELMA_BIND_HOST" ($env:RHELMA_BIND_HOST)
Set-EnvDefault "RHELMA_BIND_PORT" ($env:RHELMA_BIND_PORT)

Set-EnvDefault "RHELMA_CONTROL_LISTEN_ADDR" "0.0.0.0:8086"
Set-EnvDefault "RHELMA_CONTROL_ADMIN_TOKEN" "dev-admin"
Set-EnvDefault "RHELMA_CONTROL_NODE_REGISTRATION_TOKEN" "dev-node-token"
Set-EnvDefault "RHELMA_SOCIAL_LISTEN_ADDR" "0.0.0.0:8085"
Set-EnvDefault "RHELMA_GATEWAY_DISCOVERY_CACHE_TTL_SECONDS" "30"
Set-EnvDefault "RHELMA_BIND_HOST" "0.0.0.0"
Set-EnvDefault "RHELMA_BIND_PORT" "3000"

# Logs
$ControlLog  = Join-Path $env:TEMP "control-service.log"
$SearchLog   = Join-Path $env:TEMP "search-service.log"
$FileLog     = Join-Path $env:TEMP "file-storage.log"
$RealtimeLog = Join-Path $env:TEMP "realtime-service.log"
$SocialLog   = Join-Path $env:TEMP "social-service.log"
$GatewayLog  = Join-Path $env:TEMP "api-gateway.log"

" " | Out-File -FilePath $ControlLog  -Encoding utf8
" " | Out-File -FilePath $SearchLog   -Encoding utf8
" " | Out-File -FilePath $FileLog     -Encoding utf8
" " | Out-File -FilePath $RealtimeLog -Encoding utf8
" " | Out-File -FilePath $SocialLog   -Encoding utf8
" " | Out-File -FilePath $GatewayLog  -Encoding utf8

try {
  Write-Step "Starting docker infra (Postgres + Redis + Qdrant + Meilisearch)..."
  if ($env:RHELMA_FILE_STORAGE__PROVIDER -eq "s3") {
    docker compose -f docker-compose.dev.yml --profile s3 up -d postgres redis qdrant meilisearch minio | Out-Null
  } else {
    docker compose -f docker-compose.dev.yml up -d postgres redis qdrant meilisearch | Out-Null
  }
  Write-Ok "Docker services started"

  Write-Step "Waiting for Postgres to become healthy..."
  for ($i = 0; $i -lt 40; $i++) {
    try {
      $status = (docker inspect -f '{{.State.Health.Status}}' rhelma-postgres 2>$null)
      if ($status -eq "healthy") { Write-Ok "Postgres is healthy"; break }
    } catch {}
    Start-Sleep -Seconds 1
    if ($i -eq 39) { Write-Err "Postgres did not become healthy in time" }
  }

  Write-Step ("Starting control-service on $env:RHELMA_CONTROL_LISTEN_ADDR ...")
  $cs = Start-Process -FilePath 'cargo' -ArgumentList @('run','-p','control-service') -WorkingDirectory $RootDir -NoNewWindow -PassThru -RedirectStandardOutput $ControlLog -RedirectStandardError $ControlLog
  Start-Sleep -Seconds 1
  Write-Step 'Waiting for control-service health...'
  Wait-HttpOk ("$($env:RHELMA_CONTROL_SERVICE_URL)/health") 60 1 'control-service'

  Write-Step ("Starting search-service on $env:RHELMA_SEARCH_LISTEN_ADDR ...")
  $search = Start-Process -FilePath 'cargo' -ArgumentList @('run','-p','search-service') -WorkingDirectory $RootDir -NoNewWindow -PassThru -RedirectStandardOutput $SearchLog -RedirectStandardError $SearchLog
  Start-Sleep -Seconds 1
  Write-Step 'Waiting for search-service health...'
  try { Wait-HttpOk ("$($env:RHELMA_SEARCH_SERVICE_URL)/healthz") 60 1 'search-service' } catch { Write-Warn "search-service health not detected yet (see $SearchLog)" }

  Write-Step ("Starting file-storage-service on $env:RHELMA_FILE_STORAGE__LISTEN_ADDR ...")
  $fs = Start-Process -FilePath 'cargo' -ArgumentList @('run','-p','file-storage-service') -WorkingDirectory $RootDir -NoNewWindow -PassThru -RedirectStandardOutput $FileLog -RedirectStandardError $FileLog
  Start-Sleep -Seconds 1
  Write-Step 'Waiting for file-storage-service health...'
  try { Wait-HttpOk ("$($env:FILE_STORAGE_URL)/healthz") 60 1 'file-storage-service' } catch { Write-Warn "file-storage health not detected yet (see $FileLog)" }

  Write-Step ("Starting realtime-service on $env:RHELMA_RT_LISTEN_ADDR ...")
  $rt = Start-Process -FilePath 'cargo' -ArgumentList @('run','-p','realtime-service') -WorkingDirectory $RootDir -NoNewWindow -PassThru -RedirectStandardOutput $RealtimeLog -RedirectStandardError $RealtimeLog
  Start-Sleep -Seconds 1
  Write-Step 'Waiting for realtime-service health...'
  try { Wait-HttpOk ("$($env:RHELMA_SMOKE_REALTIME_URL)/healthz") 60 1 'realtime-service' } catch { Write-Warn "realtime health not detected yet (see $RealtimeLog)" }

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

  $reg = Invoke-RestMethod -Method Post -Uri ("$($env:RHELMA_CONTROL_SERVICE_URL)/v1/nodes/register") -Headers @{ 'x-control-node-registration-token' = $env:RHELMA_CONTROL_NODE_REGISTRATION_TOKEN } -ContentType 'application/json' -Body $payload

  Write-Ok ("Registered node_id=$($reg.node_id)")

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
  Write-Host ("Gateway:         http://127.0.0.1:$($env:RHELMA_BIND_PORT)/")
  Write-Host ("Logs (Temp):     $ControlLog, $SearchLog, $FileLog, $RealtimeLog, $SocialLog, $GatewayLog")
  Write-Host ''
  Write-Step 'Press Ctrl-C to stop (services will be stopped).'

  while ($true) { Start-Sleep -Seconds 2 }
}
finally {
  Write-Warn 'Stopping...'
  try { docker compose -f docker-compose.dev.yml down | Out-Null } catch {}
  foreach ($p in @($gw, $ss, $rt, $fs, $search, $cs)) { try { if ($p) { $p.Kill() } } catch {} }
  try { if ($hbJob) { Stop-Job $hbJob -Force | Out-Null; Remove-Job $hbJob -Force | Out-Null } } catch {}
}
