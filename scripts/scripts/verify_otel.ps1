$ErrorActionPreference = "Stop"

# Optional OTEL verification gate.
#
# Enable by setting:
#   RHELMA_VERIFY_OTEL=1
#
# Kept separate so local `./scripts/verify.ps1` stays fast by default.

if ($env:RHELMA_VERIFY_OTEL -ne "1") {
  Write-Host "RHELMA_VERIFY_OTEL is not set to 1; skipping OTEL verification"
  exit 0
}

Write-Host "Running OTEL verification (rhelma-event-kafka --features otel)"

cargo test -p rhelma-event-kafka --features otel --tests
if ($LASTEXITCODE -ne 0) {
  throw "OTEL verification failed (exit $LASTEXITCODE)"
}

Write-Host "OTEL verification OK"
