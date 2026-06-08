# VERSIONS.md — rhelma-core Complete Version History

**Document:** VERSIONS.md  
**Last Updated:** December 6, 2025  
**Current Version:** 5.1.0

---

## 📊 Version Overview

| Version | Release Date | Status | Stability | Support | Notes |
|---------|--------------|--------|-----------|---------|-------|
| **5.1.0** | Dec 6, 2025 | ✅ Current | Stable | 2 years | v5.1 Contract implementation |
| 5.0.x | Jun 1, 2025 | ⚠️ Old | Interim | Ended | Early v5 release |
| 1.1.4 | Dec 1, 2025 | ❌ Legacy | Outdated | Ended | Type system hardening |
| 1.1.2 | Dec 1, 2025 | ❌ Legacy | Outdated | Ended | Observability features |
| 1.1.1 | Dec 1, 2025 | ❌ Legacy | Outdated | Ended | Bug fixes |

---

## 🚀 Current Version: 5.1.0

**Release Date:** December 6, 2025  
**Status:** ✅ Stable & Production-Ready  
**Support Until:** December 6, 2027

### What's New in v5.1.0

✨ **Zero-Trust RequestContext v5.1**
- Immutable fields (all private)
- Complete request metadata
- Safe propagation across services

✨ **Unified Error Model**
- 15+ error variants
- Stable labels for observability
- HTTP status mapping

✨ **Strong Type System**
- TenantId, RegionId, UserId, Email
- Validation at construction
- Compile-time safety

✨ **Multi-Tenant Governance**
- TenantProfile with SLA/DR
- Residency policies (GDPR-ready)
- 3 tenancy tiers

✨ **Configuration Management**
- Environment-based (no files!)
- Strict validation
- Observability config

✨ **Full Documentation**
- 14 comprehensive guides
- 150+ code examples
- Multiple learning paths

### Installation

```toml
[dependencies]
rhelma-core = "5.1"

# With features:
rhelma-core = { version = "5.1", features = ["full"] }
```

### Key Features

- 🔐 Zero-Trust security by default
- 🧩 Type-safe identifiers
- 🏢 Multi-tenant governance
- 📈 Observability foundation
- ❗ Unified error model
- ⚙️ Deterministic configuration

### Learn More

- **[README.md](../README.md)** — Project overview
- **[docs/INDEX.md](../docs/INDEX.md)** — Full documentation index
- **[CHANGELOG.md](../CHANGELOG.md)** — Detailed release notes

---

## 📈 Version Timeline

### v5.1.0 (Current) — Dec 6, 2025

**Status:** ✅ Stable  
**Type:** Production Release  

**Key Changes:**
- Complete Rhelma Contract v5.1 implementation
- RequestContext immutability enforced
- 15+ error variants with stable labels
- Type-safe identifiers (TenantId, RegionId, UserId, Email)
- Multi-tenant with residency enforcement
- Comprehensive documentation (35,000+ lines)

**Breaking Changes From v1.x:**
- RequestContext fields now private (use accessors)
- from_headers() returns Result (strict validation)
- Email validation RFC 5322 compliant
- Environment validation strict (no aliases)
- Identifier validation (lowercase only)

**Migration:** [See Migration Guide](../docs/11-MIGRATION-GUIDE.md) (3-5 hours)

**Support:** 2 years (until Dec 6, 2027)

---

### v5.0.x — Jun 1, 2025

**Status:** ⚠️ Outdated  
**Type:** Interim Release  

**What It Had:**
- Early v5 contract
- Mutable RequestContext (less secure)
- Basic error model
- Configuration system

**Why Upgrade:**
- ❌ RequestContext is mutable (security issue)
- ❌ No strong identifiers
- ❌ Limited error types
- ❌ No residency enforcement
- ❌ Poor documentation

**Action:** Upgrade to v5.1.0 immediately

**Support:** Ended (no patches)

---

### v1.1.4 — Dec 1, 2025

**Status:** ❌ Legacy  
**Type:** Pre-v5 Release  

**What Changed:**
```
### Added
- Validation-aware TenantId and RegionId
- Email validation on deserialization
- Additional tests
```

**Improvements Over v1.1.2:**
- Type system hardening
- Deserialization validation
- Configuration error reporting

**Why It's Outdated:**
- ❌ RequestContext still mutable
- ❌ Basic error handling
- ❌ No Zero-Trust by default
- ❌ Limited documentation
- ❌ No strong observability

**Action:** Upgrade to v5.1.0

**Support:** Ended (no patches)

---

### v1.1.2 — Dec 1, 2025

**Status:** ❌ Legacy  
**Type:** Pre-v5 Release  

**What Changed:**
```
### Added
- ConnectionMetadata::inactivity_duration()
- Display impl for PresenceStatus
```

**Features:**
- Realtime session tracking
- Presence status enum
- Inactivity measurement

**Why It's Outdated:**
- ❌ Missing strong types
- ❌ Basic error handling
- ❌ No security hardening
- ❌ Poor documentation
- ❌ Mutable RequestContext

**Action:** Upgrade to v5.1.0

**Support:** Ended (no patches)

---

### v1.1.1 — Dec 1, 2025

**Status:** ❌ Legacy  
**Type:** Pre-v5 Release  

**What Changed:**
```
### Fixed
- Removed outdated anyhow usage
- Finalized realtime_types.rs
- Removed unused timestamp logic
- Eliminated all warnings
```

**Changes:**
- UUIDv7-only for timestamps
- Realtime types complete
- Code cleanup

**Why It's Outdated:**
- ❌ Very basic implementation
- ❌ No type safety
- ❌ No error model
- ❌ No documentation
- ❌ Pre-contract release

**Action:** Upgrade to v5.1.0

**Support:** Ended (no patches)

---

## 🔄 Upgrade Paths

### From v1.1.x → v5.1.0

**Effort:** 3-5 hours  
**Breaking:** Yes (significant)  
**Risk:** Low (clear migration path)

**Steps:**
1. Update Cargo.toml: `rhelma-core = "5.1"`
2. Fix RequestContext field access → accessors
3. Update error handling (use RhelmaError variants)
4. Validate identifier formats
5. Run tests: `cargo test --all-features`

**Resources:**
- [Migration Guide](../docs/11-MIGRATION-GUIDE.md)
- [Error Handling](../docs/02-ERROR-HANDLING.md)
- [RequestContext](../docs/03-REQUEST-CONTEXT.md)

---

### From v5.0.x → v5.1.0

**Effort:** 1-2 hours  
**Breaking:** Minor (mostly additions)  
**Risk:** Low (patch upgrade)

**Key Changes:**
- RequestContext fields become immutable
- New error variants added
- Configuration stricter

**Resources:**
- [CHANGELOG.md](../CHANGELOG.md)
- [Migration Guide](../docs/11-MIGRATION-GUIDE.md)

---

## 📋 Version Comparison

### Feature Matrix

| Feature | v1.1.x | v5.0.x | v5.1.0 |
|---------|--------|--------|--------|
| **RequestContext** | Mutable | Mutable | Immutable ✅ |
| **Error Model** | Basic | 5 types | 15+ types ✅ |
| **Type Safety** | Loose strings | Strings | Strong types ✅ |
| **Validation** | Permissive | Basic | Strict ✅ |
| **Multi-Tenant** | Partial | Basic | Full ✅ |
| **Residency** | None | None | GDPR ✅ |
| **Configuration** | File-based | Env | Env (strict) ✅ |
| **Documentation** | Minimal | Basic | Comprehensive ✅ |
| **Security** | Not focused | Basic | Zero-Trust ✅ |

---

## 🎯 Version Selection Guide

### Use v5.1.0 If You:

✅ Building new SaaS platform  
✅ Need multi-tenant support  
✅ Require GDPR compliance  
✅ Want production-ready code  
✅ Need comprehensive documentation  
✅ Care about security  
✅ **Building anything new** (recommended)

### Use v5.0.x Only If:

⚠️ Already using it (upgrade ASAP)  
⚠️ Testing upgrade path  
⚠️ Have specific reason (not recommended)

### Never Use v1.1.x For New Projects

❌ Legacy  
❌ No support  
❌ Poor security  
❌ Minimal documentation  

---

## 📞 Support Timeline

### v5.1.0 (Current)
```
Dec 6, 2025  ├─ Release
             │
Dec 6, 2026  ├─ End of Active Development
             │
Dec 6, 2027  └─ End of Support (2 years)
```

**Support:** Full (bug fixes, security patches)  
**Security Patches:** Until Dec 6, 2027  

### v5.0.x
```
Jun 1, 2025  ├─ Release
             │
Dec 6, 2025  └─ End of Support (6 months)
```

**Status:** No longer supported  
**Action:** Upgrade to v5.1.0

### v1.1.x
```
Dec 1, 2025  ├─ Release
             │
Dec 6, 2025  └─ End of Support (5 days)
```

**Status:** No longer supported  
**Action:** Upgrade to v5.1.0

---

## 🔐 Security Support

### v5.1.0
- ✅ Security patches: Until Dec 6, 2027
- ✅ Bug fixes: Until Dec 6, 2027
- ✅ New features: Until Dec 6, 2026

### v5.0.x
- ❌ No security patches
- ❌ No bug fixes

### v1.1.x
- ❌ No security patches
- ❌ No bug fixes

---

## 📚 Documentation by Version

### v5.1.0
- ✅ Comprehensive (14 guides, 35,000+ lines)
- ✅ Multiple learning paths
- ✅ 150+ code examples
- ✅ Migration guide
- ✅ Troubleshooting
- **[View Docs](../docs/INDEX.md)**

### v5.0.x
- ⚠️ Basic documentation
- ⚠️ Limited examples
- **Recommend upgrading to v5.1.0**

### v1.1.x
- ❌ Minimal documentation
- ❌ Source code only
- **Recommend upgrading to v5.1.0**

---

## 🔍 Detailed Change Log

### v5.1.0 Complete Changelog

**[See Full CHANGELOG.md](../CHANGELOG.md)**

Includes:
- All added features (15+)
- Breaking changes from v1.x
- Bug fixes (4 major)
- Security improvements (8)
- Performance metrics
- Acknowledgments

### v5.0.x Changelog

**Status:** Outdated, see v5.1.0 migration notes

### v1.1.x Changelogs

**v1.1.4:**
- Validation-aware TenantId/RegionId
- Email deserialization validation
- AppConfig error reporting

**v1.1.2:**
- ConnectionMetadata inactivity tracking
- PresenceStatus Display impl

**v1.1.1:**
- Code cleanup
- Removed outdated dependencies
- UUIDv7 only

---

## ✅ Checklist: Upgrade to v5.1.0

### Before Upgrade
- [ ] Read migration guide
- [ ] Backup current code
- [ ] Review breaking changes
- [ ] Estimate effort (3-5 hours for v1.x)

### During Upgrade
- [ ] Update Cargo.toml
- [ ] Fix compilation errors
- [ ] Update RequestContext usage
- [ ] Update error handling
- [ ] Update tests

### After Upgrade
- [ ] Run full test suite
- [ ] Review error handling
- [ ] Test in staging
- [ ] Deploy to production

---

## 🤔 FAQ

### Q: Is v5.1.0 backwards compatible with v1.1.x?
**A:** No. It's a major version change. Use migration guide.

### Q: Should I upgrade from v5.0.x to v5.1.0?
**A:** Yes, immediately. v5.0.x is no longer supported.

### Q: Can I stay on v1.1.4?
**A:** Not recommended. It's legacy and unsupported.

### Q: How long is v5.1.0 supported?
**A:** Until December 6, 2027 (2 years from release).

### Q: What happens after support ends?
**A:** You can still use it, but no patches. A new v5.2+ may exist.

### Q: Where do I report security issues?
**A:** See README.md for security contact info.

---

## 🚀 Recommended Action

### For Everyone

**Upgrade to v5.1.0 now:**

```bash
# Update Cargo.toml
rhelma-core = "5.1"

# Follow migration guide
cargo check
cargo test --all-features
```

**Why:**
- ✅ Current version (Dec 6, 2025)
- ✅ Production-ready
- ✅ Secure by default
- ✅ Fully documented
- ✅ 2 years support

---

## 📖 Further Reading

- **[README.md](../README.md)** — Project overview
- **[CHANGELOG.md](../CHANGELOG.md)** — Detailed changes
- **[Migration Guide](../docs/11-MIGRATION-GUIDE.md)** — v1.x → v5.1 upgrade
- **[Documentation Index](../docs/INDEX.md)** — All guides

---

**Last Updated:** December 6, 2025  
**Current Version:** 5.1.0  
**Status:** ✅ Production-Ready







