# Testing

This section documents how to validate Rhelma locally and in CI.

## Quick start

Run the standard developer suite:

```bash
bash scripts/test_all.sh
```

Windows:

```powershell
./scripts/test_all.ps1
```

## End-to-end

See:

- `docs/contributing/END_TO_END_TESTING.md`
- `scripts/e2e_local.(sh|ps1)`
- `scripts/smoke_local.(sh|ps1)`

## Notes

- Unit tests should be deterministic and avoid external dependencies.
- Use the live tier only for validating "real" process wiring (ports, env, docker infra).
