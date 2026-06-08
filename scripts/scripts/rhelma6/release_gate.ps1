Param(
  [string]$ReportPath = "benchmarks/out/release_gate_report.md",
  [int]$TimeoutSec = 2,
  [string]$ApiGatewayUrl = "http://127.0.0.1:3000",
  [string]$AiOrchUrl = "http://127.0.0.1:4000",
  [string]$NodeRegistryUrl = "http://127.0.0.1:8090",
  [string]$K6BaseUrl = "",
  [switch]$SkipSmoke,
  [switch]$SkipLoad
)

$ErrorActionPreference = "Stop"

function UtcNowIso() {
  return (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
}

function Write-MdLine([string]$Line) {
  Add-Content -Path $ReportPath -Value $Line
}

function Get-GitInfoLine() {
  try {
    if (Test-Path ".git") {
      $branch = (git rev-parse --abbrev-ref HEAD 2>$null)
      $describe = (git describe --always --dirty --tags 2>$null)
      if (-not $describe) { $describe = (git rev-parse --short HEAD 2>$null) }
      return "- Git: $branch @ $describe"
    }
  } catch {}
  return "- Git: ⏭️ (repo not a git checkout)"
}

function Get-ToolLine([string]$Name, [scriptblock]$Cmd) {
  try {
    $c = Get-Command $Name -ErrorAction SilentlyContinue
    if ($null -ne $c) {
      $out = & $Cmd 2>$null | Select-Object -First 1
      if (-not $out) { $out = "(unknown)" }
      return "- $Name: $out"
    }
  } catch {}
  return "- $Name: ⏭️ (not found)"
}

function Invoke-Step([string]$Name, [scriptblock]$Action) {
  Write-MdLine ""
  Write-MdLine "## $Name"
  Write-MdLine ""
  $start = UtcNowIso
  $startS = [int][double]::Parse((Get-Date -Date (Get-Date).ToUniversalTime() -UFormat %s))
  Write-MdLine "- Start: $start"
  $log = Join-Path (Split-Path $ReportPath) ("release_gate_" + ($Name -replace "[^a-zA-Z0-9]+", "_") + ".log")
  $rc = 0
  try {
    & $Action 2>&1 | Tee-Object -FilePath $log | Out-Null
  } catch {
    $rc = 1
    $_ | Out-String | Add-Content -Path $log
  }
  $end = UtcNowIso
  $endS = [int][double]::Parse((Get-Date -Date (Get-Date).ToUniversalTime() -UFormat %s))
  $dur = $endS - $startS
  Write-MdLine "- End: $end"
  Write-MdLine "- Duration: ${dur}s"
  if ($rc -eq 0) {
    Write-MdLine "- Result: ✅ PASS"
  } else {
    Write-MdLine "- Result: ❌ FAIL"
  }
  Write-MdLine ""
  Write-MdLine "<details><summary>Output (tail)</summary>"
  Write-MdLine ""
  Write-MdLine "```"
  Get-Content -Path $log -Tail 200 | ForEach-Object { $_ -replace "```", "`\`\`" } | Add-Content -Path $ReportPath
  Write-MdLine "```"
  Write-MdLine ""
  Write-MdLine "</details>"
  return $rc
}

$outDir = Split-Path $ReportPath
if (-not (Test-Path $outDir)) {
  New-Item -ItemType Directory -Path $outDir | Out-Null
}

Set-Content -Path $ReportPath -Value "# Rhelma6 Release Gate Report`n"

Write-MdLine ""
Write-MdLine "## Decision block"
Write-MdLine ""
Write-MdLine "> **GO / NO-GO** (fill when you promote)"
Write-MdLine "> - Verify: ⬜ PASS / ⬜ FAIL"
Write-MdLine "> - Smoke: ⬜ PASS / ⬜ FAIL"
Write-MdLine "> - Load: ⬜ PASS / ⬜ FAIL / ⬜ SKIP"
Write-MdLine "> - OTEL verify: ⬜ PASS / ⬜ FAIL / ⬜ SKIP (optional)"
Write-MdLine "> - Change approved: ⬜"
Write-MdLine "> - Rollback plan reviewed: ⬜"
Write-MdLine "> - Decision: ⬜ GO / ⬜ NO-GO"
Write-MdLine "> - Approver: ________  Time: ________"
Write-MdLine ""
Write-MdLine "## Context"
Write-MdLine ""
Write-MdLine "- Generated: $(UtcNowIso)"
Write-MdLine "- Host: $([System.Environment]::OSVersion.VersionString)"
Write-MdLine (Get-GitInfoLine)
Write-MdLine ""
Write-MdLine "## Tooling"
Write-MdLine ""
Write-MdLine (Get-ToolLine "rustc" { rustc --version })
Write-MdLine (Get-ToolLine "cargo" { cargo --version })
Write-MdLine (Get-ToolLine "k6" { k6 version })
Write-MdLine (Get-ToolLine "docker" { docker --version })
Write-MdLine (Get-ToolLine "kubectl" { kubectl version --client --short })
Write-MdLine ""
Write-MdLine "## What this gate checks"
Write-MdLine ""
Write-MdLine "Required:"
Write-MdLine "- `scripts/verify.*` (format/lint/tests + repo guards)"
Write-MdLine "- `scripts/rhelma6/smoke_core.*` (fast health checks for critical services)"
Write-MdLine ""
Write-MdLine "Optional:"
Write-MdLine "- Quick k6 load signal (only when `k6` is available)"
Write-MdLine ""
Write-MdLine "Helpful next steps:"
Write-MdLine "- `docs/runbooks/rollout_canary_rollback.md`"
Write-MdLine "- `docs/runbooks/incident_response.md`"
Write-MdLine "- `docs/runbooks/regional_failover.md`"

$overall = 0
$requiredIncomplete = 0
$otelEnabled = ($env:RHELMA_RELEASE_GATE_OTEL_VERIFY -eq "1")
$otelState = "SKIP"
$otelRc = 0
$verifyRc = Invoke-Step "Verify" { $env:RHELMA_VERIFY_OTEL="0"; & ./scripts/verify.ps1 }
if ($verifyRc -ne 0) { $overall = 1 }

if ($otelEnabled) {
  if (Test-Path "./scripts/verify_otel.ps1") {
    $otelRc = Invoke-Step "OTEL verify" { $env:RHELMA_VERIFY_OTEL="1"; & ./scripts/verify_otel.ps1 }
    if ($otelRc -ne 0) { $otelState = "FAIL"; $overall = 1 } else { $otelState = "PASS" }
  } else {
    $otelState = "SKIP"
    Write-MdLine ""
    Write-MdLine "## OTEL verify"
    Write-MdLine ""
    Write-MdLine "- Result: ⏭️ SKIP (scripts/verify_otel.ps1 not found)"
  }
} else {
  $otelState = "SKIP"
  Write-MdLine ""
  Write-MdLine "## OTEL verify"
  Write-MdLine ""
  Write-MdLine "- Result: ⏭️ SKIP (disabled; set RHELMA_RELEASE_GATE_OTEL_VERIFY=1 to enable)"
}

$skipSmoke = $SkipSmoke -or ($env:RHELMA_RELEASE_GATE_SKIP_SMOKE -eq "1")
$skipLoad = $SkipLoad -or ($env:RHELMA_RELEASE_GATE_SKIP_LOAD -eq "1")

$smokeState = "PASS"
$smokeRc = 0

if ($skipSmoke) {
  $smokeState = "SKIP"
  $requiredIncomplete = 1
  Write-MdLine ""
  Write-MdLine "## Smoke (core)"
  Write-MdLine ""
  Write-MdLine "- Result: ⏭️ SKIP (disabled by RHELMA_RELEASE_GATE_SKIP_SMOKE=1 or -SkipSmoke)"
  Write-MdLine ""
  Write-MdLine "Provide RHELMA_SMOKE_* base URLs and re-run without skipping to enable."
} else {
  $smokeRc = Invoke-Step "Smoke (core)" { & ./scripts/rhelma6/smoke_core.ps1 -TimeoutSec $TimeoutSec -ApiGatewayUrl $ApiGatewayUrl -AiOrchUrl $AiOrchUrl -NodeRegistryUrl $NodeRegistryUrl }
  if ($smokeRc -ne 0) { $smokeState = "FAIL"; $overall = 1 } else { $smokeState = "PASS" }
}

$loadState = "PASS"

if ($skipLoad) {
  $loadState = "SKIP"
  Write-MdLine ""
  Write-MdLine "## Load (k6 quick)"
  Write-MdLine ""
  Write-MdLine "- Result: ⏭️ SKIP (disabled by RHELMA_RELEASE_GATE_SKIP_LOAD=1 or -SkipLoad)"
} else {
  $k6 = Get-Command k6 -ErrorAction SilentlyContinue
  if ($null -ne $k6 -and $K6BaseUrl -ne "") {
    $loadRc = Invoke-Step "Load (k6 quick)" { $env:RHELMA6_BASE = $K6BaseUrl; k6 run ./scripts/rhelma6/load/k6_smoke.js }
    if ($loadRc -ne 0) { $loadState = "FAIL"; $overall = 1 }
  } else {
    $loadState = "SKIP"
    Write-MdLine ""
    Write-MdLine "## Load (k6 quick)"
    Write-MdLine ""
    Write-MdLine "- Result: ⏭️ SKIP (k6 not found or K6BaseUrl not set)"
  }
}

Write-MdLine ""
Write-MdLine "# Summary"
Write-MdLine ""
Write-MdLine "| Gate | Required | Result |"
Write-MdLine "|---|---:|---|"
Write-MdLine "| Verify | ✅ | " + ($(if ($verifyRc -eq 0) { "✅ PASS" } else { "❌ FAIL" })) + " |"
Write-MdLine "| OTEL verify | Optional | " + ($(if ($otelState -eq "PASS") { "✅ PASS" } elseif ($otelState -eq "FAIL") { "❌ FAIL" } else { "⏭️ SKIP" })) + " |"
Write-MdLine "| Smoke (core) | ✅ (can be skipped) | " + ($(if ($smokeState -eq "PASS") { "✅ PASS" } elseif ($smokeState -eq "FAIL") { "❌ FAIL" } else { "⏭️ SKIP" })) + " |"
Write-MdLine "| Load (k6 quick) | Optional | " + ($(if ($loadState -eq "PASS") { "✅ PASS" } elseif ($loadState -eq "FAIL") { "❌ FAIL" } else { "⏭️ SKIP" })) + " |"
Write-MdLine ""
if ($overall -eq 0 -and $requiredIncomplete -eq 0) {
  Write-MdLine "✅ **Recommendation:** GO (all required gates passed)."
} elseif ($overall -eq 0 -and $requiredIncomplete -ne 0) {
  Write-MdLine "⚠️ **Recommendation:** INCOMPLETE (a required gate was skipped by configuration)."
} else {
  Write-MdLine "❌ **Recommendation:** NO-GO (one or more required gates failed)."
}
Write-MdLine ""
# Emit machine-readable manifest + PR-ready snippets
$manifestPath = Join-Path $outDir "release_gate_manifest.json"
$prCommentPath = Join-Path $outDir "release_gate_pr_comment.md"
$goNoGoPath = Join-Path $outDir "release_gate_go_no_go_block.md"

function Get-Sha256([string]$Path) {
  try { return (Get-FileHash -Algorithm SHA256 -Path $Path).Hash.ToLower() } catch { return "" }
}

$reportSha = Get-Sha256 $ReportPath
$logs = Get-ChildItem -Path $outDir -Filter "release_gate_*.log" -ErrorAction SilentlyContinue
$artifacts = @()
$artifacts += @{ path = ("benchmarks/out/" + (Split-Path $ReportPath -Leaf)); sha256 = $reportSha }
foreach ($l in $logs) {
  $artifacts += @{ path = ("benchmarks/out/" + $l.Name); sha256 = (Get-Sha256 $l.FullName) }
}

$gitInfo = $null
try {
  if (Test-Path ".git") {
    $branch = (git rev-parse --abbrev-ref HEAD 2>$null)
    $describe = (git describe --always --dirty --tags 2>$null)
    if (-not $describe) { $describe = (git rev-parse --short HEAD 2>$null) }
    $gitInfo = @{ branch = $branch; describe = $describe }
  }
} catch {}

$manifestObj = @{
  generated_at = (UtcNowIso)
  required_incomplete = $requiredIncomplete
  git = $gitInfo
  results = @{
    overall = $overall
    verify = @{ exit_code = $verifyRc; state = $(if ($verifyRc -eq 0) { "PASS" } else { "FAIL" }) }
    otel_verify = @{ exit_code = $otelRc; state = $otelState }
    smoke_core = @{ exit_code = $smokeRc; state = $smokeState }
    load_k6_quick = @{ state = $loadState }
  }
  artifacts = $artifacts
}
$manifestJson = $manifestObj | ConvertTo-Json -Depth 6
Set-Content -Path $manifestPath -Value $manifestJson

$overallWord = $(if ($overall -eq 0 -and $requiredIncomplete -eq 0) { "GO" } elseif ($overall -eq 0 -and $requiredIncomplete -ne 0) { "INCOMPLETE" } else { "NO-GO" })
$overallIcon = $(if ($overall -eq 0 -and $requiredIncomplete -eq 0) { "✅" } elseif ($overall -eq 0 -and $requiredIncomplete -ne 0) { "⚠️" } else { "❌" })
$vMark = $(if ($verifyRc -eq 0) { "[x]" } else { "[ ]" })
$oMark = $(if ($otelState -eq "PASS") { "[x]" } else { "[ ]" })
$sMark = $(if ($smokeState -eq "PASS") { "[x]" } else { "[ ]" })
$lMark = $(if ($loadState -eq "PASS") { "[x]" } else { "[ ]" })

$goNoGo = @"
## GO/NO-GO (paste into release ticket)

- $vMark Verify PASS
- $oMark OTEL verify PASS (optional)
- $sMark Smoke (core) PASS
- $lMark Load (k6 quick) PASS (optional)
- [ ] Change approved
- [ ] Rollback plan reviewed
- [ ] Decision: GO / NO-GO
- Approver: ________  Time: ________
"@
Set-Content -Path $goNoGoPath -Value $goNoGo

$prComment = @"
<!-- Rhelma6 Release Gate (autogenerated) -->
## Rhelma6 Release Gate — $overallIcon **$overallWord**

**Gates**
- Verify: $(if ($verifyRc -eq 0) { '✅ PASS' } else { '❌ FAIL' })
- OTEL verify: $(if ($otelState -eq 'PASS') { '✅ PASS' } elseif ($otelState -eq 'FAIL') { '❌ FAIL' } else { '⏭️ SKIP' })
- Smoke (core): $(if ($smokeState -eq 'PASS') { '✅ PASS' } elseif ($smokeState -eq 'FAIL') { '❌ FAIL' } else { '⏭️ SKIP' })
- Load (k6 quick): $(if ($loadState -eq 'PASS') { '✅ PASS' } elseif ($loadState -eq 'FAIL') { '❌ FAIL' } else { '⏭️ SKIP' })

**Artifacts (repo paths)**
- `benchmarks/out/release_gate_report.md`
- `benchmarks/out/release_gate_manifest.json`
- `benchmarks/out/release_gate_pr_comment.md` (this file)
- `benchmarks/out/release_gate_go_no_go_block.md`

**Go/No-Go checklist**
- $vMark Verify PASS
- $oMark OTEL verify PASS (optional)
- $sMark Smoke (core) PASS
- $lMark Load (k6 quick) PASS (optional)
- [ ] Change approved
- [ ] Rollback plan reviewed
- [ ] Decision: GO / NO-GO
- Approver: ________  Time: ________

<details><summary>Release notes / next actions</summary>

- If **NO-GO**: follow `docs/runbooks/rollout_canary_rollback.md` and `docs/runbooks/incident_response.md`.
- If this is a multi-region change: review `docs/runbooks/regional_failover.md`.

</details>
"@
Set-Content -Path $prCommentPath -Value $prComment

Write-Host "Report written to: $ReportPath"

exit $overall
