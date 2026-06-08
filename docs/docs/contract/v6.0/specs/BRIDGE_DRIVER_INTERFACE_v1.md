# Bridge Driver Interface v1 (Rhelma 6)

This spec defines the **driver boundary** between Rhelma Realm and external settlement networks.

## Goals
- Keep bridging generic and portable.
- Make verification deterministic and auditable.
- Provide stable types for contracts and receipts.

## Types
- `BridgeIntentV1`
- `ExternalProofV1`
- `SettlementResultV1`

## Determinism & audit
Drivers MUST:
- be deterministic (same input => same output)
- not depend on wall-clock time, randomness, or remote state without explicit proof payload
- compute an audit digest over (intent + proof)

## Error model
- `UnsupportedChain`
- `InvalidProof`
- `Rejected`
- `Internal`

## Security requirements (for production drivers)
- run in a sandboxed environment (defense-in-depth)
- be allowed only by policy (quorum approval)
- produce structured evidence for Jury/Appeals
- implement rate limiting / abuse controls

## Mock driver
Phase 23 includes `MockChainDriver` for "mocknet" to validate the pipeline end-to-end.
