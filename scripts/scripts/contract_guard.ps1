param(
  [Parameter(Mandatory=$false)][string]$Root = ".",
  [Parameter(Mandatory=$false)][int]$TimeoutSec = 180
)
& (Join-Path $PSScriptRoot "guards\contract_guard.ps1") -Root $Root -TimeoutSec $TimeoutSec
