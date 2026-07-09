# Open-Source Release Checklist

Use this checklist before publishing a public Rhelma release.

## Source Hygiene

- Confirm `git status` only contains intended release changes.
- Search for secrets, tokens, private URLs, customer names, production IPs, and local-only paths.
- Confirm `.env`, production manifests, real certificates, and private keys are not tracked.
- Confirm `.env.example` contains safe placeholder values only.
- Verify `Cargo.toml` license metadata matches repository license files.

## Build And Test

- Run formatting for the workspace.
- Run clippy with warnings denied.
- Run the workspace test suite.
- Run public smoke tests for the services included in the release.
- Build generated API documentation without warnings.

## Documentation

- Update `README.md` quick start instructions.
- Update `docs/INDEX.md` and `docs/README.md` if new pages were added.
- Verify public API contracts and OpenAPI files are current.
- Mark experimental features clearly.
- Confirm commercial features are described only at the product level.

## Demo

- Verify the Asrnegar demo can run without private modules.
- Confirm demo data is synthetic.
- Confirm public demo rate limits, moderation defaults, and abuse controls are documented.
- Confirm demo observability does not expose private telemetry.

## Release

- Create a version tag.
- Publish release notes.
- Attach generated artifacts only if they contain no secrets.
- Publish matching documentation updates on `rhelma.ir`.
- Announce the Asrnegar demo URL if it is ready for public traffic.
