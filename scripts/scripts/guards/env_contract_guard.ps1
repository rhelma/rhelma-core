param(
  [string]$Root="."
)

$ErrorActionPreference = "Stop"

# Enforce env/region contract primarily at *app surfaces*.
$scanRoots = @('apps','observability')

# Forbidden: direct reads of env/region from process env (string literal forms).
$callPatterns = @(
  'env::var\("RHELMA_ENV"\)',
  'env::var\("RHELMA_ENVIRONMENT"\)',
  'env::var\("RHELMA_REGION"\)',
  'env::var\("RHELMA_ENV_NAME"\)',
  'std::env::var\("RHELMA_ENV"\)',
  'std::env::var\("RHELMA_ENVIRONMENT"\)',
  'std::env::var\("RHELMA_REGION"\)',
  'std::env::var\("RHELMA_ENV_NAME"\)',
  'env::var_os\("RHELMA_ENV"\)',
  'env::var_os\("RHELMA_ENVIRONMENT"\)',
  'env::var_os\("RHELMA_REGION"\)',
  'env::var_os\("RHELMA_ENV_NAME"\)',
  'std::env::var_os\("RHELMA_ENV"\)',
  'std::env::var_os\("RHELMA_ENVIRONMENT"\)',
  'std::env::var_os\("RHELMA_REGION"\)',
  'std::env::var_os\("RHELMA_ENV_NAME"\)',
  '\.var\("RHELMA_ENV"\)',
  '\.var\("RHELMA_ENVIRONMENT"\)',
  '\.var\("RHELMA_REGION"\)'
)

# Close const-key loophole: forbid defining these keys as const/static in app code.
$constPatterns = @(
  '\bconst\s+\w+\s*:\s*&?\s*str\s*=\s*"RHELMA_ENV"',
  '\bconst\s+\w+\s*:\s*&?\s*str\s*=\s*"RHELMA_ENVIRONMENT"',
  '\bconst\s+\w+\s*:\s*&?\s*str\s*=\s*"RHELMA_REGION"',
  '\bconst\s+\w+\s*:\s*&?\s*str\s*=\s*"RHELMA_ENV_NAME"',
  '\bstatic\s+\w+\s*:\s*&?\s*str\s*=\s*"RHELMA_ENV"',
  '\bstatic\s+\w+\s*:\s*&?\s*str\s*=\s*"RHELMA_ENVIRONMENT"',
  '\bstatic\s+\w+\s*:\s*&?\s*str\s*=\s*"RHELMA_REGION"',
  '\bstatic\s+\w+\s*:\s*&?\s*str\s*=\s*"RHELMA_ENV_NAME"'
)

# Allowed hint: central env loader (strict)
$allowedHints = @(
  'CentralEnv::from_env_strict'
)

# Skip docs/tests/examples/migrations
$allowPathRe = '(tests[\/]|examples[\/]|\.md$|migrations[\/]|target[\/]|node_modules[\/]|\.git[\/])'

$rootResolved = (Resolve-Path $Root).Path
$allowlistFile = Join-Path $rootResolved "scripts\guards\env_contract_guard_allowlist.txt"
$allowlistedPaths = @()
if (Test-Path $allowlistFile) {
  $allowlistedPaths = (Get-Content $allowlistFile | Where-Object { $_ -and $_.Trim().Length -gt 0 })
}

function Is-Allowlisted {
  param([string]$Text)
  foreach ($p in $allowlistedPaths) {
    if ($Text -like "*${p}*") { return $true }
  }
  return $false
}

function Get-RustFiles {
  param([string]$Root,[string[]]$Roots)
  $files = @()
  foreach ($r in $Roots) {
    $full = Join-Path $Root $r
    if (-not (Test-Path $full)) { continue }
    $files += Get-ChildItem -Path $full -Recurse -File -ErrorAction SilentlyContinue |
      Where-Object {
        $_.FullName -match '\.rs$' -and
        $_.FullName -notmatch "\\(target|node_modules|\.git)\\" -and
        $_.FullName -notmatch $allowPathRe -and
        -not (Is-Allowlisted -Text $_.FullName)
      }
  }
  return $files
}

function Find-RegexMatches {
  param(
    [Parameter(Mandatory=$true)][string]$Regex,
    [Parameter(Mandatory=$true)][string]$Root,
    [Parameter(Mandatory=$true)][string[]]$Roots
  )

  if (Get-Command rg -ErrorAction SilentlyContinue) {
    $args = @('-n','--hidden','--no-ignore-vcs','--glob','*.rs')
    $args += $Regex
    $args += $Roots
    Push-Location $Root
    try {
      $m = & rg @args 2>$null
      if ($LASTEXITCODE -ne 0) { return @() }
      return ($m | Where-Object { $_ -notmatch $allowPathRe -and -not (Is-Allowlisted -Text $_) })
    } finally {
      Pop-Location
    }
  }

  $files = Get-RustFiles -Root $Root -Roots $Roots
  if (-not $files) { return @() }

  $out = @()
  foreach ($f in $files) {
    $m = Select-String -Path $f.FullName -Pattern $Regex -ErrorAction SilentlyContinue
    if ($m) { $out += $m }
  }
  return $out
}

$hits = $false

foreach ($p in $callPatterns) {
  $m = Find-RegexMatches -Regex $p -Root $rootResolved -Roots $scanRoots
  if ($m -and $m.Count -gt 0) {
    Write-Host "----"
    Write-Host "Found disallowed env access pattern: $p"
    $m | ForEach-Object { Write-Output $_ }
    $hits = $true
  }
}

foreach ($p in $constPatterns) {
  $m = Find-RegexMatches -Regex $p -Root $rootResolved -Roots $scanRoots
  if ($m -and $m.Count -gt 0) {
    Write-Host "----"
    Write-Host "Found disallowed env key const/static (const-key loophole): $p"
    $m | ForEach-Object { Write-Output $_ }
    $hits = $true
  }
}

if ($hits) {
  throw "Direct/indirect access to RHELMA_ENV/RHELMA_REGION is forbidden in app surfaces. Use CentralEnv::from_env_strict()."
}

$foundAllowed = $false
foreach ($a in $allowedHints) {
  $m = Find-RegexMatches -Regex $a -Root $rootResolved -Roots $scanRoots
  if ($m -and $m.Count -gt 0) { $foundAllowed = $true; break }
}

if (-not $foundAllowed) {
  Write-Host "⚠️  Warning: No CentralEnv::from_env_strict found in app surfaces. Are configs migrated?"
}

Write-Host "✅ env_contract_guard: OK"
