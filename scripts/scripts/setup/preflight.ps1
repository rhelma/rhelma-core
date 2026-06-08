$ErrorActionPreference = "Stop"

param(
  [switch]$Strict
)

function Ok($msg) { Write-Host "✅ $msg" }
function Warn($msg) { Write-Host "⚠️  $msg" -ForegroundColor Yellow }
function Die($msg) { Write-Host "❌ $msg" -ForegroundColor Red; exit 127 }

function Must(
  [Parameter(Mandatory=$true)][string]$Cmd,
  [Parameter(Mandatory=$true)][string]$Hint
) {
  if (-not (Get-Command $Cmd -ErrorAction SilentlyContinue)) {
    Die "Required command '$Cmd' not found. $Hint"
  }
  Ok $Cmd
}

function Maybe(
  [Parameter(Mandatory=$true)][string]$Cmd,
  [Parameter(Mandatory=$true)][string]$Hint
) {
  if (Get-Command $Cmd -ErrorAction SilentlyContinue) {
    Ok $Cmd
    return
  }

  if ($Strict) {
    Die "Missing recommended command '$Cmd'. $Hint"
  }
  Warn "Missing recommended command '$Cmd'. $Hint"
}

Write-Host "== Rhelma preflight =="

Must git "Install Git and retry."
Must cargo "Install Rust toolchain (rustup recommended) and retry."

Maybe rustfmt "Run: rustup component add rustfmt"
Maybe clippy "Run: rustup component add clippy"

Maybe docker "Needed for docker-compose based dev stacks."
Maybe openssl "Needed for scripts/setup/generate-keys.(sh|ps1)."
Maybe node "Needed for Svelte frontends under apps/(admin-web|web)."
Maybe npm "Needed for Svelte frontends under apps/(admin-web|web)."

Maybe rg "Optional (faster scans). Scripts fall back to Select-String / grep."

Write-Host "preflight: OK"
