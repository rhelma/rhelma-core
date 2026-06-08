# Security Governance Enforcement (Phase 11)

## New action types
- `propose_quarantine`
- `propose_unquarantine`

## Execution rule
Only after:
1) incident triage,
2) police proposal,
3) jury vote,
4) incident resolve

...may the governance service call node-registry internal endpoints.

This maintains human-in-the-loop and avoids centralized power.
