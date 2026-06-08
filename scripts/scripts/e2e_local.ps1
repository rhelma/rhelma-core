param(
  [Parameter(Mandatory=$false)][string]$Root = "."
)
& (Join-Path $PSScriptRoot "e2e\e2e_local.ps1") -Root $Root
