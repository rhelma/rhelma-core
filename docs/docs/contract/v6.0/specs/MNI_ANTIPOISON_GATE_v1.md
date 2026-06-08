# MNI Anti-poison Gate v1

## Inputs
- `proposal_ref` (Phase 39)
- `eval_result` (signed)
- `challenge_set` (signed)
- `dataset_anchor_head` (from Phase 33)
- `node_trainer_attestation` (optional)

## Gate Heuristics
1) **Signature validity**: both challenge set and eval result must verify.
2) **Thresholds**:
   - `aggregate_score >= pass` => PASS
   - `aggregate_score >= pass_with_constraints` => PASS_WITH_CONSTRAINTS
   - otherwise => REJECT
3) **Red flags**:
   - any `red_flags` triggered => downgrade one level (PASS->CONSTRAINTS, CONSTRAINTS->REJECT)
4) **Diversity checks** (optional): challenge set must include at least N categories.

## Outputs
### GateDecisionV1
- `decision`: "pass" | "pass_with_constraints" | "reject"
- `reason_codes`: array<string>
- `constraints` (optional):
  - `max_routing_weight`
  - `extra_sampling_rate`
  - `require_more_challenges`

## Logging
Every gate decision must be append-only logged and checkpointable.
