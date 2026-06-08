param(
  [Parameter(Mandatory=$false)][string]$Root = "."
)

Set-Location $Root

function Fail([string]$msg) {
  Write-Error "❌ $msg"
  exit 1
}

# Legacy headers that must not appear in app surfaces.
$Disallowed = @(
  'x-request-id',
  'x-correlation-id',
  'x-rhelma-trace-id',
  'x-rhelma-span-id'
)

# If RHELMA_GUARDS_STRICT_X_RHELMA=1, enforce that only these x-rhelma-* headers are allowed in apps/.
$AllowedXRhelmaRe = 'x-rhelma-(request-id|correlation-id)'
$StrictXRhelma = $env:RHELMA_GUARDS_STRICT_X_RHELMA
if ([string]::IsNullOrWhiteSpace($StrictXRhelma)) { $StrictXRhelma = '0' }

# Scan only app entrypoints/surfaces.
$ScanRoots = @('apps')

# Skip obvious non-source locations (match both Windows and POSIX path separators).
$SkipRe = '(target[\\/]|node_modules[\\/]|\.git[\\/]|dist[\\/]|build[\\/]|coverage[\\/]|tests[\\/]|fixtures[\\/])'

$AllowlistFile = "scripts/guards/header_contract_guard_allowlist.txt"
$Allowlist = @()
if (Test-Path $AllowlistFile) {
  $Allowlist = Get-Content $AllowlistFile | Where-Object { $_ -and $_.Trim().Length -gt 0 }
}

function Filter-Hits([string[]]$lines) {
  if (-not $lines) { return @() }
  $out = $lines | Where-Object { $_ -notmatch $SkipRe }
  if ($Allowlist.Count -gt 0) {
    foreach ($entry in $Allowlist) {
      $out = $out | Where-Object { $_ -notmatch [Regex]::Escape($entry) }
    }
  }
  return $out
}

function Find-Matches([string]$pattern) {
  if (Get-Command rg -ErrorAction SilentlyContinue) {
    $raw = & rg -n --hidden --no-ignore-vcs --fixed-strings `
      --glob '!target/**' --glob '!node_modules/**' --glob '!.git/**' --glob '!**/*.md' `
      --glob '*.rs' --glob '*.js' --glob '*.ts' `
      $pattern $ScanRoots 2>$null
    if ($LASTEXITCODE -ne 0) { return @() }
    return Filter-Hits $raw
  }

  # Fallback: Select-String
  $all = @()
  foreach ($r in $ScanRoots) {
    if (Test-Path $r) {
      $all += Select-String -Path (Join-Path $r "**\*.rs"), (Join-Path $r "**\*.js"), (Join-Path $r "**\*.ts") -SimpleMatch $pattern -List:$false -ErrorAction SilentlyContinue |
        ForEach-Object { "{0}:{1}:{2}" -f $_.Path, $_.LineNumber, $_.Line.Trim() }
    }
  }
  return Filter-Hits $all
}

$hitsAny = $false
foreach ($ptn in $Disallowed) {
  $hits = Find-Matches $ptn
  if ($hits.Count -gt 0) {
    Write-Host "----"
    Write-Host "Found disallowed legacy header token: $ptn"
    $hits | ForEach-Object { Write-Host $_ }
    $hitsAny = $true
  }
}

if ($hitsAny) {
  Fail "Legacy headers are forbidden in apps/. Use x-rhelma-request-id, x-rhelma-correlation-id, and W3C traceparent."
}

if ($StrictXRhelma -eq '1') {
  $raw = Find-Matches 'x-rhelma-'
  if ($raw.Count -gt 0) {
    $disallowed = $raw | Where-Object { $_ -notmatch $AllowedXRhelmaRe }
    if ($disallowed.Count -gt 0) {
      Write-Host "----"
      Write-Host "Found disallowed x-rhelma-* header token(s) in apps/ (strict mode):"
      $disallowed | ForEach-Object { Write-Host $_ }
      Fail "Disallowed x-rhelma-* headers detected in apps/ (strict mode)."
    }
  }
}

Write-Host "✅ header_contract_guard: OK"
