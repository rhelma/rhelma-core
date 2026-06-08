param(
  [Parameter(Mandatory=$false)][string]$Root = "."
)
& (Join-Path $PSScriptRoot "smoke\smoke_staging.ps1") -Root $Root
