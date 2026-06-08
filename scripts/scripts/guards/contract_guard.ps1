param(
  [Parameter(Mandatory=$false)][string]$Root = ".",
  [Parameter(Mandatory=$false)][int]$TimeoutSec = 180
)
Write-Host "[contract_guard] scanning for legacy HTTP headers under: $Root"

# NOTE: We intentionally keep these as fixed-string patterns.
# The scan excludes build artifacts and guard scripts themselves.
$patterns = @(
  '"x-request-id"', 'x-request-id',
  '"x-correlation-id"', 'x-correlation-id',
  '"x-trace-id"', 'x-trace-id',
  '"x-span-id"', 'x-span-id',
  '"x-rhelma-trace-id"', 'x-rhelma-trace-id',
  '"x-rhelma-span-id"', 'x-rhelma-span-id'
)

# Paths we allow to *mention* legacy headers (docs, tests, internal libs, tooling).
# IMPORTANT: ripgrep output on Windows uses backslashes; match both '/' and '\\'.
$AllowRe = '(README|CHANGELOG|docs[\\/]|\\.md$|migrations[\\/]|tests[\\/]|crates[\\/]|extras[\\/]|scripts[\\/]|target[\\/]|\\.cargo[\\/]|\\.git[\\/]|node_modules[\\/])'

$rgExcludeArgs = @(
  '--glob', '!.git/**',
  '--glob', '!target/**',
  '--glob', '!.cargo/**',
  '--glob', '!node_modules/**',
  '--glob', '!scripts/guards/**'
)

function Find-Matches {
  param([string]$Pattern)

  $rg = Get-Command rg -ErrorAction SilentlyContinue
  if ($rg) {
    $m = & rg -n --fixed-strings --hidden --no-ignore-vcs @rgExcludeArgs $Pattern $Root 2>$null
    if ($LASTEXITCODE -ne 0 -or -not $m) { return @() }
    return ($m | rg -v $AllowRe)
  }
# Fallback (no ripgrep): PowerShell search (slower). Best-effort exclusions.
# IMPORTANT: In fallback mode we may see absolute paths (e.g. D:\repo\target\...). So skip dirs
# by matching path segments anywhere, not just at the beginning.
$rootFull = (Resolve-Path $Root).Path
$skipRe = '([\\/])(\.git|target|\.cargo|node_modules)([\\/]|$)'

# Narrow the scan to text-like files to avoid binary/huge artifacts slowing the guard.
$allowedExt = @(
  '.rs','.toml','.yaml','.yml','.json',
  '.ts','.tsx','.js','.jsx',
  '.py','.go','.java','.kt','.cs',
  '.sh','.ps1','.psm1',
  '.md','.txt','.proto','.sql',
  '.env','.cfg','.ini','.conf','.lock'
)
$specialNames = @('Dockerfile','Makefile','Justfile')

$all = Get-ChildItem -Path $rootFull -Recurse -File -Force -ErrorAction SilentlyContinue |
  Where-Object {
    $_.FullName -notmatch $skipRe -and (
      $allowedExt -contains $_.Extension.ToLowerInvariant() -or
      $specialNames -contains $_.Name
    )
  }

$sw = [Diagnostics.Stopwatch]::StartNew()
$hits = @()
foreach ($f in $all) {
  if ($TimeoutSec -gt 0 -and $sw.Elapsed.TotalSeconds -gt $TimeoutSec) {
    Write-Host "[contract_guard] TIMEOUT after $TimeoutSec sec (fallback mode). Install ripgrep (rg) for fast scans." -ForegroundColor Yellow
    exit 2
  }

  $ms = Select-String -Path $f.FullName -SimpleMatch -Pattern $Pattern -ErrorAction SilentlyContinue
  if ($ms) {
    foreach ($h in $ms) {
      $line = "{0}:{1}:{2}" -f $h.Path, $h.LineNumber, $h.Line
      if ($line -notmatch $AllowRe) { $hits += $line }
    }
  }
}
return $hits
}

$violations = @()
foreach ($p in $patterns) {
  $found = Find-Matches -Pattern $p
  if ($found.Count -gt 0) {
    $violations += @{
      pattern = $p
      matches = $found
    }
  }
}

if ($violations.Count -gt 0) {
  foreach ($v in $violations) {
    Write-Host "[contract_guard] FAIL: found legacy header pattern: $($v.pattern)" -ForegroundColor Red
    $v.matches | ForEach-Object { Write-Host $_ }
    Write-Host ""
  }
  exit 1
}

Write-Host "[contract_guard] OK" -ForegroundColor Green
exit 0
