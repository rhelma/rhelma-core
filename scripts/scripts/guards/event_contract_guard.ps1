param(
  [Parameter(Mandatory=$false)][string]$Root = "."
)

Write-Host "[event_contract_guard] scanning for unsafe event publishing patterns under: $Root"

# Focus on publish-boundary safety (not on how envelopes are constructed).
# Hard fail:
#   - Passing an EventEnvelope literal directly into publish() without a finalize_* call.
# Soft warning (heuristic):
#   - publish() call without a nearby finalize_* (may be safe if the bus enforces contracts).

$allowPathRe = '(tests[\\/]|examples[\\/]|\.md$|migrations[\\/]|target[\\/]|node_modules[\\/]|\.git[\\/])'

function Get-RustFiles {
  param([string]$Root)
  return Get-ChildItem -Path $Root -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match '\.rs$' -and $_.FullName -notmatch $allowPathRe }
}

$files = Get-RustFiles -Root $Root

# Hard fail: publish(EventEnvelope { ... }) without finalize_* on the same line.
$hardMatches = @()
foreach ($f in $files) {
  $content = Get-Content -Raw -ErrorAction SilentlyContinue $f.FullName
  if (-not $content) { continue }
  if ($content -notmatch 'EventEnvelope') { continue }

  $lines = Get-Content -ErrorAction SilentlyContinue $f.FullName
  for ($i=0; $i -lt $lines.Count; $i++) {
    $line = $lines[$i]
    if ($line -match '\.publish\s*\(\s*EventEnvelope\s*\{') {
      if ($line -match '\.finalize_[A-Za-z0-9_]*\s*\(') { continue }
      $hardMatches += "${($f.FullName)}:$($i+1): $line"
    }
  }
}

if ($hardMatches.Count -gt 0) {
  Write-Host ""
  Write-Host "[event_contract_guard] FAIL: direct publish(EventEnvelope {..}) detected. Build envelope, then call finalize_*() before publish()."
  $hardMatches | ForEach-Object { Write-Output $_ }
  exit 1
}

# Soft warning: publish(...) without nearby finalize_* (heuristic)
$warnMatches = @()
foreach ($f in $files) {
  $content = Get-Content -Raw -ErrorAction SilentlyContinue $f.FullName
  if (-not $content) { continue }
  if ($content -notmatch 'EventEnvelope') { continue }
  if ($content -notmatch '\.publish\s*\(') { continue }

  $lines = Get-Content -ErrorAction SilentlyContinue $f.FullName
  $buffer = New-Object System.Collections.Generic.Queue[string]
  for ($i=0; $i -lt $lines.Count; $i++) {
    $line = $lines[$i]

    # enqueue current line (keep last 10)
    $buffer.Enqueue($line)
    if ($buffer.Count -gt 10) { [void]$buffer.Dequeue() }

    if ($line -match '\.publish\s*\(') {
      if ($line -match '\.finalize_[A-Za-z0-9_]*\s*\(') { continue }
      if ($line -match 'publish_with_observability') { continue }

      $hasFinalize = $false
      foreach ($b in $buffer) {
        if ($b -match '\.finalize_[A-Za-z0-9_]*\s*\(') { $hasFinalize = $true; break }
        if ($b -match 'publish_with_observability') { $hasFinalize = $true; break }
      }

      if (-not $hasFinalize) {
        $warnMatches += "[event_contract_guard] WARN ${($f.FullName)}:$($i+1): $line"
      }
    }
  }
}

if ($warnMatches.Count -gt 0) {
  $warnMatches | ForEach-Object { Write-Output $_ }
  Write-Host "[event_contract_guard] NOTE: some publish() calls were found without a nearby finalize_* (heuristic warnings)."
  Write-Host "[event_contract_guard] If safe (e.g., bus enforces contracts), consider switching to finalize_* before publish() or publish_with_observability()."
}

Write-Host "[event_contract_guard] OK"
