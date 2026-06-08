param(
  [ValidateSet("inprocess","live")]
  [string]$Mode = $(if ($env:RHELMA_E2E_MODE) { $env:RHELMA_E2E_MODE } else { "inprocess" }),
  [string]$Boot = $(if ($env:RHELMA_E2E_BOOT) { $env:RHELMA_E2E_BOOT } else { "0" }),
  [string]$Services = $(if ($env:RHELMA_E2E_SERVICES) { $env:RHELMA_E2E_SERVICES } else { "api-gateway,search-service" })
)

$ErrorActionPreference = "Stop"

function Run-Native {
  param(
    [Parameter(Mandatory = $true)][string]$Name,
    [Parameter(Mandatory = $true)][scriptblock]$Block
  )
  & $Block
  if ($LASTEXITCODE -ne 0) {
    Die "$Name failed (exit $LASTEXITCODE)"
  }
}

function Log($msg) { Write-Host "`n[e2e] $msg" }
function Dump-Logs {
  $logsDir = Join-Path $RootDir ".e2e/logs"
  if (-not (Test-Path $logsDir)) { return }
  Log "log tails (last 120 lines per service)"
  Get-ChildItem -Path $logsDir -Filter "*.log" -ErrorAction SilentlyContinue | ForEach-Object {
    Write-Host "`n--- $($_.Name) ---"
    try { Get-Content -Path $_.FullName -ErrorAction SilentlyContinue | Select-Object -Last 120 } catch {}
  }
}
function Die($msg) {
  Write-Host "[e2e] ERROR: $msg"
  Dump-Logs
  exit 1
}

$RootDir = (Resolve-Path (Join-Path $PSScriptRoot "../.."))
Set-Location $RootDir

# Load .env if present (best-effort)
$EnvFile = Join-Path $RootDir ".env"
if (Test-Path $EnvFile) {
  Get-Content $EnvFile | ForEach-Object {
    if ($_ -match '^\s*#') { return }
    if ($_ -match '^\s*$') { return }
    if ($_ -match '^([^=]+)=(.*)$') {
      $k = $Matches[1].Trim()
      $v = $Matches[2].Trim().Trim('"')
      [Environment]::SetEnvironmentVariable($k, $v)
    }
  }
}

# Central identity defaults
if (-not $env:RHELMA_ENV) { $env:RHELMA_ENV = "development" }
if (-not $env:RHELMA_ENVIRONMENT) { $env:RHELMA_ENVIRONMENT = $env:RHELMA_ENV }
if (-not $env:RHELMA_REGION) { $env:RHELMA_REGION = "local" }
if (-not $env:RHELMA_SERVICE_VERSION) { $env:RHELMA_SERVICE_VERSION = "0.0.0-dev" }

function Expand-Services([string]$csv) {
  switch ($csv.Trim().ToLowerInvariant()) {
    "core" {
      return "api-gateway,ai-orchestrator,search-service,file-storage-service,realtime-service,node-registry"
    }
    "all" {
      return "api-gateway,ai-orchestrator,search-service,file-storage-service,realtime-service,node-registry,gossip-discovery,guardian-agent,security-governance,value-ledger,value-ledger-federation,mni-rag,patch-applier,rhelma-node,edge-worker,web,digital-family-vault"
    }
    default { return $csv }
  }
}

$ServicesExpanded = Expand-Services $Services

function Has-Service([string]$name) {
  return $ServicesExpanded -match "(^|,)$([Regex]::Escape($name))(,|$)"
}

# URLs
$ApiUrl = if ($env:RHELMA_E2E_API_GATEWAY_URL) { $env:RHELMA_E2E_API_GATEWAY_URL } else { "http://127.0.0.1:3000" }
$AiOrchUrl = if ($env:RHELMA_E2E_AI_ORCH_URL) { $env:RHELMA_E2E_AI_ORCH_URL } else { "http://127.0.0.1:4000" }
$SearchUrl = if ($env:RHELMA_E2E_SEARCH_URL) { $env:RHELMA_E2E_SEARCH_URL } else { "http://127.0.0.1:8082" }
$FileStorageUrl = if ($env:RHELMA_E2E_FILE_STORAGE_URL) { $env:RHELMA_E2E_FILE_STORAGE_URL } else { "http://127.0.0.1:3005" }
$RealtimeUrl = if ($env:RHELMA_E2E_REALTIME_URL) { $env:RHELMA_E2E_REALTIME_URL } else { "http://127.0.0.1:9000" }
$NodeRegistryUrl = if ($env:RHELMA_E2E_NODE_REGISTRY_URL) { $env:RHELMA_E2E_NODE_REGISTRY_URL } else { "http://127.0.0.1:8090" }
$LlmNodeUrl = if ($env:RHELMA_E2E_LLM_NODE_URL) { $env:RHELMA_E2E_LLM_NODE_URL } else { "http://127.0.0.1:8088" }

$TimeoutSec = if ($env:RHELMA_E2E_WAIT_TIMEOUT_SEC) { [int]$env:RHELMA_E2E_WAIT_TIMEOUT_SEC } else { 45 }

function Wait-HttpOk([string]$Url, [int]$TimeoutSec) {
  $sw = [Diagnostics.Stopwatch]::StartNew()
  while ($sw.Elapsed.TotalSeconds -lt $TimeoutSec) {
    try {
      Invoke-WebRequest -UseBasicParsing -Uri $Url -TimeoutSec 2 | Out-Null
      return $true
    } catch {
      Start-Sleep -Milliseconds 500
    }
  }
  return $false
}

$pids = @()
function Cleanup {
  if ($pids.Count -gt 0) {
    Log "stopping booted services..."
    foreach ($pid in $pids) {
      try { Stop-Process -Id $pid -Force -ErrorAction SilentlyContinue } catch {}
    }
  }

  if ($Mode -eq "live" -and $Boot -eq "1") {
    try { docker compose -f docker-compose.dev.yml down | Out-Null } catch {}
  }
}
Register-EngineEvent PowerShell.Exiting -Action { Cleanup } | Out-Null

function Profiles-For-Services([string]$servicesCsv) {
  if ($env:RHELMA_E2E_DOCKER_PROFILES) { return $env:RHELMA_E2E_DOCKER_PROFILES }
  $profiles = @()
  if ($servicesCsv -match '(^|,)ai-orchestrator(,|$)' -or $servicesCsv -match '(^|,)patch-applier(,|$)') { $profiles += "kafka" }
  if ($env:RHELMA_E2E_ENABLE_OBS -eq "1") { $profiles += "obs" }
  if ($env:RHELMA_E2E_ENABLE_S3 -eq "1") { $profiles += "s3" }
  return ($profiles -join ',')
}

function Boot-Infra([string]$servicesCsv) {
  if (-not (Get-Command docker -ErrorAction SilentlyContinue)) { Die "docker is required for RHELMA_E2E_BOOT=1" }
  try { docker compose version | Out-Null } catch { Die "'docker compose' (Compose v2) is required" }

  $profilesCsv = Profiles-For-Services $servicesCsv
  Log "booting infra via docker compose (docker-compose.dev.yml) profiles='$(if ($profilesCsv) { $profilesCsv } else { "none" })'"

  $args = @("compose","-f","docker-compose.dev.yml")
  if ($profilesCsv) {
    $profilesCsv.Split(',') | ForEach-Object {
      $p = $_.Trim(); if ($p) { $args += @("--profile", $p) }
    }
  }
  $args += @("up","-d","--remove-orphans")

  docker @args | Out-Null
  if ($LASTEXITCODE -ne 0) { Die "docker compose up failed (exit $LASTEXITCODE)" }
}

function Boot-Service([string]$Svc) {
  if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { Die "cargo is required to boot services" }
  $logsDir = Join-Path $RootDir ".e2e/logs"
  New-Item -ItemType Directory -Force -Path $logsDir | Out-Null
  Log "booting service: $Svc"
  $logPath = Join-Path $logsDir "$Svc.log"
  $p = Start-Process -FilePath "cargo" -ArgumentList @("run","-q","-p",$Svc) -RedirectStandardOutput $logPath -RedirectStandardError $logPath -PassThru
  $pids += $p.Id

  Start-Sleep -Milliseconds 800
  $p.Refresh()
  if ($p.HasExited) {
    $tail = ""
    try { $tail = ((Get-Content -Path $logPath -ErrorAction SilentlyContinue | Select-Object -Last 60) -join "`n") } catch {}
    Die "service '$Svc' exited early (code $($p.ExitCode)). Log tail:`n$tail"
  }
}

function Run-Inprocess {
  if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { Die "cargo is required for inprocess e2e" }
  Log "running inprocess tests (e2e-tests)"
  Run-Native -Name "cargo test -p e2e-tests" -Block { cargo test -q -p e2e-tests }
  Log "inprocess tests: OK"
}

function Run-Live {
  Log "waiting for selected services (timeout ${TimeoutSec}s)"

  if (Has-Service "api-gateway") {
    if (-not (Wait-HttpOk "$ApiUrl/health/ready" $TimeoutSec)) { Die "api-gateway not ready at $ApiUrl/health/ready" }
  }
  if (Has-Service "ai-orchestrator") {
    if (-not (Wait-HttpOk "$AiOrchUrl/ready" $TimeoutSec)) { Die "ai-orchestrator not ready at $AiOrchUrl/ready" }
  }
  if (Has-Service "search-service") {
    if (-not (Wait-HttpOk "$SearchUrl/admin/health" $TimeoutSec)) { Die "search-service not ready at $SearchUrl/admin/health" }
  }
  if (Has-Service "file-storage-service") {
    if (-not (Wait-HttpOk "$FileStorageUrl/health" $TimeoutSec)) { Die "file-storage-service not ready at $FileStorageUrl/health" }
  }
  if (Has-Service "realtime-service") {
    if (-not (Wait-HttpOk "$RealtimeUrl/readyz" $TimeoutSec)) { Die "realtime-service not ready at $RealtimeUrl/readyz" }
  }
  if (Has-Service "node-registry") {
    if (-not (Wait-HttpOk "$NodeRegistryUrl/readyz" $TimeoutSec)) { Die "node-registry not ready at $NodeRegistryUrl/readyz" }
  }
  if (Has-Service "rhelma-node") {
    if (-not (Wait-HttpOk "$LlmNodeUrl/health" $TimeoutSec)) { Die "rhelma-node not ready at $LlmNodeUrl/health" }
  }

  # Wire smoke URLs from the E2E harness.
  $env:RHELMA_E2E_API_GATEWAY_URL = $ApiUrl
  $env:RHELMA_E2E_AI_ORCH_URL = $AiOrchUrl
  $env:RHELMA_E2E_SEARCH_URL = $SearchUrl
  $env:RHELMA_E2E_FILE_STORAGE_URL = $FileStorageUrl
  $env:RHELMA_E2E_REALTIME_URL = $RealtimeUrl
  $env:RHELMA_E2E_NODE_REGISTRY_URL = $NodeRegistryUrl
  $env:RHELMA_E2E_LLM_NODE_URL = $LlmNodeUrl
  $env:RHELMA_E2E_WAIT_TIMEOUT_SEC = "$TimeoutSec"

  # Set smoke skip flags based on selected services.
  $env:RHELMA_SMOKE_SKIP_API_GATEWAY = if (Has-Service "api-gateway") { "0" } else { "1" }
  $env:RHELMA_SMOKE_SKIP_AI_ORCH = if (Has-Service "ai-orchestrator") { "0" } else { "1" }
  $env:RHELMA_SMOKE_SKIP_SEARCH = if (Has-Service "search-service") { "0" } else { "1" }
  $env:RHELMA_SMOKE_SKIP_FILE_STORAGE = if (Has-Service "file-storage-service") { "0" } else { "1" }
  $env:RHELMA_SMOKE_SKIP_REALTIME = if (Has-Service "realtime-service") { "0" } else { "1" }
  $env:RHELMA_SMOKE_SKIP_NODE_REGISTRY = if (Has-Service "node-registry") { "0" } else { "1" }
  $env:RHELMA_SMOKE_SKIP_LLM_NODE = if (Has-Service "rhelma-node") { "0" } else { "1" }

  Log "running live smoke checks"
  & ./scripts/smoke_local.ps1
  if ($LASTEXITCODE -ne 0) { Die "smoke_local failed (exit $LASTEXITCODE)" }

  Log "live smoke: OK"
}

Log "mode=$Mode boot=$Boot services=$ServicesExpanded"

if ($Mode -eq "inprocess") {
  Run-Inprocess
  exit 0
}

if ($Mode -ne "live") { Die "unknown mode: $Mode (expected inprocess|live)" }

if ($Boot -eq "1") {
  Boot-Infra $ServicesExpanded
  $ServicesExpanded.Split(',') | ForEach-Object { $s = $_.Trim(); if ($s) { Boot-Service $s } }
}

Run-Live
