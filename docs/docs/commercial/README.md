# Commercial Layer

This directory documents the boundary for commercial Rhelma capabilities. It should not contain proprietary implementation details, customer data, production secrets, or private deployment manifests.

## Kept Private

- Hosted production operations
- Enterprise administration
- Billing and subscription logic
- Customer-specific integrations
- Private AI workflows
- Compliance exports and custom reports
- Real infrastructure inventories

## Public References

Public documentation may describe commercial capabilities at a product level, but implementation should live in a private repository or private package registry.

Recommended private repository:

```text
rhelma-commercial/
  enterprise-modules/
  admin-panel/
  billing/
  deployment/
  integrations/
  support/
```

Private modules should consume the public core through stable crates, APIs, and event contracts.

See `../../COMMERCIAL_BOUNDARY.md` for the active commercial boundary.
