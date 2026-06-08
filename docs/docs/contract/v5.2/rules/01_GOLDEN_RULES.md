# Golden Rules (v5.2)

## 1) Single source of truth
- Production invariants: `docs/contract/vX.Y/`
- Rollout & gates: `docs/archive/phase-notes/phases/`
- Guides & how-to: `docs/`

## 2) Keep docs discoverable
- If you add a doc, you must link it from the relevant index:
  - `docs/README.md`
  - `docs/contract/v5.2/00_INDEX_v5.2.md` (contract)
  - `docs/contract/v5.2/rules/README.md` (rules)
  - `docs/archive/phase-notes/phases/README.md` (gates)

## 3) No secrets, no PII
- Never commit secrets.
- Logs/metrics must follow the scrubbing & cardinality rules (see: `docs/contract/v5.2/02_OBSERVABILITY_v5.2.md`).

## 4) Observability is mandatory
- Any new endpoint/event must have consistent tracing/logging/metrics.

## 5) CI is the enforcement
- Keep `scripts/verify*.{sh,ps1}` deterministic and machine-parseable.
