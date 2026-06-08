# Local smoke checks for a developer-run Rhelma stack.
#
# Thin wrapper around scripts/smoke_staging.ps1.

$ErrorActionPreference = "Stop"

# Provide local defaults consistent with docker-compose.dev.yml + common cargo run ports.
if (-not $env:RHELMA_SMOKE_TIMEOUT_SEC) { $env:RHELMA_SMOKE_TIMEOUT_SEC = if ($env:RHELMA_E2E_WAIT_TIMEOUT_SEC) { $env:RHELMA_E2E_WAIT_TIMEOUT_SEC } else { "2" } }
if (-not $env:RHELMA_SMOKE_API_GATEWAY_URL) { $env:RHELMA_SMOKE_API_GATEWAY_URL = if ($env:RHELMA_E2E_API_GATEWAY_URL) { $env:RHELMA_E2E_API_GATEWAY_URL } else { "http://127.0.0.1:3000" } }
if (-not $env:RHELMA_SMOKE_AI_ORCH_URL) { $env:RHELMA_SMOKE_AI_ORCH_URL = if ($env:RHELMA_E2E_AI_ORCH_URL) { $env:RHELMA_E2E_AI_ORCH_URL } else { "http://127.0.0.1:4000" } }
if (-not $env:RHELMA_SMOKE_SEARCH_URL) { $env:RHELMA_SMOKE_SEARCH_URL = if ($env:RHELMA_E2E_SEARCH_URL) { $env:RHELMA_E2E_SEARCH_URL } else { "http://127.0.0.1:8082" } }
if (-not $env:RHELMA_SMOKE_FILE_STORAGE_URL) { $env:RHELMA_SMOKE_FILE_STORAGE_URL = if ($env:RHELMA_E2E_FILE_STORAGE_URL) { $env:RHELMA_E2E_FILE_STORAGE_URL } else { "http://127.0.0.1:3005" } }
if (-not $env:RHELMA_SMOKE_REALTIME_URL) { $env:RHELMA_SMOKE_REALTIME_URL = if ($env:RHELMA_E2E_REALTIME_URL) { $env:RHELMA_E2E_REALTIME_URL } else { "http://127.0.0.1:9000" } }
if (-not $env:RHELMA_SMOKE_NODE_REGISTRY_URL) { $env:RHELMA_SMOKE_NODE_REGISTRY_URL = if ($env:RHELMA_E2E_NODE_REGISTRY_URL) { $env:RHELMA_E2E_NODE_REGISTRY_URL } else { "http://127.0.0.1:8090" } }
if (-not $env:RHELMA_SMOKE_LLM_NODE_URL) { $env:RHELMA_SMOKE_LLM_NODE_URL = if ($env:RHELMA_E2E_LLM_NODE_URL) { $env:RHELMA_E2E_LLM_NODE_URL } else { "http://127.0.0.1:8088" } }

& ./scripts/smoke_staging.ps1
