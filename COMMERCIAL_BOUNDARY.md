# Commercial Boundary

This boundary defines what stays private for the commercial Rhelma offering.

## Goal

Keep the open-source core useful while protecting commercial operations, customer-specific work, and proprietary modules.

## Private Repositories

Recommended private structure:

```text
rhelma-commercial/
  enterprise-modules/
  admin-panel/
  billing/
  customer-integrations/
  private-deploy/
  support-tools/
  compliance/
```

## Commercial Candidates In Current Workspace

These current paths need review before any public release. They may be commercial, internal, or public only after sanitization:

- `apps/admin-web`
- `apps/control-service`
- `apps/agent-handoff`
- `apps/ai-companion`
- `apps/ai-orchestrator`
- `apps/bridge-adapter`
- `apps/digital-family-vault`
- `apps/edge-worker`
- `apps/guardian-agent`
- `apps/mni-rag`
- `apps/multi-frontend`
- `apps/patch-applier`
- `apps/realm-hub`
- `apps/region-health-aggregator`
- `apps/rhelma-bridge-drivers`
- `apps/rhelma-governance-signer`
- `apps/rhelma-node`
- `apps/security-governance`
- `apps/value-ledger`
- `apps/value-ledger-federation`
- `extras/`
- `deploy/`
- production-oriented files under `infra/`
- customer-specific scripts under `scripts/`

## Commercial Rules

- Private modules must consume public crates through stable APIs.
- Private services should not require modifying public crate internals.
- Customer-specific behavior should be implemented as plugins, private adapters, or private services.
- Public docs may mention commercial capabilities, but not private implementation details.
- Production operations, secrets, support playbooks, and customer environments must stay private.

## Review Questions

Before moving any current app to public release, answer:

- Can it run with `.env.example` only?
- Does it expose proprietary workflow or customer-specific logic?
- Does it require private infrastructure?
- Does it contain internal URLs, tokens, or operational assumptions?
- Does it belong to the platform core, the social product, or the commercial layer?
