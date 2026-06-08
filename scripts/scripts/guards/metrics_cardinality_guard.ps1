$ErrorActionPreference = "Stop"

# Metrics cardinality guard (Rhelma v5.2)
#
# Fails if any Rust code uses a raw HTTP path (e.g. req.uri().path()) as the metrics
# endpoint label. Best-effort static scan.

$repoRoot = "."
if ($args.Count -ge 1 -and $args[0]) { $repoRoot = $args[0] }

# Find files containing record_http_request
$matches = Get-ChildItem -Path $repoRoot -Recurse -Filter *.rs -ErrorAction SilentlyContinue |
  Where-Object { $_.FullName -notmatch "\\target\\|\\\.git\\" } |
  Select-String -Pattern "record_http_request" |
  Select-Object -ExpandProperty Path -Unique

$fail = $false

foreach ($file in $matches) {
  $content = Get-Content -Raw -Path $file

  if ($content -match "record_http_request(_with_bytes|_with_labels)?\([^,]*,[^,]*(req\.)?uri\(\)\.path\(\)") {
    Write-Host "[FAIL] raw uri().path() used as metrics endpoint label: $file"
    $fail = $true
  }

  if (($content -match "let\s+path\s*=\s*.*uri\(\)\.path\(\)") -and
      ($content -match "record_http_request(_with_bytes|_with_labels)?\([^,]*,[^,]*\bpath\b")) {
    Write-Host "[FAIL] variable 'path' derived from uri().path() used as metrics endpoint label: $file"
    $fail = $true
  }
}

if ($fail) {
  Write-Host "[metrics_cardinality_guard] FAILED"
  exit 1
}

Write-Host "[metrics_cardinality_guard] OK"
