$ErrorActionPreference = "Stop"

New-Item -ItemType Directory -Force -Path ".githooks" | Out-Null

git config core.hooksPath .githooks
Write-Host "Installed git hooks path: .githooks/"
