<#
Outbound HTTP context guard (Reqwest).

Windows wrapper around scripts/outbound_http_context_guard.py.
If Python is not available, this guard is skipped.
#>

param(
  [Parameter(Mandatory = $false)][string]$Root = "."
)

$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path $Root).Path

function Find-Python {
  $py = Get-Command python -ErrorAction SilentlyContinue
  if ($null -ne $py) { return $py.Path }
  $py = Get-Command py -ErrorAction SilentlyContinue
  if ($null -ne $py) { return $py.Path }
  return $null
}

$pythonExe = Find-Python
if ($null -eq $pythonExe) {
  Write-Host "[outbound_http_context_guard] SKIP: python not found (install python3 to enable)"
  exit 0
}

$scriptPath = Join-Path $PSScriptRoot "outbound_http_context_guard.py"
if (-not (Test-Path $scriptPath)) {
  Write-Host "[outbound_http_context_guard] SKIP: missing $scriptPath"
  exit 0
}

& $pythonExe $scriptPath $repoRoot
if ($LASTEXITCODE -ne 0) {
  Write-Host "[outbound_http_context_guard] FAILED (exit $LASTEXITCODE)"
  exit $LASTEXITCODE
}

Write-Host "[outbound_http_context_guard] OK"
