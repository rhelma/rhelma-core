# Contributing to rhelma-core

Thank you for your interest in contributing to rhelma-core! This document provides guidelines and instructions for contributing.

---

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
2. [Getting Started](#getting-started)
3. [Development Setup](#development-setup)
4. [Making Changes](#making-changes)
5. [Coding Standards](#coding-standards)
6. [Testing](#testing)
7. [Documentation](#documentation)
8. [Submitting Changes](#submitting-changes)
9. [Review Process](#review-process)
10. [Release Process](#release-process)

---

## Code of Conduct

This project adheres to the Rust Code of Conduct. By participating, you agree to:

- Be respectful and inclusive
- Welcome people of all backgrounds
- Report unacceptable behavior to maintainers

---

## Getting Started

### Prerequisites

- **Rust:** 1.70+ (check with `rustc --version`)
- **Cargo:** Latest stable
- **Git:** For version control

### Fork & Clone

```bash
# Fork on GitHub, then clone your fork
git clone https://github.com/YOUR_USERNAME/rhelma.git
cd rhelma/crates/rhelma-core

# Add upstream remote
git remote add upstream https://github.com/asrnegar/rhelma.git
```

### Verify Installation

```bash
# Build project
cargo build --all-features

# Run tests
cargo test --all-features

# Check code quality
cargo clippy --all-features
```

---

## Development Setup

### Project Structure

```
rhelma-core/
├── src/
│   ├── config.rs           # Configuration loading
│   ├── error.rs            # Error types
│   ├── request_context.rs  # RequestContext v5.1
│   ├── types/
│   │   ├── ids.rs          # Strong identifiers
│   │   ├── pagination.rs   # Pagination helpers
│   │   └── common.rs       # Common types
│   ├── tenancy.rs          # Multi-tenant governance
│   ├── security.rs         # Security utilities
│   ├── realtime_types.rs   # Realtime primitives
│   ├── observability.rs    # Observability config
│   ├── trace_context.rs    # Distributed tracing
│   ├── prelude.rs          # Re-exports
│   └── lib.rs              # Library root
├── tests/
│   ├── config_test.rs
│   └── integration_test.rs
├── docs/
│   ├── 01-ARCHITECTURE.md
│   ├── A1-API-REFERENCE.md
│   └── ...
└── Cargo.toml
```

### IDE Setup

**VS Code:**
```json
{
  "rust-analyzer.cargo.features": ["full"],
  "rust-analyzer.checkOnSave.command": "clippy",
  "[rust]": {
    "editor.formatOnSave": true,
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

**Vim/Neovim:**
```
# Use rust.vim + coc-rust-analyzer
```

---

## Making Changes

### Branch Naming

```bash
# Feature
git checkout -b feature/request-context-v6

# Bug fix
git checkout -b fix/email-validation-edge-case

# Documentation
git checkout -b docs/add-security-guide

# Chore
git checkout -b chore/update-dependencies
```

### Commit Messages

**Format:**
```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:** feat, fix, docs, style, refactor, test, chore, ci

**Examples:**
```
feat(error): add security_policy error variant

Add new RhelmaError::SecurityPolicy variant for zero-trust violations.
This enables residency policy enforcement at the type level.

Implements part of Rhelma Contract v5.1 security requirements.
```

```
fix(email): strict RFC 5322 validation

Replace permissive email parser with validator crate for RFC 5322
compliance. Rejects emails with spaces, multiple @, invalid TLD.

Fixes #123: Email validation bypass
```

```
docs(architecture): add integration patterns

Add section on HTTP middleware integration with RequestContext
extraction and error handling best practices.
```

---

## Coding Standards

### Style Guide

Follow Rust conventions:

```bash
# Format code
cargo fmt --all

# Check formatting
cargo fmt --all -- --check
```

### Naming Conventions

```rust
// ✅ Good: Clear, descriptive names
pub fn validate_residency(&self, region: &RegionId) -> Result<(), RhelmaError>

// ❌ Bad: Abbreviated or unclear
pub fn val_res(&self, r: &RegionId) -> Result<(), RhelmaError>
```

### Documentation

Every public item requires documentation:

```rust
/// Validates that a tenant's region matches their residency policy.
///
/// # Arguments
/// * `region` — The region to validate
///
/// # Returns
/// - `Ok(())` if region is allowed
/// - `Err(RhelmaError::SecurityPolicy)` if region violates policy
///
/// # Examples
///
/// ```
/// let tenant = TenantProfile {
///     residency: ResidencyPolicy::RegionalRequired,
///     primary_region: RegionId::parse("eu-west-1")?,
///     ..Default::default()
/// };
///
/// tenant.validate_residency(&RegionId::parse("eu-west-1")?)?;  // ✅
/// tenant.validate_residency(&RegionId::parse("us-west-2")?)?;  // ❌ Error
/// ```
pub fn validate_residency(&self, region: &RegionId) -> Result<(), RhelmaError> {
    // ...
}
```

### Error Handling

```rust
// ✅ Good: Return Result, use ? operator
fn load_config() -> RhelmaResult<AppConfig> {
    AppConfig::from_env_only()
        .rhelma_context("while loading config")
}

// ❌ Bad: Panics
fn load_config() -> AppConfig {
    AppConfig::from_env_only().unwrap()
}

// ❌ Bad: Ignoring errors
fn load_config() -> Option<AppConfig> {
    AppConfig::from_env_only().ok()
}
```

### Type Safety

```rust
// ✅ Good: Use strong types
fn create_invoice(tenant: TenantId, user: UserId) -> RhelmaResult<Invoice> {
    // ...
}

// ❌ Bad: Using strings
fn create_invoice(tenant: String, user: String) -> RhelmaResult<Invoice> {
    // ...
}
```

### Mutability

```rust
// ✅ Good: Builders are immutable, return new instance
let ctx = RequestContext::empty()
    .with_tenant(tenant)
    .with_region(region);

// ❌ Bad: Mutable fields
let mut ctx = RequestContext::empty();
ctx.tenant_id = Some(tenant);  // ❌ Can't do this, fields are private
```

---

## Testing

### Unit Tests

Place tests in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_id_parse_valid() {
        let tenant = TenantId::parse("acme-corp").unwrap();
        assert_eq!(tenant.as_str(), "acme-corp");
    }

    #[test]
    fn test_tenant_id_parse_invalid_uppercase() {
        let result = TenantId::parse("ACME-CORP");
        assert!(result.is_err());
    }

    #[test]
    fn test_tenant_id_parse_invalid_space() {
        let result = TenantId::parse("acme corp");
        assert!(result.is_err());
    }
}
```

### Integration Tests

Place in `tests/` directory:

```rust
// tests/integration_test.rs
#[tokio::test]
async fn test_request_context_from_headers() {
    let headers = vec![
        ("x-request-id", "550e8400-e29b-41d4-a716-446655440000"),
        ("x-tenant-id", "test-tenant"),
        ("x-region", "eu-west-1"),
    ];

    let ctx = RequestContext::from_headers(headers).unwrap();
    assert!(ctx.has_tenant());
    assert!(ctx.has_region());
}
```

### Running Tests

```bash
# All tests
cargo test --all-features

# Specific test
cargo test test_tenant_id_parse_valid

# With output
cargo test -- --nocapture

# Doc tests only
cargo test --doc

# Integration tests only
cargo test --test '*'
```

### Coverage Requirements

- **Minimum:** 85% code coverage
- **Target:** 95%+ for critical paths
- **Tools:** `cargo tarpaulin` (optional)

```bash
cargo tarpaulin --all-features --timeout 300
```

---

## Documentation

### Rustdoc

```bash
# Build and open documentation
cargo doc --open

# Check documentation completeness
cargo doc --no-deps --document-private-items
```

### Markdown Docs

Update documentation when:

- Adding new public types
- Changing error behavior
- Adding configuration options
- Adding examples

**Files to Update:**

- `docs/01-ARCHITECTURE.md` — If architecture changes
- `docs/A1-API-REFERENCE.md` — If public API changes
- `docs/INDEX.md` — If adding new sections
- `CHANGELOG.md` — Always

### Examples

Include working examples in documentation:

```rust
/// # Examples
///
/// ```
/// use rhelma_core::prelude::*;
///
/// let tenant = TenantId::parse("acme-corp")?;
/// let region = RegionId::parse("eu-west-1")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
```

---

## Submitting Changes

### Prepare Commit

```bash
# Format code
cargo fmt --all

# Check with clippy
cargo clippy --all-features -- -D warnings

# Run full test suite
cargo test --all-features

# Run doc tests
cargo test --doc
```

### Create Pull Request

**Title Format:**
```
[<scope>] <description>

Examples:
[error] Add new security_policy error variant
[docs] Add integration patterns guide
[fix] Fix email validation bypass
```

**Description:**

```markdown
## Description
Brief explanation of changes.

## Related Issue
Fixes #123

## Type of Change
- [ ] New feature
- [ ] Bug fix
- [ ] Documentation update
- [ ] Breaking change

## Testing
Describe how you tested these changes.

## Checklist
- [ ] Code formatted (`cargo fmt`)
- [ ] Lints pass (`cargo clippy`)
- [ ] Tests pass (`cargo test --all-features`)
- [ ] Docs updated
- [ ] Changelog updated
```

### Push & Create PR

```bash
# Push to your fork
git push origin feature/my-feature

# Open PR on GitHub
# Fill in template
# Respond to feedback
```

---

## Review Process

### Automated Checks

All PRs must pass:

- ✅ `cargo fmt --check` (formatting)
- ✅ `cargo clippy --all-features` (linting)
- ✅ `cargo test --all-features` (tests)
- ✅ `cargo test --doc` (doc tests)

### Manual Review

Maintainers will review:

- ✅ Code quality & style
- ✅ API design & consistency
- ✅ Test coverage
- ✅ Documentation
- ✅ Rhelma Contract compliance
- ✅ Security implications

### Approval & Merge

- Requires 1 approval from maintainer
- All checks must pass
- Maintainer will merge with squash commit

---

## Release Process

### Version Bumping

Follows [Semantic Versioning](https://semver.org/):

```
MAJOR.MINOR.PATCH

5.1.0  ← v5.1 stable release
5.1.1  ← v5.1 patch release
5.2.0  ← v5.2 feature release (breaking changes)
```

### Release Checklist

- [ ] Update version in `Cargo.toml`
- [ ] Update `CHANGELOG.md`
- [ ] Update documentation if needed
- [ ] Run full test suite
- [ ] Create release branch
- [ ] Create PR with version bump
- [ ] Merge after approval
- [ ] Create git tag: `rhelma-core-v5.1.0`
- [ ] Push tag: `git push origin rhelma-core-v5.1.0`
- [ ] Publish to crates.io: `cargo publish`
- [ ] Create GitHub release with changelog

---

## Common Tasks

### Adding a New Error Type

```rust
// 1. Add variant to RhelmaError enum (src/error.rs)
#[error("new error: {0}")]
NewError(String),

// 2. Add HTTP status mapping (if needed)
RhelmaError::NewError(_) => (StatusCode::BAD_REQUEST, "new_error"),

// 3. Add to as_str() method
RhelmaError::NewError(_) => "new_error",

// 4. Update documentation
// 5. Add tests
// 6. Update CHANGELOG.md
```

### Adding a New Type

```rust
// 1. Create new module or add to existing
// 2. Implement required traits: Debug, Clone, Serialize, Deserialize
// 3. Add validation via parse() method
// 4. Add to prelude.rs if public
// 5. Document with rustdoc examples
// 6. Add tests
// 7. Update CHANGELOG.md
```

### Updating Documentation

```bash
# Edit docs
vim docs/01-ARCHITECTURE.md

# Verify links work
cargo doc --open

# Check formatting
prettier --check docs/

# Update if needed
prettier --write docs/
```

---

## Getting Help

- **Questions:** Open a GitHub discussion
- **Bug Report:** Open an issue with minimal reproduction
- **Feature Request:** Open an issue with use case
- **Security Issues:** Email security@example.com (don't open public issue)

---

## Recognition

Contributors are recognized in:

- `CONTRIBUTORS.md` file
- GitHub contributors page
- Release notes for major contributions

---

## License

By contributing, you agree that your contributions will be licensed under the same terms as the project (Apache 2.0 OR MIT).

---

**Thank you for contributing to rhelma-core! 🚀**

For detailed guidelines, see:
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rhelma Contract v5.1](../docs/01_MAIN_ARCHITECTURE_v5.1.md)
- [rhelma-core Architecture](./docs/01-ARCHITECTURE.md)







