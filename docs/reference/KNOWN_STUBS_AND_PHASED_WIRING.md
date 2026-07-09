# Known stubs and phased wiring

This repo intentionally contains **phase-scoped stubs** (commented or opt-in code paths)
to keep the platform moving without blocking core MVP functionality.

The goal of this doc is to make these stubs **discoverable and explicit** so you can
decide what to wire next.

## How to list stubs

Linux/macOS/WSL:

```bash
bash scripts/dev/stub-report.sh
```

Windows:

```powershell
.\scripts\dev\stub-report.ps1
```

## Categories

### Opt-in / later-phase wiring

These are typically safe to defer for MVP:

- MNI / RAG federation policy clients
- Value Ledger Federation policy artifacts
- Node-registry trust hints and VLF deposit-hold wiring

### Should be reviewed before production launch

These may be acceptable for MVP, but should be reviewed before a hardened release:

- Random-vector embedding stub in search-service (only acceptable when an embedding backend is not configured)
- Any "routes audit" or "runner contract" scaffolds that are still present in enabled code paths

## Notes

- The presence of the word "stub" in a comment does **not** necessarily mean it is reachable at runtime.
- Always confirm feature flags and routing before treating a stub as production-impacting.