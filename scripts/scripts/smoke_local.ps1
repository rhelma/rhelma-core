param(
  [Parameter(Mandatory=$false)][string]$Root = "."
)
& (Join-Path $PSScriptRoot "smoke\smoke_local.ps1") -Root $Root
