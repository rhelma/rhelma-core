# ai-orchestrator Routing Hooks (Phase 11)

## New policy knobs
- `RHELMA_AI_ORCH__NODE_ROUTING__MIN_REPUTATION`
- `RHELMA_AI_ORCH__NODE_ROUTING__REQUIRE_ATTESTED`

## Expected behavior
- Prefer eligible nodes (active, not quarantined).
- Enforce min reputation + attestation gates.
- Keep existing retry/failover logic.
- Do not auto-quarantine from the orchestrator; only governance can apply policy actions.
