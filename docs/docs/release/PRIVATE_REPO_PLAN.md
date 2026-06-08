# Private Repository Plan

## Target Repository

`rhelma-commercial`

## Include

- Enterprise administration
- Billing and subscriptions
- Hosted operations
- Private deployment manifests
- Customer integrations
- Compliance and support tools
- Proprietary AI workflows

## Current Workspace Review Candidates

Start from the paths listed in `COMMERCIAL_BOUNDARY.md`.

## Dependency Rule

The commercial repository should depend on the public Rhelma core through stable crates, APIs, event contracts, and SDKs. Avoid copying public code into private modules.

## Work Items

- Create private repository.
- Move or mirror private-only modules after review.
- Add private CI with access to required secrets.
- Keep customer data and production configs outside public history.
- Document compatibility with public core versions.
