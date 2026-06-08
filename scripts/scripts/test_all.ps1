# Runs the standard developer verification suite.
#
# Default: fast, local, deterministic.
# Optional: set RHELMA_TEST_LIVE=1 to boot infra + core services and run live smoke.

$ErrorActionPreference = "Stop"

& ./scripts/verify_all.ps1
& ./scripts/e2e_local.ps1

if ($env:RHELMA_TEST_LIVE -eq "1") {
  $env:RHELMA_E2E_MODE = "live"
  $env:RHELMA_E2E_BOOT = "1"
  $env:RHELMA_E2E_SERVICES = "core"
  & ./scripts/e2e_local.ps1
}

Write-Host "[test_all] OK"
