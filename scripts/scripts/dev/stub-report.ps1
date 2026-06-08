$ErrorActionPreference = "Stop"

Write-Host "== Rhelma stub report =="

$roots = @("apps", "crates", "observability")
$pattern = "stub|TODO:.*stub|intentionally a stub|stubs / opt-in wiring"

foreach ($r in $roots) {
  if (Test-Path $r) {
    Get-ChildItem -Path $r -Recurse -File -Filter "*.rs" |
      Where-Object { $_.FullName -notmatch "[\\/]target[\\/]" } |
      ForEach-Object {
        $matches = Select-String -Path $_.FullName -Pattern $pattern -AllMatches -ErrorAction SilentlyContinue
        if ($matches) { $matches }
      }
  }
}

Write-Host ""
Write-Host "Tip: see docs\reference\KNOWN_STUBS_AND_PHASED_WIRING.md"
