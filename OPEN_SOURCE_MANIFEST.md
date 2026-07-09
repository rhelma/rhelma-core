# Open Source Manifest

This manifest defines the public Rhelma core for publication.

## Goal

Publish a useful, buildable, documented Rust platform core without exposing customer-specific operations, proprietary commercial modules, production secrets, or private integrations.

## Public Crates

These crates are candidates for the public repository:

- `crates/rhelma-core`
- `crates/rhelma-auth`
- `crates/rhelma-config`
- `crates/rhelma-db`
- `crates/rhelma-cache`
- `crates/rhelma-event`
- `crates/rhelma-event-kafka`
- `crates/rhelma-http-observability`
- `crates/rhelma-logger`
- `crates/rhelma-metrics`
- `crates/rhelma-tracing`
- `crates/rhelma-attestation`
- `crates/rhelma-ai-contracts`
- `crates/rhelma-ai-attestation`
- `crates/rhelma-sandbox-runner`
- `crates/rhelma-realm-telemetry`

## Public Services

These services can be included if they run with public configuration and synthetic data:

- `apps/api-gateway`
- `apps/social-service`
- `apps/search-service`
- `apps/realtime-service`
- `apps/file-storage-service`
- `apps/node-registry`
- `apps/rhelma-attestation-verifier`
- `apps/sandbox-runner`

## Public SDKs

- `packages/sdk-js`
- `packages/sdk-python`
- `packages/sdk-go`

## Public Documentation

- `README.md`
- `CONTRIBUTING.md`
- `CODE_OF_CONDUCT.md`
- `SECURITY.md`
- `ROADMAP.md`
- `docs/INDEX.md`
- `docs/getting-started/`
- `docs/architecture/`
- `docs/contract/`
- `docs/reference/`
- `docs/testing/`
- `docs/open-source/`
- `docs/sites/`

## Public Infrastructure

Include only local and example infrastructure:

- `docker-compose.dev.yml`
- `.env.public.example` exported as `.env.example`
- `infra/` files that are explicitly local or examples
- `observability/` examples that do not expose production telemetry

## Exclude From Public Release

- Real `.env` files
- Production certificates, keys, tokens, credentials, or private endpoints
- Customer names, incident records, private telemetry, or internal support logs
- Private deployment manifests
- Billing and subscription logic
- Enterprise administration modules
- Customer-specific integrations

## Required Before Publish

- Run `docs/open-source/RELEASE_CHECKLIST.md`.
- Confirm all default services build without private dependencies.
- Confirm public docs describe commercial features only at product level.
- Confirm `Cargo.toml` workspace members do not require private-only paths.
- Confirm generated OpenAPI and SDK examples match the public services.
