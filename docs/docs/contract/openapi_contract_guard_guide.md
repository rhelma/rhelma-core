# OpenAPI Contract Guard

This guard validates that key OpenAPI specs exist and follow Rhelma rules.

## What it checks
- Required spec files exist under `docs/openapi/`
- Each spec contains:
  - `openapi: 3.0.x`
  - `info.version: 6.0.0`
  - `x-rhelma-contract-version: v6.0`
- Required paths exist (multi-region essentials)
- Canonical examples under `docs/openapi/examples/` match their referenced schemas

## Run
### Bash
```bash
bash scripts/openapi_contract_guard.sh .
```

### PowerShell
```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/openapi_contract_guard.ps1 -Root "."
```

## Notes
- If `python3` + `PyYAML` are installed, the guard performs deep schema + example validation.
- Without python, it falls back to minimal regex checks.
