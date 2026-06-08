# Verification & quality gates

Rhelma uses **repeatable verification** to prevent regressions and enforce contract requirements.

## Where to look

- Verification scripts (entrypoint): `scripts/verify.*`
- Completeness report (docs + runbooks hygiene): `scripts/dev/completeness-report.*`
- Contract rules (normative): `docs/contract/v6.0/rules/`
- Operational hardening runbook: `docs/operations/LAUNCH_HARDENING_PLAYBOOK_v1.md`

Related:

- End-to-end testing harness: `docs/contributing/END_TO_END_TESTING.md`

## What is enforced

- Formatting & lint (workspace-wide)
- Unit/integration tests for affected packages
- Contract-aligned behavior (security, observability, eventing discipline)
- Rollout safety: canary, SLO checks, rollback readiness

## Historical material

Earlier work tracked gates as “phases”. Those notes are preserved for reference only:

- Historical gate notes: `docs/archive/phase-notes/phases/`

New work should document gates here (or in `docs/operations/`) using domain names, not phase numbers.
