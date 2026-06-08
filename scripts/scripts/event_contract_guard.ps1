param(
  [Parameter(Mandatory=$false)][string]$Root = "."
)
& (Join-Path $PSScriptRoot "guards\event_contract_guard.ps1") -Root $Root
