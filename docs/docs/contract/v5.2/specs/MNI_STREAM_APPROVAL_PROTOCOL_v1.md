# MNI Stream Approval Protocol v1

## Purpose
Provide a standardized, auditable workflow for approving sensitive training streams using the Rhelma Realm Security System:
System Judge + Honorary Police + Human Jury.

## Objects
### StreamApprovalRequestV1
- `stream_id` (string)
- `stream_revision` (u64)
- `dataset_head_hash` (hex string, optional)
- `requested_by` (string)
- `reason` (string)
- `created_at` (RFC3339)

### StreamApprovalResultV1
- `stream_id`
- `stream_revision`
- `decision` = `approved` | `rejected`
- `jury_incident_id` (string)
- `decided_at` (RFC3339)
- `notes` (string, optional)

## Security
- Requests SHOULD be signed (Ed25519) or at least authenticated via service token in early phases.
- Callback MUST be authenticated (shared secret token at minimum).

## State machine
- draft -> pending_jury -> approved
- draft -> pending_jury -> rejected
- rejected -> draft (only via new revision)
