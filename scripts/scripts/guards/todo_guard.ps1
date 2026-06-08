Param(
  [string]$Root = (Resolve-Path (Join-Path $PSScriptRoot "..\.."))
)

$ErrorActionPreference = "Stop"

$allowlistFile = Join-Path $Root ".todo-allowlist"

# Scan only code-bearing directories (exclude docs/ and scripts/).
$searchDirs = @(
  Join-Path $Root "apps",
  Join-Path $Root "crates",
  Join-Path $Root "observability",
  Join-Path $Root "extras",
  Join-Path $Root "infra",
) | Where-Object { Test-Path $_ }

$excludeDirs = @("target", ".git", "node_modules")
$excludeFiles = @("Cargo.lock", "package-lock.json", "yarn.lock")

$matches = @()

foreach ($dir in $searchDirs) {
  Get-ChildItem -Recurse -File -Path $dir -ErrorAction SilentlyContinue | ForEach-Object {
    $full = $_.FullName
    foreach ($d in $excludeDirs) {
      if ($full -like "*$([IO.Path]::DirectorySeparatorChar)$d$([IO.Path]::DirectorySeparatorChar)*") { return }
    }
    if ($excludeFiles -contains $_.Name) { return }

    $content = Get-Content -LiteralPath $full -ErrorAction SilentlyContinue
    for ($i = 0; $i -lt $content.Count; $i++) {
      if ($content[$i] -match "\b(TODO|FIXME|HACK)\b") {
        $matches += "{0}:{1}:{2}" -f $full, ($i + 1), $content[$i]
      }
    }
  }
}

if ($matches.Count -eq 0) {
  Write-Host "todo_guard: OK (no TODO/FIXME/HACK found in code dirs)"
  exit 0
}

$patterns = @()
if (Test-Path $allowlistFile) {
  $patterns = Get-Content -LiteralPath $allowlistFile | Where-Object { $_ -and ($_ -notmatch '^[\s]*#') }
}

$filtered = @()
if ($patterns.Count -eq 0) {
  $filtered = $matches
} else {
  foreach ($m in $matches) {
    $allowed = $false
    foreach ($p in $patterns) {
      if ($m -match $p) { $allowed = $true; break }
    }
    if (-not $allowed) { $filtered += $m }
  }
}

if ($filtered.Count -eq 0) {
  Write-Host "todo_guard: OK (matches are allowlisted)"
  exit 0
}

Write-Host "todo_guard: FAIL — found TODO/FIXME/HACK that must be resolved or allowlisted:" -ForegroundColor Red
$filtered | ForEach-Object { Write-Host $_ }
exit 1
