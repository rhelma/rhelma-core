$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

# scripts/dev/register-local-social-node.ps1
# Registers social-service as a node for realm=central in control-service.

function Set-EnvDefault([string]$name, [string]$value) {
  $current = [Environment]::GetEnvironmentVariable($name)
  if ([string]::IsNullOrWhiteSpace($current)) {
    [Environment]::SetEnvironmentVariable($name, $value)
    $env:$name = $value
  }
}

function Import-DotEnv([string]$path) {
  if (-not (Test-Path $path)) { return }
  Get-Content $path | ForEach-Object {
    $line = $_.Trim()
    if ($line.Length -eq 0) { return }
    if ($line.StartsWith('#')) { return }
    $idx = $line.IndexOf('=')
    if ($idx -lt 1) { return }
    $key = $line.Substring(0, $idx).Trim()
    $val = $line.Substring($idx + 1).Trim()
    if (($val.StartsWith('"') -and $val.EndsWith('"')) -or ($val.StartsWith("'") -and $val.EndsWith("'"))) {
      $val = $val.Substring(1, $val.Length - 2)
    }
    $existing = [Environment]::GetEnvironmentVariable($key)
    if ([string]::IsNullOrWhiteSpace($existing)) {
      [Environment]::SetEnvironmentVariable($key, $val)
      $env:$key = $val
    }
  }
}

$RootDir = Resolve-Path (Join-Path $PSScriptRoot '..\..') | Select-Object -ExpandProperty Path
Set-Location $RootDir
Import-DotEnv (Join-Path $RootDir '.env')

Set-EnvDefault 'RHELMA_CONTROL_SERVICE_URL' 'http://127.0.0.1:8086'
Set-EnvDefault 'RHELMA_SOCIAL_SERVICE_URL' 'http://127.0.0.1:8085'
Set-EnvDefault 'RHELMA_REGION' 'local'
Set-EnvDefault 'RHELMA_CONTROL_NODE_REGISTRATION_TOKEN' 'dev-node-token'
Set-EnvDefault 'RHELMA_SERVICE_VERSION' '0.0.0-dev'

$payload = @{
  name = 'local-social'
  region = $env:RHELMA_REGION
  public_base_url = $env:RHELMA_SOCIAL_SERVICE_URL
  realm_slug = 'central'
  capabilities = @{ 'social-service' = $true }
  version = $env:RHELMA_SERVICE_VERSION
} | ConvertTo-Json -Depth 6

Invoke-RestMethod -Method Post -Uri ("$($env:RHELMA_CONTROL_SERVICE_URL)/v1/nodes/register") -Headers @{ 'x-control-node-registration-token' = $env:RHELMA_CONTROL_NODE_REGISTRATION_TOKEN } -ContentType 'application/json' -Body $payload -TimeoutSec 10 | ConvertTo-Json -Depth 20
