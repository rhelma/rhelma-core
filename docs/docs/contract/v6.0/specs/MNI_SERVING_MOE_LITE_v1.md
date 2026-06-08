# MNI Serving (MoE-lite) — Specification v1

## Motivation
Full distributed MoE training is later-phase. Serving can still be **MoE-like** by routing requests to specialized
executors (experts) and mixing results *optionally*.

## Terms
- **Expert**: a named capability class (code, security, governance, creative, ops, etc.)
- **Candidate**: a concrete execution target (MNI-RAG, LLM node, local model, etc.)
- **Decision**: a deterministic selection record (auditable)

## Data types (conceptual)
- ExpertProfile:
  - `expert_id`, `tags[]`, `required_attested`, `min_reputation`, `preferred_regions[]`
  - `max_cost_units`, `max_latency_ms`
- RoutingDecision:
  - `decision_id` (sha256)
  - `request_hash` (sha256 over stable fields)
  - `candidate_id`
  - `expert_id` (optional)
  - `policy_hash` (active policy head)
  - `created_at_unix`

## Determinism
Routing must be deterministic for the same:
- request stable hash
- policy head hash
- candidate set snapshot hash

## Fallback
- bounded attempts (e.g., 3)
- avoid repeating the same candidate
- if all fail → return standardized error with decision trace
