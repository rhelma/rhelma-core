param(
  [Parameter(Mandatory = $false)][string]$Root = "."
)

$ErrorActionPreference = "Stop"

# Scrapeability / metrics endpoint guard (Phase 93)
#
# Ensures:
# 1) Prometheus local config (Phase 92) includes the expected jobs.
# 2) Each job uses metrics_path: /metrics.
# 3) Each service contains a /metrics route (best-effort static scan).

$repoRoot = (Resolve-Path $Root).Path

$promCfg = Join-Path $repoRoot "infra/monitoring/prometheus/prometheus.yml"
if (-not (Test-Path $promCfg)) {
  Write-Host "[scrapeability_guard] FAILED: missing $promCfg"
  exit 1
}

$cfgText = Get-Content -Raw -Path $promCfg

$needJobs = @(
  "api-gateway",
  "search-service",
  "ai-orchestrator",
  "rhelma-llm-node",
  "realtime-service",
  "file-storage"
)

$fail = $false

foreach ($job in $needJobs) {
  if ($cfgText -notmatch "(?m)^\s*-\s*job_name:\s*$job\s*$") {
    Write-Host "[FAIL] prometheus.yml is missing job_name: $job"
    $fail = $true
  }
}

if ($cfgText -notmatch "(?m)^\s*metrics_path:\s*/metrics\s*$") {
  Write-Host "[FAIL] prometheus.yml does not contain metrics_path: /metrics"
  $fail = $true
}

function Require-MetricsRoute([string]$name, [string[]]$dirs) {
  $found = $false
  foreach ($d in $dirs) {
    $p = Join-Path $repoRoot $d
    if (Test-Path $p) {
      $hits = Get-ChildItem -Path $p -Recurse -Filter *.rs -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -notmatch "\\target\\|\\\.git\\" } |
        Select-String -Pattern "\"/metrics\"" -SimpleMatch -ErrorAction SilentlyContinue
      if ($hits) { $found = $true; break }
    }
  }

  if (-not $found) {
    Write-Host "[FAIL] could not find a /metrics route for $name (scanned: $($dirs -join ', '))"
    $script:fail = $true
  }
}

Require-MetricsRoute "api-gateway" @("apps/api-gateway")
Require-MetricsRoute "search-service" @("apps/search-service")
Require-MetricsRoute "ai-orchestrator" @("apps/ai-orchestrator")
Require-MetricsRoute "realtime-service" @("apps/realtime-service")
Require-MetricsRoute "file-storage" @("apps/file-storage", "apps/file-storage-service")
Require-MetricsRoute "rhelma-llm-node" @("extras/llm-node", "apps/llm-node")

if ($fail) {
  Write-Host "[scrapeability_guard] FAILED"
  exit 1
}

Write-Host "[scrapeability_guard] OK"
