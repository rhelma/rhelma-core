# Contributing

This repository uses **strict linting** and a **contract-first** approach.

## Quick checks

Run these before opening a PR:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

## Supply chain checks (recommended)

These are also enforced in CI.

```bash
cargo install cargo-audit
cargo audit

cargo install cargo-deny
cargo deny check
```

## Coding rules

- `#![forbid(unsafe_code)]` in crates unless there is a strong, reviewed reason.
- No wildcard topics or regex subscriptions in Kafka configs (use explicit allow-lists).
- Prefer strong types from `rhelma-core` for tenant/region/request IDs.
- Keep errors sanitized: never leak secrets, tokens, or internal stack traces.
- Public APIs must have Rustdoc, including `# Errors` for fallible functions.

## Tests

- Unit tests for pure logic.
- Contract tests (headers/events) for boundaries between services/crates.
- Integration tests must be feature-gated if they require external services (Kafka/Redis/etc.).
