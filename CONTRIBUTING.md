# Contributing to Rhelma Core

Thank you for contributing to Rhelma Core.

Rhelma is an open-source Rust platform focused on AI-native workspaces, agents, services, and event-driven infrastructure.

## Development

Requirements:

- Rust toolchain
- Docker (for integration services)
- Node.js (for frontend tooling where applicable)

Run:

```bash
cargo fmt --all
cargo test --workspace
```

## Architecture Principles

Contributions should respect:

- Workspace-first identity
- User != Workspace != Tenant
- Explicit service contracts
- Capability-based actions
- Policy and entitlement checks
- Secure defaults

## Code Guidelines

- Prefer safe Rust (`forbid(unsafe_code)` where applicable).
- Add tests for new behavior.
- Avoid duplicate infrastructure.
- Keep APIs documented.
- Never commit secrets, tokens, or private configuration.

## Pull Requests

Include:

- Problem description
- Design decision
- Tests performed
- Documentation updates

## Security

Do not report vulnerabilities publicly. See SECURITY.md.
