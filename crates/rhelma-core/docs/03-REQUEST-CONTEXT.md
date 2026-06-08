# RequestContext v5.2 Preview — Zero-Trust Identity (Internal)

**Document:** 03-REQUEST-CONTEXT.md  
**Version:** 5.1.x (documenting v5.2 preview APIs)  
**Status:** Preview / Internal (not for public production yet)

---

## Table of Contents

1. [Overview](#overview)
2. [Core Principles](#core-principles)
3. [Structure](#structure)
4. [Creation](#creation)
5. [Headers](#headers)
6. [Accessors](#accessors)
7. [Builders](#builders)
8. [Propagation](#propagation)
9. [Security](#security)
10. [Examples](#examples)
11. [Best Practices](#best-practices)

---

## Overview

**RequestContext v5.2 preview** (implemented as `RequestContextV52`) is an immutable, zero-trust object carrying request metadata across all layers:

- 🔐 **Immutable** — All fields private, cannot be changed
- 🆔 **Identity** — Tenant, user, region, device
- 🔒 **Security** — Scopes, roles, MFA level
- 📍 **Tracing** — request_id, correlation_id, trace_id
- 🌍 **Multi-region** — Region, residency validation
- ✅ **Validated** — Headers validated on construction

## When to use the v5.2 preview

- Internal services and gateways where you want **strict** header validation.
- Services adopting the **canonical** `x-rhelma-*` headers + UUIDv7.
- Environments where you can roll out changes safely and monitor failures.

**Do not require v5.2 preview** for general v5.1 production usage.

**Every Rhelma request MUST include a RequestContext (v5.1).**

> If you opt into the v5.2 preview, use `RequestContextV52` at boundaries where you need stricter header enforcement.

---


## Differences from v5.1 (only if you adopt v5.2 preview)

- Header names are now **canonicalized** to `x-rhelma-request-id` and `x-rhelma-correlation-id` (UUIDv7 expected).
- External entrypoints MUST also enforce `traceparent` and `x-residency` (see [Headers](#headers)).
- Legacy aliases (`x-request-id`, `x-correlation-id`) may be accepted temporarily for internal traffic, but should be treated as deprecated.

## Core Principles

### 1. Zero-Trust (Never Trust, Always Verify)

Every field comes from **untrusted input** (HTTP headers):

```rust
// ✅ All validated
RequestContext::from_headers(untrusted_headers)?

// ❌ No assumptions
// We don't assume valid format; we validate
```

### 2. Immutability

Once created, RequestContext cannot be modified:

```rust
// ✅ Build it correctly from the start
let ctx = RequestContext::empty()
    .with_tenant(tenant)?
    .with_region(region)?;

// ❌ Cannot modify after creation
// ctx.tenant_id = Some(other_tenant);  // Compile error!
```

### 3. Safe Sharing

RequestContext can be safely shared across:

```rust
// ✅ Shared with clone
let ctx_clone = ctx.clone();
tokio::spawn(async move {
    use_context(&ctx_clone).await;  // Safe, immutable
});

// ✅ Passed to multiple services
service_a(&ctx)?;
service_b(&ctx)?;

// No data races possible
```

### 4. Transparent Propagation

Every layer receives full context:

```
HTTP Request
    ↓
Parse RequestContext (from headers)
    ↓
Service Handler (receives ctx)
    ↓
Database Query (filters by tenant_id from ctx)
    ↓
Event Published (includes correlation_id from ctx)
    ↓
Event Consumer (extracts tenant_id from event)
```

---

## Structure

### Full Schema

```rust
pub struct RequestContext {
    // Tracing & Correlation
    request_id: Uuid,                    // Unique per request
    correlation_id: Option<String>,      // End-to-end tracking
    trace: Option<TraceContext>,         // W3C traceparent

    // Tenancy & Residency
    tenant_id: Option<TenantId>,         // Validated string
    region: Option<RegionId>,            // Validated string

    // Identity
    user_id: Option<UserId>,             // UUID
    user_email: Option<Email>,           // RFC 5322 validated
    session_id: Option<String>,          // Session identifier

    // Zero-Trust Security Metadata
    client_ip: Option<String>,           // Source IP
    user_agent: Option<String>,          // Browser/client info
    device_id: Option<String>,           // Device identifier
    scopes: Vec<String>,                 // OAuth scopes
    roles: Vec<String>,                  // User roles
    mfa_level: Option<String>,           // MFA status
    risk_level: Option<String>,          // Risk scoring

    // UX / Localization
    locale: Option<String>,              // Language/locale
}
```

### Field Categories

| Category | Fields | Required | Mutable |
|----------|--------|----------|---------|
| **Tracing** | request_id, correlation_id, trace | ✅ request_id | ❌ No |
| **Tenancy** | tenant_id, region | Depends | ❌ No |
| **Identity** | user_id, user_email, session_id | Depends | ❌ No |
| **Security** | client_ip, user_agent, device_id, scopes, roles, mfa_level, risk_level | Optional | ❌ No |
| **UX** | locale | Optional | ❌ No |

---

## Creation

### From HTTP Headers

```rust
pub fn from_headers<'a, H>(headers: H) -> Result<Self, RhelmaError>
where
    H: IntoIterator<Item = (&'a str, &'a str)>
```

Parses RequestContext from HTTP headers with **strict validation**.

**Recognized Headers:**

| Header | Type | Validation | Required |
|--------|------|-----------|----------|
| `x-rhelma-request-id` | UUID | UUID v4/v7 format | Optional* |
| `x-rhelma-correlation-id` | String | Any string | Optional |
| `x-tenant-id` | String | TenantId format | Optional |
| `x-region` | String | RegionId format | Optional |
| `x-user-id` | UUID | UUID format | Optional |
| `x-user-email` | String | RFC 5322 | Optional |
| `x-session-id` | String | Any string | Optional |
| `x-client-ip` | String | Any string | Optional |
| `x-user-agent` | String | Any string | Optional |
| `x-device-id` | String | Any string | Optional |
| `x-locale` | String | Any string | Optional |

**Note:** Missing `x-rhelma-request-id` generates random UUID v4

**Example:**

```rust
let headers = vec![
    ("x-rhelma-request-id", "550e8400-e29b-41d4-a716-446655440000"),
    ("x-rhelma-correlation-id", "abc-123"),
    ("x-tenant-id", "acme-corp"),
    ("x-region", "eu-west-1"),
    ("x-user-id", "550e8400-e29b-41d4-a716-446655440001"),
    ("x-user-email", "alice@example.com"),
];

let ctx = RequestContext::from_headers(headers)?;
// ✅ All fields validated and set
```

### Default (Empty)

```rust
pub fn empty() -> Self
```

Creates context with:
- Random `request_id` (UUID v4)
- All other fields: None/empty

**Use for:** Testing, background tasks

```rust
let ctx = RequestContext::empty();
assert!(ctx.request_id() != Uuid::nil());  // Has random ID
assert!(ctx.tenant_id().is_none());        // No tenant
```

### Header Parsing Behavior

| Scenario | Behavior |
|----------|----------|
| Header missing | Field is None (no error) |
| Header present, invalid format | RhelmaError::BadRequest |
| Invalid UUID in x-rhelma-request-id | RhelmaError::BadRequest |
| Invalid TenantId format | Field ignored (not set) |
| Invalid RegionId format | Field ignored (not set) |
| Invalid Email | Field ignored (not set) |

**Example: Invalid UUID**

```rust
let headers = vec![
    ("x-rhelma-request-id", "not-a-uuid"),  // ❌ Invalid
];

let result = RequestContext::from_headers(headers);
assert!(result.is_err());  // Returns RhelmaError::BadRequest
```

**Example: Invalid TenantId**

```rust
let headers = vec![
    ("x-tenant-id", "INVALID-NAME"),  // ❌ Uppercase not allowed
];

let ctx = RequestContext::from_headers(headers)?;
assert!(ctx.tenant_id().is_none());  // Field ignored, no error
```

---

## Headers

### Standard Rhelma Headers

**Full Header Set:**

```bash
# Tracing (required)
x-rhelma-request-id: 018d3c9f-2f4a-7d26-9d6f-5e6f8f4e1d10     # UUIDv7
x-rhelma-correlation-id: 018d3ca0-3b2f-7c11-9a5d-2f1a0c9b8d22 # UUIDv7
traceparent: 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01

# Tenancy & Residency (required for external HTTP APIs)
x-tenant-id: acme-corp
x-region: eu-west-1
x-residency: REGIONAL_STRICT   # GLOBAL | REGIONAL_PREFERRED | REGIONAL_STRICT

# Optional identity / auth
x-user-id: 550e8400-e29b-41d4-a716-446655440000
x-scopes: read:files write:files
x-roles: admin
```

### Recommended Minimum

For most services, include:

```bash
x-rhelma-request-id: <uuid>           # Always
x-tenant-id: <validated-id>    # If multi-tenant
x-region: <region>              # If multi-region
x-user-id: <uuid>               # If authenticated
```

### Gateway Responsibility

API Gateway SHOULD:

1. ✅ Extract from incoming request
2. ✅ Validate header format
3. ✅ Generate or validate x-rhelma-request-id (UUIDv7)
4. ✅ Propagate to all downstream services
5. ✅ Return in response headers (for tracking)

```rust
// Gateway middleware
async fn gateway_middleware(req: Request) -> Result<Request, RhelmaError> {
    // Extract or generate request ID
    let request_id = req
        .headers()
        .get("x-rhelma-request-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or_else(|| Uuid::new_v4());
    
    // Ensure request ID is set
    if req.headers().get("x-rhelma-request-id").is_none() {
        req.headers_mut().insert(
            "x-rhelma-request-id",
            request_id.to_string().parse()?,
        );
    }
    
    Ok(req)
}
```

---

## Accessors

### Getter Methods

All fields are private; access via methods:

```rust
// Tracing
pub fn request_id(&self) -> Uuid
pub fn correlation_id(&self) -> Option<&str>

// Tenancy
pub fn tenant_id(&self) -> Option<&TenantId>
pub fn region(&self) -> Option<&RegionId>

// Identity
pub fn user_id(&self) -> Option<&UserId>
pub fn user_email(&self) -> Option<&Email>

// Predicates
pub fn has_tenant(&self) -> bool
pub fn has_region(&self) -> bool

// UX
pub fn locale(&self) -> Option<&str>
```

### Example Usage

```rust
// Always have request_id
let id = ctx.request_id();
println!("Request: {}", id);

// Optionally have tenant
if let Some(tenant) = ctx.tenant_id() {
    println!("Tenant: {}", tenant.as_str());
} else {
    println!("No tenant specified");
}

// Check predicates
if ctx.has_tenant() && ctx.has_region() {
    println!("Multi-tenant request");
}

// Get locale or default
let locale = ctx.locale().unwrap_or("en-US");
```

### ⚠️ What's NOT Exposed

For security, these are private and cannot be accessed:

- `scopes` vector (add via builder)
- `roles` vector (add via builder)
- `mfa_level` (set once)
- `risk_level` (for internal use)

Access them by cloning and rebuilding:

```rust
// ❌ Cannot access
let scopes = &ctx.scopes;  // Compile error

// ✅ Only way: rebuild with scope
let ctx_with_scope = ctx.clone()
    .add_scope("read:invoices");
```

---

## Builders

### Fluent Builder Pattern

Create context step-by-step:

```rust
let ctx = RequestContext::empty()
    .with_tenant(TenantId::parse("acme-corp")?)
    .with_region(RegionId::parse("eu-west-1")?)
    .with_user(UserId::new(), Some(email))
    .with_locale("en-US")
    .add_scope("read:invoices")
    .add_scope("write:payments")
    .add_role("admin");
```

### Builder Methods

```rust
pub fn with_tenant(self, t: TenantId) -> Self
pub fn with_region(self, r: RegionId) -> Self
pub fn with_user(self, id: UserId, email: Option<Email>) -> Self
pub fn with_locale<S: Into<String>>(self, loc: S) -> Self
pub fn add_scope<S: Into<String>>(self, s: S) -> Self
pub fn add_role<S: Into<String>>(self, r: S) -> Self
```

### Immutability Guarantee

Builders return **new instances**, not mutable references:

```rust
let ctx1 = RequestContext::empty();
let ctx2 = ctx1.with_tenant(tenant)?;

// ctx1 is unchanged
assert!(ctx1.tenant_id().is_none());

// ctx2 has tenant
assert!(ctx2.tenant_id().is_some());
```

### Real-World Example

```rust
// Build from validated inputs
let ctx = RequestContext::empty()
    .with_tenant(TenantId::parse(tenant_param)?)
    .with_region(RegionId::parse(region_param)?)
    .with_user(
        UserId::parse(user_id_param)?,
        Email::parse(email_param).ok(),
    )
    .add_scope("read:data")
    .add_scope("write:data")
    .add_role(user_role);
```

---

## Propagation

### Across HTTP Boundaries

```rust
// Service A (receives request)
async fn handler_a(ctx: RequestContext) -> Result<Response> {
    // Service B is internal
    let response = call_service_b(&ctx).await?;
    Ok(response)
}

// Service B (receives context)
async fn call_service_b(ctx: &RequestContext) -> RhelmaResult<Data> {
    // Include context in outbound request
    let headers = vec![
        ("x-rhelma-request-id", ctx.request_id().to_string()),
        ("x-rhelma-correlation-id", ctx.correlation_id().unwrap_or("").to_string()),
        ("x-tenant-id", ctx.tenant_id().map(|t| t.as_str()).unwrap_or("")),
    ];
    
    http_client.post(url)
        .headers(headers)
        .send()
        .await?
}
```

### Across Async Boundaries

```rust
// Capture context in closure
let ctx = RequestContext::empty().with_tenant(tenant)?;

tokio::spawn({
    let ctx = ctx.clone();  // Clone for async move
    async move {
        process_in_background(&ctx).await;
    }
});

// Context is safe to share (immutable)
```

### In Events

```rust
// Publish event with context
let event = Event {
    request_id: ctx.request_id().to_string(),
    correlation_id: ctx.correlation_id().map(|s| s.to_string()),
    tenant_id: ctx.tenant_id().map(|t| t.as_str().to_string()),
    // ... event payload
};

event_bus.publish(&event).await?;
```

### In Database Queries

```rust
// All queries must filter by tenant
async fn get_invoice(
    ctx: &RequestContext,
    pool: &PgPool,
    invoice_id: &str,
) -> RhelmaResult<Invoice> {
    let tenant = ctx.tenant_id()
        .ok_or_else(|| RhelmaError::Auth("missing tenant".into()))?;
    
    sqlx::query_as::<_, Invoice>(
        "SELECT * FROM invoices WHERE id = $1 AND tenant_id = $2"
    )
    .bind(invoice_id)
    .bind(tenant.as_str())  // From context, always included
    .fetch_one(pool)
    .await?
}
```

### In Logs

```rust
use tracing::{info, warn, error};

fn process_request(ctx: &RequestContext, data: &Data) -> RhelmaResult<()> {
    info!(
        request_id = %ctx.request_id(),
        tenant_id = ?ctx.tenant_id().map(|t| t.as_str()),
        "Processing request"
    );
    
    match operation() {
        Ok(result) => {
            info!(
                request_id = %ctx.request_id(),
                "Operation succeeded"
            );
        }
        Err(err) => {
            warn!(
                request_id = %ctx.request_id(),
                error = ?err,
                "Operation failed"
            );
        }
    }
    
    Ok(())
}
```

---

## Security

### Immutability

RequestContext fields cannot be modified:

```rust
// ✅ Build correctly from start
let ctx = RequestContext::empty()
    .with_tenant(verified_tenant)?;

// ❌ Cannot modify
// ctx.tenant_id = Some(other_tenant);  // Compile error

// ❌ No mutable reference possible
// let ctx_mut = &mut ctx;  // Cannot get mut ref
```

### Validation

All input fields validated:

```rust
// UUID must be valid
RequestContext::from_headers(vec![
    ("x-rhelma-request-id", "invalid"),  // ❌ RhelmaError::BadRequest
])?;

// Tenant ID must follow format
RequestContext::from_headers(vec![
    ("x-tenant-id", "INVALID"),  // ❌ Ignored (not set)
])?;

// Email must be RFC 5322
RequestContext::from_headers(vec![
    ("x-user-email", "not-email@"),  // ❌ Ignored (not set)
])?;
```

### Isolation

Each context is independent:

```rust
let ctx1 = RequestContext::empty()
    .with_tenant(TenantId::parse("tenant-1")?);

let ctx2 = RequestContext::empty()
    .with_tenant(TenantId::parse("tenant-2")?);

// Cannot mix them
// ctx1 == ctx2  // False, different tenants
```

### No Secrets

RequestContext should NOT carry:

- ❌ Passwords
- ❌ API keys
- ❌ OAuth tokens
- ❌ Private data
- ❌ PII (except email)

---

## Examples

### Example 1: HTTP Handler

```rust
use axum::{extract::Extension, Json};
use rhelma_core::prelude::*;

#[derive(serde::Deserialize)]
struct CreateInvoiceRequest {
    amount: f64,
    customer_email: String,
}

async fn create_invoice(
    Extension(ctx): Extension<RequestContext>,
    Json(req): Json<CreateInvoiceRequest>,
) -> Result<Json<Invoice>, RhelmaError> {
    // Validate tenant exists
    let tenant = ctx.tenant_id()
        .ok_or_else(|| RhelmaError::Auth("missing tenant".into()))?;
    
    // Validate email
    Email::parse(&req.customer_email)
        .rhelma_context("while validating email")?;
    
    // Create invoice for this tenant
    let invoice = Invoice::create(tenant, &req)
        .await
        .rhelma_context("while creating invoice")?;
    
    // Log with context
    info!(
        request_id = %ctx.request_id(),
        tenant_id = tenant.as_str(),
        invoice_id = %invoice.id,
        "Invoice created"
    );
    
    Ok(Json(invoice))
}
```

### Example 2: Middleware Extraction

```rust
use axum::{extract::Request, middleware::Next, response::Response};

async fn extract_context(
    mut req: Request,
    next: Next,
) -> Response {
    // Extract headers
    let headers: Vec<(&str, &str)> = req
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            value.to_str().ok().map(|v| (name.as_str(), v))
        })
        .collect();
    
    // Parse context
    match RequestContext::from_headers(headers) {
        Ok(ctx) => {
            // Store in extensions
            req.extensions_mut().insert(ctx.clone());
            next.run(req).await
        }
        Err(err) => {
            warn!("Failed to parse RequestContext: {}", err);
            (axum::http::StatusCode::BAD_REQUEST, err.to_string()).into_response()
        }
    }
}
```

### Example 3: Multi-Service Call

```rust
async fn process_order(
    ctx: &RequestContext,
    order_id: &str,
) -> RhelmaResult<Order> {
    let tenant = ctx.tenant_id()
        .ok_or_else(|| RhelmaError::Auth("missing tenant".into()))?;
    
    // Call payment service
    let payment_result = call_payment_service(&ctx, order_id).await?;
    
    // Call fulfillment service
    let fulfillment = call_fulfillment_service(&ctx, order_id).await?;
    
    // Both services receive same context
    Ok(Order {
        id: order_id.to_string(),
        payment: payment_result,
        fulfillment,
        tenant_id: tenant.as_str().to_string(),
    })
}

async fn call_payment_service(
    ctx: &RequestContext,
    order_id: &str,
) -> RhelmaResult<Payment> {
    // Propagate context headers
    let response = http_client
        .post("http://payment:8080/charge")
        .header("x-rhelma-request-id", ctx.request_id().to_string())
        .header("x-rhelma-correlation-id", ctx.correlation_id().unwrap_or(""))
        .header("x-tenant-id", ctx.tenant_id().map(|t| t.as_str()).unwrap_or(""))
        .json(&json!({ "order_id": order_id }))
        .send()
        .await?;
    
    response.json().await?
}
```

---

## Best Practices

### ✅ Do's

1. **Always extract from headers** at API boundary
   ```rust
   let ctx = RequestContext::from_headers(headers)?;
   ```

2. **Propagate to all services**
   ```rust
   service_a(&ctx)?;
   service_b(&ctx)?;
   ```

3. **Include in logs**
   ```rust
   info!(request_id = %ctx.request_id(), "...");
   ```

4. **Filter by tenant**
   ```rust
   SELECT * FROM data WHERE tenant_id = ctx.tenant_id()?
   ```

5. **Validate required fields**
   ```rust
   let tenant = ctx.tenant_id()
       .ok_or(RhelmaError::Auth("missing tenant".into()))?;
   ```

### ❌ Don'ts

1. **Don't create new context mid-request**
   ```rust
   // ❌ Bad: loses tracing context
   let new_ctx = RequestContext::empty();
   
   // ✅ Good: reuse existing
   process_with_context(&ctx);
   ```

2. **Don't store secrets in context**
   ```rust
   // ❌ Bad: leaks API key
   ctx.with_secret("api_key", key)?;
   
   // ✅ Good: use service-to-service auth
   ```

3. **Don't modify context during request**
   ```rust
   // ❌ Cannot modify (fields private)
   ctx.tenant_id = Some(other);
   
   // ✅ Build new context if needed
   let ctx2 = RequestContext::empty()
       .with_tenant(other)?;
   ```

4. **Don't skip validation**
   ```rust
   // ❌ Bad: trusts headers
   let tenant = req.header("x-tenant-id");
   
   // ✅ Good: validates
   let ctx = RequestContext::from_headers(headers)?;
   let tenant = ctx.tenant_id()?;
   ```

---

**Last Updated:** December 6, 2025











