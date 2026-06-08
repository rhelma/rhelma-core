param(
  [string]$Root = "."
)
# Best-effort: use bash if available
if (Get-Command bash -ErrorAction SilentlyContinue) {
  bash scripts/openapi_contract_guard.sh $Root
} else {
  Write-Host "bash not available; skipping openapi_contract_guard"
}
