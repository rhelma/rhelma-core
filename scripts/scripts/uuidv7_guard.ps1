param(
  [Parameter(Mandatory=$false)][string]$Root = "."
)
& (Join-Path $PSScriptRoot "guards\uuidv7_guard.ps1") -Root $Root
