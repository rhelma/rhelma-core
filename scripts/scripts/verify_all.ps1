$ErrorActionPreference = "Stop"

function Run-Step {
  param(
    [Parameter(Mandatory = $true)][string]$Name,
    [Parameter(Mandatory = $true)][scriptblock]$Block
  )

  Write-Host "\n=== $Name ==="
  & $Block
  if ($LASTEXITCODE -ne 0) {
    throw "$Name failed (exit $LASTEXITCODE)"
  }
}

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Push-Location $Root
try {
  Run-Step -Name "Repo structure" -Block { bash ./scripts/check-structure.sh }

  # Core Rust verification
  Run-Step -Name "Rust verify (fmt/clippy/tests)" -Block { powershell -NoProfile -ExecutionPolicy Bypass -File ./scripts/verify.ps1 }

  # Observability + contract gates
  Run-Step -Name "Observability verify" -Block { powershell -NoProfile -ExecutionPolicy Bypass -File ./scripts/verify_observability.ps1 -Root "." }
  Run-Step -Name "Contract guard" -Block { powershell -NoProfile -ExecutionPolicy Bypass -File ./scripts/contract_guard.ps1 }
  Run-Step -Name "Env contract guard" -Block { powershell -NoProfile -ExecutionPolicy Bypass -File ./scripts/env_contract_guard.ps1 }
  Run-Step -Name "Event contract guard" -Block { powershell -NoProfile -ExecutionPolicy Bypass -File ./scripts/event_contract_guard.ps1 }

  # Quality/consistency guards
  Run-Step -Name "UUIDv7 guard" -Block { powershell -NoProfile -ExecutionPolicy Bypass -File ./scripts/uuidv7_guard.ps1 }
  Run-Step -Name "Scrapeability guard" -Block { powershell -NoProfile -ExecutionPolicy Bypass -File ./scripts/scrapeability_guard.ps1 }
  Run-Step -Name "Metrics cardinality guard" -Block { powershell -NoProfile -ExecutionPolicy Bypass -File ./scripts/metrics_cardinality_guard.ps1 }

  # Optional: outbound HTTP context guard
  $pyGuard = Join-Path $Root "scripts/outbound_http_context_guard.py"
  if (Test-Path $pyGuard) {
    $py = Get-Command python -ErrorAction SilentlyContinue
    if ($null -eq $py) { $py = Get-Command python3 -ErrorAction SilentlyContinue }
    if ($null -ne $py) {
      Run-Step -Name "Outbound HTTP context guard" -Block { & $py.Path $pyGuard }
    } else {
      Write-Host "python/python3 not found; skipping outbound_http_context_guard.py"
    }
  }

  Run-Step -Name "TODO/FIXME/HACK guard" -Block { powershell -NoProfile -ExecutionPolicy Bypass -File ./scripts/todo_guard.ps1 }

  Write-Host "\nverify_all: OK"
} finally {
  Pop-Location
}
