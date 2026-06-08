param(
  [Parameter(Mandatory=$false)][string]$Root = "."
)
& (Join-Path $PSScriptRoot "setup\generate-keys.ps1")
