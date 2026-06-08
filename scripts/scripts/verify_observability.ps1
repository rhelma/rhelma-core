<#
Observability / Contract conformance verification.

This script is invoked by scripts/verify.ps1 and CI.

Important: It MUST accept -Root, and it must fail the build if a guard fails.
#>

param(
  [Parameter(Mandatory = $false)][string]$Root = "."
)

$ErrorActionPreference = "Stop"

function Invoke-Guard {
  param(
    [Parameter(Mandatory = $true)][string]$ScriptPath,
    [Parameter(Mandatory = $true)][string]$Root
  )

  if (-not (Test-Path $ScriptPath)) {
    Write-Host "[verify_observability] SKIP: missing $ScriptPath"
    return
  }

  powershell -NoProfile -ExecutionPolicy Bypass -File $ScriptPath -Root $Root
  if ($LASTEXITCODE -ne 0) {
    throw "guard failed: $ScriptPath (exit $LASTEXITCODE)"
  }
}

$repoRoot = (Resolve-Path $Root).Path

Invoke-Guard -ScriptPath "scripts/contract_guard.ps1" -Root $repoRoot
Invoke-Guard -ScriptPath "scripts/env_contract_guard.ps1" -Root $repoRoot
Invoke-Guard -ScriptPath "scripts/uuidv7_guard.ps1" -Root $repoRoot
Invoke-Guard -ScriptPath "scripts/event_contract_guard.ps1" -Root $repoRoot

Invoke-Guard -ScriptPath "scripts/metrics_cardinality_guard.ps1" -Root $repoRoot
Invoke-Guard -ScriptPath "scripts/scrapeability_guard.ps1" -Root $repoRoot

# Outbound HTTP propagation anti-drift (reqwest)
Invoke-Guard -ScriptPath "scripts/outbound_http_context_guard.ps1" -Root $repoRoot

Write-Host "[verify_observability] OK"
