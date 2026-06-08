param(
  [Parameter(Mandatory=$false)][string]$Root = "."
)
& (Join-Path $PSScriptRoot "guards\metrics_cardinality_guard.ps1") -Root $Root
