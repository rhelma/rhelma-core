# Launch Hardening Playbook v1

## Public Node Onboarding Safety
- Default rate limits for:
  - node registration
  - report creation
  - evidence uploads
- Default sandbox policy: deny-by-default, allowlist only.

## Abuse Controls
- Sybil resistance (initial):
  - attestation tiers
  - reputation-weighted routing
  - challenge tasks for new nodes
- Quarantine policy:
  - soft-quarantine first (routing dampening)
  - hard quarantine only after jury-approved resolution

## Operational Checklist
- Observability: traces/metrics/logs enabled
- Backups: ledger and audit logs
- Incident response rotation and procedures

## Rollback
- Every high-impact change must have a rollback plan and a time-bounded rollback window.
