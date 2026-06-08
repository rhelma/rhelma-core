$ErrorActionPreference = "Stop"

$root = if ($args.Length -gt 0) { $args[0] } else { "." }
$guard = Join-Path (Split-Path -Parent $MyInvocation.MyCommand.Path) "guards/header_contract_guard.ps1"

powershell -NoProfile -ExecutionPolicy Bypass -File $guard -Root $root
exit $LASTEXITCODE
