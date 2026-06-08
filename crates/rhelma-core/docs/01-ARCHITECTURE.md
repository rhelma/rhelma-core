# rhelma-core Architecture v5.1

**Document:** 01-ARCHITECTURE.md  
**Version:** 5.1.0  
**Status:** Final

---

## Table of Contents

1. [Overview](#overview)
2. [Core Principles](#core-principles)
3. [Module Structure](#module-structure)
4. [RequestContext Flow](#requestcontext-flow)
5. [Type System](#type-system)
6. [Error Handling Architecture](#error-handling-architecture)
7. [Configuration Model](#configuration-model)
8. [Zero-Trust Design](#zero-trust-design)
9. [Integration Points](#integration-points)

---

## Overview

**rhelma-core** is the foundational library for Rhelma platform services. It provides:

- **RequestContext v5.1** — Immutable, zero-trust request metadata
- **Unified Error Model** — 15+ error variants with stable labels
- **Strong Type System** — Validated identifiers preventing category errors
- **Configuration Management** — Strict, deterministic config loading
- **Multi-Tenant Governance** — Tenancy tiers, residency enforcement
- **Observability Foundation** — Structured config for logs, metrics, traces

**Every Rhelma service MUST use rhelma-core v5.1.**

---

## Core Principles

### 1. Zero-Trust Everywhere

All data is untrusted until validated:

- Every header parsed → Result type
- Every identifier validated → ParseError on invalid
- Every field immutable → Prevents accidental mutation
- Every access logged → Audit trail maintained

### 2. Fail Fast, Fail Safe

- Invalid config → Startup abort (no silent fallback)
- Invalid request → Clear error response
- Invalid identifier → Type error at compile-time
- Invalid credentials → Auth error immediately

### 3. Type Safety Over Stringly Typing

- `TenantId` ≠ `UserId` (compile error if mixed)
- `Email` validates RFC 5322 on construction
- `RegionId` enforces lowercase alphanumeric format
- No string-based error codes (use enums)

### 4. Observability as First-Class

- Every operation traced
- Every request has request_id + correlation_id
- Every error has stable label
- Every transaction auditable

### 5. Simplicity & Determinism

- One way to do things (no magic)
- Deterministic behavior (same input → same output)
- Minimal dependencies (anyhow, serde, chrono, uuid only)
- Explicit over implicit (no hidden side effects)

---

## Module Structure

```
rhelma-core/
│
├── config.rs
│   ├── AppConfig              (Load from environment)
│   ├── from_env_only()        (Raw loading, no validation)
│   └── validate_all()         (Strict validation)
│
├── error.rs
│   ├── RhelmaError              (15+ variants)
│   ├── ValidationError
│   ├── ErrorExt trait         (rhelma_context() method)
│   └── HTTP mapping (axum feature)
│
├── request_context.rs
│   ├── RequestContext         (Immutable, private fields)
│   ├── from_headers()         (Parse from HTTP headers)
│   ├── empty()                (Default context)
│   └── Builder API            (with_tenant, with_region, etc.)
│
├── tenancy.rs
│   ├── TenantProfile          (Tenant metadata)
│   ├── TenancyTier            (Isolation levels)
│   ├── ResidencyPolicy        (Data residency rules)
│   └── validate_residency()   (Enforcement)
│
├── types/
│   ├── ids.rs
│   │   ├── UserId             (UUID-based)
│   │   ├── TenantId           (Validated string)
│   │   ├── RegionId           (Validated string)
│   │   └── Email              (RFC 5322 validated)
│   │
│   ├── pagination.rs
│   │   ├── PageRequest        (Offset/limit)
│   │   └── Paginated<T>       (Response wrapper)
│   │
│   └── common.rs
│       └── RuntimeEnvironment (development/staging/production)
│
├── observability.rs
│   ├── UnifiedObservabilityConfig
│   ├── Environment enum
│   ├── LogFormat enum
│   └── from_app_config()
│
├── realtime_types.rs
│   ├── RealtimeSessionId      (UUID-based)
│   ├── ConnectionMetadata     (Session tracking)
│   ├── PresenceStatus         (Online/Away/Offline)
│   └── RoomId                 (Validated)
│
├── security.rs
│   ├── PasswordPolicy         (Configurable validation)
│   └── RateLimitKeyBuilder    (Namespaced keys)
│
├── trace_context.rs
│   ├── TraceContext           (W3C traceparent)
│   ├── from_traceparent()
│   └── extract_from_headers()
│
├── http_error.rs
│   ├── HttpErrorBody          (Standard error response)
│   └── HttpErrorMapping trait
│
├── result.rs
│   └── RhelmaResult<T>          (Type alias for Result<T, RhelmaError>)
│
├── traits/
│   └── extensions.rs
│       └── ErrorExt, ResultExt
│
└── prelude.rs
    └── Re-exports (use rhelma_core::prelude::*;)
```

---

## RequestContext Flow

### 1. Request Arrives (HTTP Gateway)

```
GET /api/invoices
Headers:
  x-request-id: 550e8400-e29b-41d4-a716-446655440000
  x-correlation-id: abc-123
  x-tenant-id: acme-corp
  x-region: eu-west-1
  x-user-id: user-uuid
```

### 2. Parse Context (Middleware)

```rust
let ctx = RequestContext::from_headers(headers)?;
//        └─ Validates UUID format
//        └─ Validates tenant ID format
//        └─ Validates region format
//        └─ Returns Result (strict validation)
```

### 3. Check Immutability

```rust
// ✅ Works (immutable builders)
let ctx = RequestContext::empty()
    .with_tenant(TenantId::parse("acme-corp")?)
    .with_region(RegionId::parse("eu-west-1")?);

// ❌ Compile error (fields are private)
// ctx.tenant_id = Some(tid);
```

### 4. Access via Methods

```rust
if let Some(tenant) = ctx.tenant_id() {
    process_for_tenant(tenant);
}
```

### 5. Propagate (Next Service/Event)

```rust
// In async task
let task_ctx = ctx.clone();
tokio::spawn(async move {
    // ctx is immutable, safe to share
});

// In event envelope
event.correlation_id = Some(ctx.correlation_id().to_string());
event.tenant_id = ctx.tenant_id().map(|t| t.as_str().to_string());

// In next HTTP request
outbound_headers.insert("x-request-id", ctx.request_id().to_string());
```

---

## Type System

### Strong Identifiers

**Problem (v1.x):**
```rust
fn process(user_id: String, tenant_id: String) {
    // Easy to mix them up
    db.find(tenant_id)?;  // Bug! Used tenant as user
}
```

**Solution (v5.1):**
```rust
fn process(user_id: UserId, tenant_id: TenantId) {
    // Compile error if types swapped
    db.find(tenant_id)?;  // ✅ Correct type
}
```

### Validation

All strong types validate on construction:

```rust
// TenantId (lowercase alphanumeric + `-`)
TenantId::parse("acme-corp")?       // ✅ Valid
TenantId::parse("ACME-CORP")?       // ❌ uppercase
TenantId::parse("acme corp")?       // ❌ space
TenantId::parse("ac")?              // ❌ too short (min 3)

// Email (RFC 5322)
Email::parse("alice@example.com")?  // ✅ Valid
Email::parse("user@")?              // ❌ Invalid
Email::parse("@domain.com")?        // ❌ Invalid

// RegionId (lowercase alphanumeric + `-`)
RegionId::parse("us-west-2")?       // ✅ Valid
RegionId::parse("US-WEST-2")?       // ❌ uppercase
RegionId::parse("usw")?             // ✅ Valid (min 3)
```

---

## Error Handling Architecture

### Error Categories

```rust
pub enum RhelmaError {
    // Configuration
    Config(String),
    
    // Validation
    Validation(String),
    BadRequest(String),
    
    // Authentication & Authorization
    Auth(String),
    Authz(String),
    
    // Resource State
    NotFound(String),
    Conflict(String),
    
    // Rate Limiting & Availability
    RateLimited(String),
    CircuitOpen(String),
    
    // Distributed Systems
    Dependency(String),
    DistributedTx(String),
    
    // Security
    SecurityPolicy(String),
    
    // Data Layer
    Database(String),
    Cache(String),
    
    // Fallback
    Internal,
    Other(anyhow::Error),
}
```

### Error Context Propagation

```rust
fn operation() -> RhelmaResult<Data> {
    load_config()
        .rhelma_context("while loading config")?
        
    fetch_tenant()
        .rhelma_context("while fetching tenant")?
        
    validate_data()
        .rhelma_context("while validating data")?
}

// Error output:
// RhelmaError::Config("connection timeout (while loading config)")
// RhelmaError::NotFound("tenant not found (while fetching tenant)")
// RhelmaError::Validation("invalid email (while validating data)")
```

### HTTP Mapping

```rust
#[cfg(feature = "axum")]
impl IntoResponse for RhelmaError {
    // RhelmaError::Validation → 400 BAD_REQUEST
    // RhelmaError::Auth → 401 UNAUTHORIZED
    // RhelmaError::Authz → 403 FORBIDDEN
    // RhelmaError::NotFound → 404 NOT_FOUND
    // RhelmaError::RateLimited → 429 TOO_MANY_REQUESTS
    // RhelmaError::Internal → 500 INTERNAL_SERVER_ERROR
    // etc.
}
```

---

## Configuration Model

### Load Phase (No Validation)

```rust
let cfg = AppConfig::from_env_only()?;
// Returns raw config with defaults:
//   RHELMA_ENV=development
//   RHELMA_REGION=local
//   json_logs=false
```

### Validate Phase (Strict Checks)

```rust
cfg.validate_all()?;
// Validates:
//   environment ∈ {development, staging, production}
//   region matches [a-z0-9-]{3,}
//   service_name not empty (in production)
```

### Usage Phase (After Validation)

```rust
println!("Service {} in {}/{}", 
    cfg.service_name.as_deref().unwrap_or("unknown"),
    cfg.environment,
    cfg.region
);
```

---

## Zero-Trust Design

### Principle: Never Trust, Always Verify

Every layer validates:

1. **Network Layer:** TLS 1.3 required
2. **API Layer:** RequestContext mandatory
3. **Authentication Layer:** JWT + signature verification
4. **Authorization Layer:** PBAC policies
5. **Data Layer:** Tenant ID validation
6. **Business Logic:** Type system checks

### Request Timeline

```
Gateway receives request
    ↓
Parse RequestContext (validate headers)
    ↓
Verify authentication (JWT signature)
    ↓
Check authorization (PBAC)
    ↓
Validate tenancy (tenant_id matches auth)
    ↓
Enforce residency (data in allowed region)
    ↓
Execute business logic
```

---

## Integration Points

### With HTTP Framework (Axum)

```rust
use axum::extract::Request;
use rhelma_core::prelude::*;

async fn extract_context(req: &Request) -> RhelmaResult<RequestContext> {
    let headers: Vec<(&str, &str)> = req
        .headers()
        .iter()
        .map(|(k, v)| (k.as_str(), v.to_str().unwrap_or("")))
        .collect();
    
    RequestContext::from_headers(headers)
        .rhelma_context("while parsing headers")
}
```

### With Database (sqlx)

```rust
#[cfg(feature = "sqlx")]
impl From<sqlx::Error> for RhelmaError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => 
                RhelmaError::NotFound("...".into()),
            _ => RhelmaError::Database(err.to_string()),
        }
    }
}
```

### With Event System (Document 04)

```rust
event.request = Some(RequestEnvelope {
    request_id: ctx.request_id().to_string(),
    correlation_id: ctx.correlation_id().map(|s| s.to_string()),
    tenant_id: ctx.tenant_id().map(|t| t.as_str().to_string()),
    trace_id: trace_context.trace_id.clone(),
});
```

### With Tracing (OpenTelemetry)

```rust
use tracing::Span;

let span = tracing::info_span!(
    "request_handler",
    request_id = %ctx.request_id(),
    tenant_id = ?ctx.tenant_id().map(|t| t.as_str()),
    correlation_id = ctx.correlation_id(),
);
```

---

## Conclusion

rhelma-core v5.1 provides the foundation for:

- **Type-safe services** (compiler-checked identifiers)
- **Secure services** (zero-trust RequestContext)
- **Observable services** (immutable, traceable context)
- **Reliable services** (deterministic error handling)
- **Multi-tenant services** (strict tenancy enforcement)

**Every service that uses rhelma-core gains these properties automatically.**

---

**Last Updated:** December 6, 2025







