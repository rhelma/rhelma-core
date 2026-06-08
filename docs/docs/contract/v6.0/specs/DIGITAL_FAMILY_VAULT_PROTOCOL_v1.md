# Digital Family & Inheritance Vault Protocol v1

## Overview
This protocol specifies a **verifiable** and **governance-safe** system for:
- guardianship
- recovery
- inheritance
- dispute and appeals

It is designed to integrate with:
- Security Governance (jury/appeals)
- Value Ledger Federation (credits/treasury)
- Bridging adapters (external settlement)

## Core ideas
1) **Guardians as multi-signers**
2) **Timelock** before irreversible actions
3) **Receipts**: every critical action produces a signed receipt
4) **Dispute gate**: humans can pause and review

## Data model
### Vault
- `vault_id`
- `subject_id` (owner)
- `label`
- `quorum` (min approvals)
- `guardians[]`:
  - `pubkey_b64`
  - `role` (`guardian`, `champion`, `judge_delegate` etc.)
  - `added_at`
- `plans[]` (versioned)

### Plan
- `plan_id`
- `kind` (`inheritance`, `recovery`)
- `beneficiaries[]` (subject_id, weight)
- `created_at`
- `status`

### Recovery Session
- `session_id`
- `vault_id`
- `started_at`
- `timelock_seconds`
- `approvals[]` (guardian pubkey + sig + ts)
- `status` (`pending`, `ready`, `finalized`, `cancelled`, `blocked`)

### Dispute
- `dispute_id`
- `vault_id`
- `session_id` (optional)
- `opened_at`
- `reason`
- `status` (`open`, `resolved_allow`, `resolved_deny`)

## Receipt format (canonical)
Receipts are signed with Ed25519 and include:
- `receipt_type`
- `vault_id`
- `session_id` (optional)
- `action`
- `payload_digest` (sha256 hex)
- `issued_at_unix`
- `signature_b64`

## Safety constraints
- Disputes freeze finalization.
- Quorum approvals must be unique by guardian pubkey.
- Approvals are bound to `session_id` and `payload_digest`.
- Clock skew must be bounded.
- Rate limiting is required at the API gateway layer.

## Optional: security-governance integration
On dispute open:
- create an incident in security-governance with evidence bundle digest
- jury vote result is imported as dispute resolution
