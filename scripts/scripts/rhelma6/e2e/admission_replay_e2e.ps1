$ErrorActionPreference = "Stop"
$REGISTRY_URL = $env:REGISTRY_URL
if ([string]::IsNullOrEmpty($REGISTRY_URL)) { $REGISTRY_URL = "http://127.0.0.1:8090" }

Write-Host "[e2e] init node"
cargo run -q -p rhelma-node -- init | Out-Null

Write-Host "[e2e] register once (should succeed)"
cargo run -q -p rhelma-node -- register --registry $REGISTRY_URL | Out-Null

Write-Host "[e2e] replay register (should fail)"
$ok = $true
try {
  cargo run -q -p rhelma-node -- register --registry $REGISTRY_URL --replay-last-admission-proof | Out-Null
} catch {
  $ok = $false
}
if ($ok) {
  throw "FAIL: replay register unexpectedly succeeded"
}
Write-Host "OK: replay rejected"
