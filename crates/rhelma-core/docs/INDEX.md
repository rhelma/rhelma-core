# rhelma-core Documentation Index

**Version:** 5.1.x (Stable)  
**Release Date:** December 6, 2025 (v5.1.0)  
**Status:** Production-Ready — v5.2 APIs are preview (internal)

This directory contains comprehensive documentation for **rhelma-core**, the foundational primitives of the Rhelma Platform.

## ⚠️ Version Notice

- **v5.1.x** → **Stable** and recommended for production.
- **v5.2 APIs** (e.g., `RequestContextV52`, `ErrorEnvelopeV52`) → **Preview / internal** until an official v5.2 release.

---

## 📚 Documentation Structure

### Core Guides

1. **[01-ARCHITECTURE.md](./01-ARCHITECTURE.md)** — System design, core primitives, and interactions
2. **[02-ERROR-HANDLING.md](./02-ERROR-HANDLING.md)** — Error model, RhelmaError enum, context propagation
3. **[03-REQUEST-CONTEXT.md](./03-REQUEST-CONTEXT.md)** — RequestContext v5.2, headers, zero-trust principles
4. **[04-CONFIGURATION.md](./04-CONFIGURATION.md)** — AppConfig loading, validation, environment variables
5. **[05-TENANCY-RESIDENCY.md](./05-TENANCY-RESIDENCY.md)** — Multi-tenant governance, data residency policies
6. **[06-TYPE-SYSTEM.md](./06-TYPE-SYSTEM.md)** — Strong identifiers, validation, immutability
7. **[07-OBSERVABILITY.md](./07-OBSERVABILITY.md)** — Logging, metrics, tracing integration

### Advanced Topics

8. **[08-SECURITY-PASSWORDS.md](./08-SECURITY-PASSWORDS.md)** — Password policies, rate limiting, zero-trust
9. **[09-REALTIME-PRESENCE.md](./09-REALTIME-PRESENCE.md)** — Session tracking, connection metadata, presence status
10. **[10-INTEGRATION-GUIDE.md](./10-INTEGRATION-GUIDE.md)** — Using rhelma-core in your service
11. **[11-MIGRATION-GUIDE.md](./11-MIGRATION-GUIDE.md)** — Upgrading to v5.2.0

**Preview:** **[12-V52-MIGRATION.md](./12-V52-MIGRATION.md)** — Adopting v5.2 preview APIs safely

### Reference

12. **[A1-API-REFERENCE.md](./A1-API-REFERENCE.md)** — Public API documentation
13. **[A2-ERROR-CODES.md](./A2-ERROR-CODES.md)** — Complete error code catalog
14. **[A3-ENVIRONMENT-VARIABLES.md](./A3-ENVIRONMENT-VARIABLES.md)** — Configuration reference

---

## 🎯 Quick Navigation

### I need to...

- **Understand the architecture** → [01-ARCHITECTURE.md](./01-ARCHITECTURE.md)
- **Handle errors properly** → [02-ERROR-HANDLING.md](./02-ERROR-HANDLING.md)
- **Parse request context** → [03-REQUEST-CONTEXT.md](./03-REQUEST-CONTEXT.md)
- **Load configuration** → [04-CONFIGURATION.md](./04-CONFIGURATION.md)
- **Manage multi-tenancy** → [05-TENANCY-RESIDENCY.md](./05-TENANCY-RESIDENCY.md)
- **Use strong types** → [06-TYPE-SYSTEM.md](./06-TYPE-SYSTEM.md)
- **Setup observability** → [07-OBSERVABILITY.md](./07-OBSERVABILITY.md)
- **Implement security** → [08-SECURITY-PASSWORDS.md](./08-SECURITY-PASSWORDS.md)
- **Build real-time features** → [09-REALTIME-PRESENCE.md](./09-REALTIME-PRESENCE.md)
- **Integrate rhelma-core** → [10-INTEGRATION-GUIDE.md](./10-INTEGRATION-GUIDE.md)
- **Migrate from v1.x** → [11-MIGRATION-GUIDE.md](./11-MIGRATION-GUIDE.md)
- **Look up API details** → [A1-API-REFERENCE.md](./A1-API-REFERENCE.md)
- **Find error codes** → [A2-ERROR-CODES.md](./A2-ERROR-CODES.md)
- **Configure environment** → [A3-ENVIRONMENT-VARIABLES.md](./A3-ENVIRONMENT-VARIABLES.md)

---

## 🏗️ Architecture Overview

```
rhelma-core
├── config              → Environment-based configuration
├── error               → Unified error model (RhelmaError)
├── request_context     → Zero-Trust RequestContext (v5.1) + v5.2 preview types
├── tenancy             → Multi-tenant governance
├── types               → Strong identifiers (TenantId, UserId, Email, etc.)
├── observability       → Structured logging & tracing
├── realtime_types      → Session tracking, presence
├── security            → Password policies, rate limiting
└── prelude             → Convenient re-exports
```

---

## 🚀 Getting Started

### 1. Install rhelma-core

```toml
[dependencies]
rhelma-core = "5.1"
```

### 2. Load Configuration

```rust
use rhelma_core::prelude::*;

let cfg = AppConfig::from_env_only()?;
cfg.validate_all()?;
```

### 3. Parse RequestContext

```rust
let ctx = RequestContext::from_headers(vec![
    ("x-request-id", "550e8400-e29b-41d4-a716-446655440000"),
    ("x-tenant-id", "acme-corp"),
    ("x-region", "eu-west-1"),
])?;
```

### 4. Handle Errors

```rust
fn process(id: String) -> RhelmaResult<Data> {
    validate_id(&id)
        .rhelma_context("while validating ID")?;
    
    fetch_data(&id)
        .rhelma_context("while fetching data")
}
```

---

## 📋 Compliance Checklist

To ensure your service is **rhelma-core v5.1 compliant**, verify:

- ✅ RequestContext (v5.1) used for all requests
- ✅ If you adopt v5.2 preview: `RequestContextV52::validate_external` enforced at edges
- ✅ Error handling follows RhelmaError model
- ✅ Configuration loaded and validated
- ✅ Tenancy and residency enforced
- ✅ Type-safe identifiers used everywhere
- ✅ Observability signals emitted
- ✅ Zero-Trust principles applied
- ✅ Secrets managed via KMS (not AppConfig)
- ✅ All tests pass

---

## 🔄 Release Cycle

| Version | Status | Release Date | Support Until |
|---------|--------|--------------|---------------|
| 5.2.0 | Preview (unreleased) | — | — |
| 5.1.0 | **Stable** | Dec 6, 2025 | Dec 6, 2027 |
| 5.0.x | Outdated | Q2 2025 | Dec 6, 2025 |
| 1.1.x | Legacy | 2025 | Q1 2026 |

---

## 📖 Related Rhelma Specifications

This crate implements core parts of:

- **Rhelma Contract v5.1** — Main Architecture
- **Observability v5.1** — Logs, metrics, tracing
- **Zero-Trust Security v5.1** — Identity and authorization
- **Error Model v5.1** — Unified error handling
- **Tenancy Model v5.1** — Multi-tenant governance

For full platform specifications, see the parent `/docs` directory.

---

## 🤝 Contributing

Contributions are welcome! When updating documentation:

1. Follow Markdown style guide
2. Include code examples where applicable
3. Keep API documentation in sync with Rust docs
4. Update this index if adding new documents
5. Run spell check

---

## ❓ FAQ

**Q: Do I need to use rhelma-core?**  
A: Yes, it's mandatory for all Rhelma services.

**Q: Can I bypass validation?**  
A: No. Zero-Trust requires validation at all boundaries.

**Q: How do I migrate from v1.x?**  
A: See [11-MIGRATION-GUIDE.md](./11-MIGRATION-GUIDE.md) for detailed steps.

**Q: What's the difference between context() and rhelma_context()?**  
A: `rhelma_context()` is the primary method; `context()` is a backwards-compatible alias.

**Q: Can TenantId contain uppercase letters?**  
A: No. Only lowercase alphanumeric and `-` are allowed.

---

## 📞 Support

- **Documentation Issues:** File a GitHub issue
- **API Questions:** Check [A1-API-REFERENCE.md](./A1-API-REFERENCE.md)
- **Error Help:** Check [A2-ERROR-CODES.md](./A2-ERROR-CODES.md)
- **Slack:** #rhelma-platform
- **Email:** rhelma-platform@example.com

---

**Last Updated:** December 18, 2025  
**Maintainers:** Rhelma Platform Team











