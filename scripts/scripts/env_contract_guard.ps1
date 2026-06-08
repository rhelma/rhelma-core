param(
  [Parameter(Mandatory=$false)][string]$Root = "."
)
& (Join-Path $PSScriptRoot "guards\env_contract_guard.ps1") -Root $Root
