param(
  [Parameter(Mandatory=$false)][string]$Root = "."
)

Write-Host "[uuidv7_guard] scanning for UUIDv4 usage near request/correlation/event identifiers under: $Root"

$allowRe = '(tests[\\/]|\.md$|migrations[\\/])'
$fail = $false

$patterns = @(
  'request[_-]?id[^\n]{0,120}Uuid::new_v4\(',
  'correlation[_-]?id[^\n]{0,120}Uuid::new_v4\(',
  'event[_-]?id[^\n]{0,120}Uuid::new_v4\(',
  'generate_(request|correlation|event)_id\([^\)]*\)[^{]*\{[^}]*Uuid::new_v4\('
)

function Find-RegexMatches {
  param(
    [Parameter(Mandatory=$true)][string]$Regex,
    [Parameter(Mandatory=$true)][string]$Root
  )

  if (Get-Command rg -ErrorAction SilentlyContinue) {
    $m = rg -n --hidden --no-ignore-vcs -U $Regex $Root 2>$null
    if ($LASTEXITCODE -ne 0) { return @() }
    return ($m | rg -v $allowRe)
  }

  # Fallback: scan lines containing Uuid::new_v4 and then filter by keywords.
  $files = Get-ChildItem -Path $Root -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -notmatch "\\\\(target|node_modules|\.git)\\\\" -and $_.FullName -match '\\.rs$' }

  $v4 = $files | Select-String -Pattern 'Uuid::new_v4\(' -ErrorAction SilentlyContinue
  return ($v4 | Where-Object { $_.Line -match 'request[_-]?id|correlation[_-]?id|event[_-]?id|generate_(request|correlation|event)_id' -and $_ -notmatch $allowRe })
}

foreach ($p in $patterns) {
  $matches = Find-RegexMatches -Regex $p -Root $Root
  if ($matches -and $matches.Count -gt 0) {
    Write-Host ""
    Write-Host "[uuidv7_guard] FAIL: possible UUIDv4 usage where UUIDv7 is expected: $p"
    Write-Output $matches
    $fail = $true
  }
}

if ($fail) {
  Write-Host ""
  Write-Host "[uuidv7_guard] Expected: Uuid::now_v7() for request_id / correlation_id / event_id"
  exit 1
}

Write-Host "[uuidv7_guard] OK"
