$ErrorActionPreference="Stop"
$Root=Resolve-Path (Join-Path $PSScriptRoot "..\..")
$Apps=Join-Path $Root "apps"
$Runbooks=Join-Path $Root "docs\runbooks"
$HttpServicesFile=Join-Path $Root "docs\reference\http_services.txt"
$NonDaemonAppsFile=Join-Path $Root "docs\reference\non_daemon_apps.txt"
$HttpServices=@{}
if(Test-Path $HttpServicesFile){
  (Get-Content $HttpServicesFile) | ForEach-Object {
    $line=$_.Trim()
    if($line -and -not $line.StartsWith("#")){
      $HttpServices[$line]=$true
    }
  }
}

$NonDaemon=@{}
if(Test-Path $NonDaemonAppsFile){
  (Get-Content $NonDaemonAppsFile) | ForEach-Object {
    $line=$_.Trim()
    if($line -and -not $line.StartsWith("#")){
      $NonDaemon[$line]=$true
    }
  }
}
$req=@("Overview","Run","Configuration","Endpoints","Observability","Security","Verification")
$gate=($env:RHELMA_VERIFY_COMPLETENESS -eq "1")
$missing=0
$generated=(Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
Write-Host "# Rhelma completeness report`n`nGenerated: $generated`n`nLegend: ✅ ok · ⚠️ partial · ❌ missing`n"
Write-Host "| Service | README | Sections | Health/metrics mentioned | Runbook | Tests dir | OpenAPI |"
Write-Host "|---|---:|---:|---:|---:|---:|---:|"
Get-ChildItem $Apps -Directory | ForEach-Object {
  $svc=$_.Name
  $readme=Join-Path $_.FullName "README.md"
  $r="❌";$s="❌";$hm="❌";$rb="❌";$t="❌";$o="❌"
  if(Test-Path $readme){
    $r="✅";$c=Get-Content $readme -Raw
    $ok=0; foreach($x in $req){ if($c -match "(?im)^##+\\s+.*$x"){ $ok++ } }
    if($ok -ge 6){$s="✅"}elseif($ok -ge 3){$s="⚠️"}else{$s="❌"}
    if($NonDaemon.ContainsKey($svc)){
      $hm="—"
    } else {
      $hasH=($c -match "(?im)healthz|/health")
      $hasM=($c -match "(?im)metrics|/metrics")
      if($hasH -and $hasM){$hm="✅"}elseif($hasH -or $hasM){$hm="⚠️"}else{$hm="❌"}
    }
  } else { $missing++ }
  $rbName=("service_{0}.md" -f ($svc -replace '-', '_'))
  if(Test-Path (Join-Path $Runbooks $rbName)){ $rb="✅" }
  if(Test-Path (Join-Path $_.FullName "tests")){ $t="✅" }
  $openapi=Join-Path $Root ("docs\openapi\{0}.yaml" -f $svc)
  if($HttpServices.ContainsKey($svc)){
    if(Test-Path $openapi){ $o="✅" } else { $o="❌" }
  } else {
    $o="—"
  }
  Write-Host ("| {0} | {1} | {2} | {3} | {4} | {5} | {6} |" -f $svc,$r,$s,$hm,$rb,$t,$o)
  if($gate){ if($r -ne "✅" -or $hm -eq "❌"){ $missing++ } }
}
Write-Host "`nTip: known phased stubs -> .\\scripts\\dev\\stub-report.ps1"
Write-Host "Tip: completeness target -> docs\\reference\\COMPLETENESS_MATRIX.md"
if($gate -and $missing -gt 0){ Write-Error ("Completeness gate failed: {0} issue(s)" -f $missing); exit 1 }
exit 0
