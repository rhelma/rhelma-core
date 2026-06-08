# MNI Challenge Set Protocol v1

## Goal
Define a portable, replayable evaluation bundle used to score MNI model updates and detect poisoning/backdoors.

## Objects

### ChallengeSetV1
- `challenge_set_id`: string
- `version`: "v1"
- `created_at_unix`: number
- `author_pubkey_b64`: string
- `signature_b64`: string (Ed25519 over canonical JSON without signature)
- `items`: array of `ChallengeItemV1`
- `scoring`: `ScoringPolicyV1`

### ChallengeItemV1
- `id`: string
- `category`: "security" | "code" | "governance" | "alignment" | "general"
- `prompt`: string
- `expected`: object (optional; depends on scoring policy)
- `red_flags`: array<string> (optional; backdoor indicators)

### ScoringPolicyV1
- `method`: "keyword" | "regex" | "judge_llm" | "hybrid"
- `thresholds`:
  - `pass`: number
  - `pass_with_constraints`: number
- `rules`: array of rules (method-specific)

## Result Bundle

### EvalResultV1
- `challenge_set_id`
- `model_ref` (artifact hash, or provider+version)
- `dataset_anchor_head` (optional)
- `run_id`
- `started_at_unix`, `ended_at_unix`
- `scores`: array of `EvalScoreItemV1`
- `aggregate_score`
- `violations`: array<string>
- `runner_attestation`: object (optional)
- `author_pubkey_b64`
- `signature_b64`

### EvalScoreItemV1
- `id`
- `score` (0..1)
- `notes` (optional)
- `violations` (optional)

## Canonicalization
Use RFC 8785 JSON canonicalization or equivalent deterministic field ordering.
