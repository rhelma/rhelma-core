# Rhelma Open-Source Strategy

Rhelma uses an open-core model. The public repository should make the platform useful, auditable, and easy to extend, while keeping customer-specific and commercial capabilities in a private layer.

## Public Core

These parts are appropriate for the open-source release:

- Core Rust crates in `crates/`, especially identity, configuration, event contracts, metrics, tracing, storage abstractions, and shared domain types.
- Runnable public services in `apps/` that demonstrate the platform contract without exposing customer-specific operations.
- The social demo surface, including `apps/social-service`, frontend demo wiring, and documented API contracts.
- Local development tooling, example configuration, smoke tests, SDKs, and non-secret deployment examples.
- Architecture, contract, security, contribution, and operations documentation that helps a third party run and review the system.

## Commercial Layer

Keep these outside the public repository or behind a private distribution channel:

- Hosted production operations, customer-specific deployment manifests, and real infrastructure inventory.
- Billing, paid subscription logic, account management for commercial customers, and sales/contract workflows.
- Advanced enterprise administration, compliance dashboards, audit exports, and custom governance reports.
- Private integrations, customer connectors, proprietary AI workflows, and data migration scripts.
- Secrets, credentials, private domains, real customer identifiers, incident data, and production telemetry samples.

## Repository Boundaries

Recommended split:

- `rhelma-project`: public core, public service contracts, social product foundation, SDKs, and public docs.
- `rhelma-commercial`: private modules, hosted operations, billing, enterprise admin, and customer integrations.
- `rhelma-sites`: public documentation site for `rhelma.ir` if the website is not kept in this repository.
- `asrnegar-social`: operational social product for `asrnegar.ir` if the social system is deployed independently.

If a private module depends on public code, it should depend on stable crates and HTTP/event contracts instead of reaching into internal service details.

## Publishing Rules

- Public code must build from documented commands without private services.
- Public examples must use `.env.example` values only.
- Public docs must not mention real customers, private infrastructure names, credentials, or non-public roadmap commitments.
- Public APIs should be documented as stable, experimental, or internal.
- Commercial features should be referenced by capability, not by private implementation details.

## License

The workspace currently declares `MIT OR Apache-2.0`. Keep crate metadata, repository license files, generated documentation, and website copy aligned with that choice.
