$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot ".."))

function Run-Step([string]$Name, [ScriptBlock]$Block) {
  Write-Host "`n=== $Name ===" -ForegroundColor Cyan
  & $Block
}

# Repo structure check is currently bash-only.
if (Get-Command bash -ErrorAction SilentlyContinue) {
  Run-Step "Repo structure" { bash (Join-Path $Root "scripts/check-structure.sh") }
} else {
  Write-Host "`n=== Repo structure ===" -ForegroundColor Cyan
  Write-Host "bash not found; skipping check-structure.sh" -ForegroundColor Yellow
}

Run-Step "Core verify" { & (Join-Path $Root "scripts/verify.ps1") }
Run-Step "Observability verify" { & (Join-Path $Root "scripts/verify_observability.ps1") }
Run-Step "Contract guard" { & (Join-Path $Root "scripts/contract_guard.ps1") }
Run-Step "Env contract guard" { & (Join-Path $Root "scripts/env_contract_guard.ps1") }
Run-Step "Event contract guard" { & (Join-Path $Root "scripts/event_contract_guard.ps1") }
Run-Step "UUIDv7 guard" { & (Join-Path $Root "scripts/uuidv7_guard.ps1") }
Run-Step "Scrapeability guard" { & (Join-Path $Root "scripts/scrapeability_guard.ps1") }
Run-Step "Metrics cardinality guard" { & (Join-Path $Root "scripts/metrics_cardinality_guard.ps1") }

# Optional: outbound HTTP context guard (introduced in later phases)
$guardPy = Join-Path $Root "scripts/outbound_http_context_guard.py"
if (Test-Path $guardPy) {
  $py = Get-Command python -ErrorAction SilentlyContinue
  if (-not $py) { $py = Get-Command py -ErrorAction SilentlyContinue }
  if ($py) {
    Run-Step "Outbound HTTP context guard" { & $py.Source $guardPy }
  } else {
    Write-Host "python not found; skipping outbound_http_context_guard.py" -ForegroundColor Yellow
  }
}

Run-Step "TODO/FIXME/HACK guard" { & (Join-Path $Root "scripts/todo_guard.ps1") }

Write-Host "`nverify_pre_frontend: OK" -ForegroundColor Green
