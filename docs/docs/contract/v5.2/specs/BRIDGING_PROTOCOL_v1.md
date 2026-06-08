# Bridging Protocol v1 (Realm ↔ Real)

## Overview
The bridge is a **two-step intent-based process**:
1) **Intent** (internal): subject requests a transfer to an external destination.
2) **Settlement** (external): adapter produces/accepts proof of settlement.

## Objects
### BridgeIntent
- `intent_id`
- `subject_id`
- `asset` (e.g., "RHELMA_CREDIT")
- `amount`
- `destination` (driver-specific)
- `status`: pending | escrowed | finalized | cancelled
- `burn_ref` (internal ledger anchor)
- `external_proof` (driver-specific)

### SignedReceipt
A signed receipt is the minimal portable proof that:
- an intent exists
- escrow/burn was performed
- destination + amount were authorized

## Governance
- Enable/disable bridging is a **quorum-controlled policy** (Phase 19+).
- Disputes are handled by Security Governance jury/appeal.

## Security
- Receipts must be signed using ed25519.
- Receipts must be *privacy-minimal*: never include personal/family secrets.
- Replays are prevented by unique IDs + idempotency.
