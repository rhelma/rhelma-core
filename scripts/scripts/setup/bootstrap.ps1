$ErrorActionPreference = "Stop"

Write-Host "== Rhelma bootstrap =="

try {
  powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\setup\preflight.ps1" | Out-Host
} catch {
  # Preflight is best-effort in bootstrap.
}

if (-not (Test-Path ".env")) {
  if (Test-Path ".env.example") {
    Copy-Item ".env.example" ".env"
    Write-Host "✅ Created .env from .env.example" -ForegroundColor Green
    Write-Host "⚠️  Review .env and adjust secrets/ports as needed." -ForegroundColor Yellow
  } else {
    Write-Host "⚠️  .env.example not found; skipping .env creation." -ForegroundColor Yellow
  }
} else {
  Write-Host "✅ .env already exists" -ForegroundColor Green
}

if (-not (Test-Path "keys\private.pem") -or -not (Test-Path "keys\public.pem")) {
  Write-Host "Generating RSA keys under .\keys ..." -ForegroundColor Cyan
  try {
    powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\setup\generate-keys.ps1" -KeysDir ".\keys" | Out-Host
  } catch {
    # best-effort
  }
} else {
  Write-Host "✅ keys\private.pem + keys\public.pem already exist" -ForegroundColor Green
}

Write-Host ""
Write-Host "Next steps:"
Write-Host "- Run full verification:   .\scripts\verify_all.ps1"
Write-Host "- Start local world stack: .\scripts\run-world.sh (WSL) or bash scripts/run-world.sh"
Write-Host "- Run local smoke checks:  .\scripts\smoke_local.ps1"
