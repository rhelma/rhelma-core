$ErrorActionPreference = "Stop"

function Get-EnvInt {
  param([Parameter(Mandatory = $true)][string]$Name)
  $v = [Environment]::GetEnvironmentVariable($Name)
  if ([string]::IsNullOrWhiteSpace($v)) { return $null }
  $n = 0
  if ([int]::TryParse($v, [ref]$n)) { return $n }
  return $null
}

function Resolve-VerifyTuning {
  $cpu = [Environment]::ProcessorCount

  # Best-effort RAM detection (Windows). If it fails, fall back to CPU-only heuristics.
  $memGb = $null
  try {
    $cs = Get-CimInstance -ClassName Win32_ComputerSystem -ErrorAction Stop
    if ($null -ne $cs -and $cs.TotalPhysicalMemory -gt 0) {
      $memGb = [double]($cs.TotalPhysicalMemory / 1GB)
    }
  } catch {
    $memGb = $null
  }

  $lowResource = ([Environment]::GetEnvironmentVariable("RHELMA_VERIFY_LOW_RESOURCE") -eq "1")
  $ci = -not [string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable("CI"))

  $jobs = Get-EnvInt "RHELMA_VERIFY_JOBS"
  $jobsFromCargoEnv = $false
  if ($null -eq $jobs) {
    $jobs = Get-EnvInt "CARGO_BUILD_JOBS"
    $jobsFromCargoEnv = ($null -ne $jobs)
  }

  if ($null -eq $jobs) {
    if ($ci) {
      # In CI we prefer speed over interactivity.
      $jobs = [Math]::Min(16, [Math]::Max(1, $cpu))
    } elseif ($null -ne $memGb -and $memGb -lt 8) {
      $jobs = 1
    } elseif ($null -ne $memGb -and $memGb -lt 16) {
      $jobs = [Math]::Min(2, $cpu)
    } else {
      # Leave one core free, cap at 4 to avoid surprise fan+RAM spikes on laptops.
      $jobs = [Math]::Min(4, [Math]::Max(1, $cpu - 1))
    }
  }

  $testThreads = Get-EnvInt "RHELMA_VERIFY_TEST_THREADS"
  $threadsFromRustEnv = $false
  if ($null -eq $testThreads) {
    $testThreads = Get-EnvInt "RUST_TEST_THREADS"
    $threadsFromRustEnv = ($null -ne $testThreads)
  }

  if ($null -eq $testThreads) {
    if ($ci) {
      $testThreads = [Math]::Min(16, $jobs)
    } elseif ($null -ne $memGb -and $memGb -lt 8) {
      $testThreads = 1
    } else {
      $testThreads = [Math]::Min(2, $jobs)
    }
  }

  if ($lowResource) {
    $jobs = 1
    $testThreads = 1
  }

  $jobs = [Math]::Max(1, $jobs)
  $testThreads = [Math]::Max(1, $testThreads)

  # Apply defaults if the caller didn't already pin them.
  if (-not $jobsFromCargoEnv -or $null -ne (Get-EnvInt "RHELMA_VERIFY_JOBS")) {
    $env:CARGO_BUILD_JOBS = "$jobs"
  }
  if (-not $threadsFromRustEnv -or $null -ne (Get-EnvInt "RHELMA_VERIFY_TEST_THREADS")) {
    $env:RUST_TEST_THREADS = "$testThreads"
  }
  if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable("RAYON_NUM_THREADS"))) {
    $env:RAYON_NUM_THREADS = "$testThreads"
  }

  $memMsg = if ($null -ne $memGb) { "{0:N1}GB" -f $memGb } else { "unknown" }
  $ciMsg = if ($ci) { ", CI=1" } else { "" }
  Write-Host "verify: tuning -> CARGO_BUILD_JOBS=$jobs, RUST_TEST_THREADS=$testThreads (CPU=$cpu, RAM=$memMsg$ciMsg)"
  if ($lowResource) {
    Write-Host "verify: RHELMA_VERIFY_LOW_RESOURCE=1 -> forcing low resource mode (jobs=1, threads=1)"
  }

  return @{
    Jobs = $jobs
    TestThreads = $testThreads
  }
}

function Run-Native {
  param(
    [Parameter(Mandatory = $true)][string]$Name,
    [Parameter(Mandatory = $true)][scriptblock]$Block
  )
  & $Block
  if ($LASTEXITCODE -ne 0) {
    throw "$Name failed (exit $LASTEXITCODE)"
  }
}

$tuning = Resolve-VerifyTuning
$jobs = $tuning.Jobs
$testThreads = $tuning.TestThreads

Run-Native -Name "cargo fmt" -Block { cargo fmt --all -- --check }
Run-Native -Name "cargo clippy" -Block { cargo clippy -j $jobs --workspace --all-targets -- -D warnings }
Run-Native -Name "cargo test" -Block { cargo test -j $jobs --workspace -- --test-threads $testThreads }

# Contract & env/event anti-drift gates (best-effort)
$cg = Join-Path (Get-Location) "scripts/contract_guard.ps1"
if (Test-Path $cg) {
  powershell -NoProfile -ExecutionPolicy Bypass -File $cg
  if ($LASTEXITCODE -ne 0) { throw "contract_guard failed (exit $LASTEXITCODE)" }
}

$eg = Join-Path (Get-Location) "scripts/env_contract_guard.ps1"
if (Test-Path $eg) {
  powershell -NoProfile -ExecutionPolicy Bypass -File $eg
  if ($LASTEXITCODE -ne 0) { throw "env_contract_guard failed (exit $LASTEXITCODE)" }
}

$evg = Join-Path (Get-Location) "scripts/event_contract_guard.ps1"
if (Test-Path $evg) {
  powershell -NoProfile -ExecutionPolicy Bypass -File $evg
  if ($LASTEXITCODE -ne 0) { throw "event_contract_guard failed (exit $LASTEXITCODE)" }
}

$hg = Join-Path (Get-Location) "scripts/header_contract_guard.ps1"
if (Test-Path $hg) {
  powershell -NoProfile -ExecutionPolicy Bypass -File $hg
  if ($LASTEXITCODE -ne 0) { throw "header_contract_guard failed (exit $LASTEXITCODE)" }
}

# Observability verification (best-effort; skips missing tests/scripts)
$obs = Join-Path (Get-Location) "scripts/verify_observability.ps1"
if (Test-Path $obs) {
  powershell -NoProfile -ExecutionPolicy Bypass -File $obs -Root "."
  if ($LASTEXITCODE -ne 0) {
    throw "verify_observability failed (exit $LASTEXITCODE)"
  }
}

$deny = Get-Command cargo-deny -ErrorAction SilentlyContinue
if ($null -ne $deny) {
  Run-Native -Name "cargo deny" -Block { cargo deny check }
} else {
  Write-Host "cargo-deny not installed; skipping (install with: cargo install --locked cargo-deny)"
}

# Completeness report (non-blocking by default; gate with RHELMA_VERIFY_COMPLETENESS=1)
$cr = Join-Path (Get-Location) "scripts\dev\completeness-report.ps1"
if (Test-Path $cr) {
  powershell -NoProfile -ExecutionPolicy Bypass -File $cr
  if ($LASTEXITCODE -ne 0) {
    throw "completeness-report failed (exit $LASTEXITCODE)"
  }
}


# OpenAPI guards (best-effort)
if (Test-Path "scripts/openapi_contract_guard.ps1") { powershell -NoProfile -ExecutionPolicy Bypass -File scripts/openapi_contract_guard.ps1 -Root "." }
if (Test-Path "scripts/openapi_drift_guard.ps1") { powershell -NoProfile -ExecutionPolicy Bypass -File scripts/openapi_drift_guard.ps1 -Root "." }
if (Test-Path "scripts/openapi_examples_guard.ps1") { powershell -NoProfile -ExecutionPolicy Bypass -File scripts/openapi_examples_guard.ps1 -Root "." }
