param(
  [string]$RepoRoot = $(Resolve-Path (Join-Path $PSScriptRoot ".."))
)

$ErrorActionPreference = "Stop"

function Run-Step([string]$Name, [scriptblock]$Cmd) {
  Write-Host "\n=== $Name ==="
  Push-Location $RepoRoot
  try { & $Cmd } finally { Pop-Location }
}

Run-Step "Pre-frontend verification" {
  powershell -NoProfile -ExecutionPolicy Bypass -File "$RepoRoot\scripts\verify_pre_frontend.ps1"
}

Run-Step "Rust: multi-frontend (cargo check)" {
  cargo check -p multi-frontend
}

$webPkg = Join-Path $RepoRoot "apps\web\package.json"
$npm = Get-Command npm -ErrorAction SilentlyContinue
if ($npm -and (Test-Path $webPkg)) {
  Run-Step "Web: install deps (if needed)" {
    Push-Location (Join-Path $RepoRoot "apps\web")
    try { npm install } finally { Pop-Location }
  }
  Run-Step "Web: lint (if present)" {
    Push-Location (Join-Path $RepoRoot "apps\web")
    try { npm run lint --if-present } finally { Pop-Location }
  }
  Run-Step "Web: build (if present)" {
    Push-Location (Join-Path $RepoRoot "apps\web")
    try { npm run build --if-present } finally { Pop-Location }
  }
} else {
  Write-Host "\n[skip] npm/apps\web not available; skipping web checks"
}

Write-Host "\nverify_frontends: OK"
