param(
    [string]$OutputDir = "",
    [switch]$Copy
)

$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")

$publicPaths = @(
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".editorconfig",
    ".gitignore",
    ".env.public.example",
    "README.md",
    "ROADMAP.md",
    "CONTRIBUTING.md",
    "CODE_OF_CONDUCT.md",
    "SECURITY.md",
    "OPEN_SOURCE_MANIFEST.md",
    "COMMERCIAL_BOUNDARY.md",
    "SOCIAL_MANIFEST.md",
    "deny.toml",
    "docker-compose.dev.yml",
    "crates/rhelma-ai-attestation",
    "crates/rhelma-ai-contracts",
    "crates/rhelma-attestation",
    "crates/rhelma-auth",
    "crates/rhelma-cache",
    "crates/rhelma-config",
    "crates/rhelma-core",
    "crates/rhelma-db",
    "crates/rhelma-event",
    "crates/rhelma-event-kafka",
    "crates/rhelma-event-kafka-agent",
    "crates/rhelma-http-observability",
    "crates/rhelma-logger",
    "crates/rhelma-metrics",
    "crates/rhelma-realm-telemetry",
    "crates/rhelma-sandbox-runner",
    "crates/rhelma-tracing",
    "apps/api-gateway",
    "apps/social-service",
    "apps/search-service",
    "apps/realtime-service",
    "apps/file-storage",
    "apps/node-registry",
    "apps/rhelma-attestation-verifier",
    "apps/sandbox-runner",
    "docs",
    "observability",
    "rhelma-sdk-js",
    "rhelma-sdk-python",
    "rhelma-sdk-go",
    "scripts/README.md",
    "scripts/bootstrap.ps1",
    "scripts/bootstrap.sh",
    "scripts/contract_guard.ps1",
    "scripts/contract_guard.sh",
    "scripts/env_contract_guard.ps1",
    "scripts/env_contract_guard.sh",
    "scripts/event_contract_guard.ps1",
    "scripts/event_contract_guard.sh",
    "scripts/header_contract_guard.ps1",
    "scripts/header_contract_guard.sh",
    "scripts/openapi_contract_guard.ps1",
    "scripts/openapi_contract_guard.sh",
    "scripts/openapi_drift_guard.ps1",
    "scripts/openapi_drift_guard.sh",
    "scripts/openapi_examples_guard.ps1",
    "scripts/openapi_examples_guard.sh",
    "scripts/prepare_public_release.ps1",
    "scripts/smoke_local.ps1",
    "scripts/smoke_local.sh",
    "scripts/todo_guard.ps1",
    "scripts/todo_guard.sh",
    "scripts/uuidv7_guard.ps1",
    "scripts/uuidv7_guard.sh",
    "scripts/verify.ps1",
    "scripts/verify.sh",
    "scripts/_lib.sh"
)

$excludedPatterns = @(
    ".env",
    "target",
    "*.pem",
    "*.key",
    "*secret*",
    "*token*",
    "target",
    "node_modules"
)

Write-Host "Rhelma public release plan"
Write-Host "Root: $root"

if (-not $Copy) {
    Write-Host ""
    Write-Host "Dry run. These paths are planned for the public release:"
    foreach ($path in $publicPaths) {
        $fullPath = Join-Path $root $path
        if (Test-Path $fullPath) {
            Write-Host "  include $path"
        } else {
            Write-Host "  missing $path"
        }
    }
    Write-Host ""
    Write-Host "Run with -Copy -OutputDir <path> after reviewing manifests."
    exit 0
}

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    throw "OutputDir is required when using -Copy."
}

$resolvedOutput = [System.IO.Path]::GetFullPath($OutputDir)
if (Test-Path $resolvedOutput) {
    throw "OutputDir already exists: $resolvedOutput"
}

New-Item -ItemType Directory -Path $resolvedOutput | Out-Null

foreach ($path in $publicPaths) {
    $source = Join-Path $root $path
    if (-not (Test-Path $source)) {
        continue
    }

    $destination = Join-Path $resolvedOutput $path
    $destinationParent = Split-Path $destination -Parent
    New-Item -ItemType Directory -Force -Path $destinationParent | Out-Null

    Copy-Item -Path $source -Destination $destination -Recurse -Force -Exclude $excludedPatterns
}

$publicEnv = Join-Path $resolvedOutput ".env.public.example"
$envExample = Join-Path $resolvedOutput ".env.example"
if (Test-Path $publicEnv) {
    Copy-Item -Path $publicEnv -Destination $envExample -Force
}

Write-Host "Public release export created: $resolvedOutput"
