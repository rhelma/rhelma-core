# Security Governance API v1 (Rhelma 6)

This API implements the Live Defense constitution in an operational, auditable way.
**It does not execute high-impact actions**; it proposes, votes, resolves, and emits verifiable records.

## Endpoints

### POST /v1/reports
Create a citizen report.

Body:
- kind: "suspicious_behavior" | "abuse" | "security_bug" | "policy_violation" | "other"
- summary: string
- evidence?: JSON (optional)
- signal_strength?: 0..100 (optional)

Returns: Report (201)

### POST /v1/reports/{id}/evidence
Add/replace evidence for a report.
- Strengthens the signal and queues triage.

### POST /v1/incidents/triage
Convert reports into an incident (System Judge only).

Headers:
- x-judge-token

Body:
- report_ids: UUID[]
- title: string

Returns: Incident (201)

### POST /v1/incidents/{id}/actions/propose
Propose a defensive action (Honorary Police only).

Headers:
- x-police-token

Body:
- kind: string (e.g. "suspend_node", "quarantine_topic", "raise_challenge_level")
- params: JSON

Returns: Incident (200)

### POST /v1/incidents/{id}/jury/start
Start jury voting (Admin or Judge; MVP).

Headers:
- x-admin-token OR x-judge-token

Returns: Incident (200)

### POST /v1/incidents/{id}/jury/vote
Cast a jury vote (citizen). In MVP, no auth is required, but in production it MUST be signed/verified.

Body:
- voter_tag: string (pseudonymous)
- approve: boolean
- reason?: string

Returns: Incident (200)

### POST /v1/incidents/{id}/appeals
File an appeal (citizen).

Body:
- appellant_tag: string
- reason: string

Returns: Incident (201)

### POST /v1/incidents/{id}/resolve
Resolve an incident (Admin).
MVP rule: requires >=2 votes, approvals > rejections => resolved, else rejected.

### GET /v1/incidents/{id}
Fetch incident details.

## Security Notes (Required for Production)
- Replace tokens with Rhelma Auth/RBAC.
- Store sensitive evidence in an access-controlled store; keep public audit metadata only.
- Add rate limits and anti-spam.
- Require cryptographic signatures for jury votes and report submissions above a threshold.

## Event Catalog (recommended)
- security.report.created@v1
- security.report.evidence_added@v1
- security.incident.created@v1
- security.action.proposed@v1
- security.jury.started@v1
- security.jury.vote@v1
- security.incident.resolved@v1
- security.incident.appeal@v1
