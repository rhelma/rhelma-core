# MNI Expert Routing Policy — v1

## Gates (hard constraints)
1. Node status must be ACTIVE (not quarantined/banned).
2. If expert requires attestation, candidate must be attested.
3. Candidate reputation must be >= `min_reputation`.
4. Residency constraints must be satisfied.

## Scoring (soft constraints)
- Higher reputation → higher score
- Lower latency estimate → higher score
- Fresh heartbeat → higher score
- Expert tag match → higher score

## Output
- `RoutingDecision` must include:
  - why chosen (compact reason codes)
  - policy head hash
  - snapshot hash

## Security notes
- No candidate-provided fields are trusted without verification.
- Routing decisions are logged and (optionally) checkpointed for external verification.
