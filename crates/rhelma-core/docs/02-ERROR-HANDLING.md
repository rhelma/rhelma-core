# Error Handling v5.1

**Document:** 02-ERROR-HANDLING.md  
**Version:** 5.1.0  
**Status:** Final

---

## Table of Contents

1. [Overview](#overview)
2. [Error Model](#error-model)
3. [Error Categories](#error-categories)
4. [Context Propagation](#context-propagation)
5. [HTTP Mapping](#http-mapping)
6. [Error Labels](#error-labels)
7. [Best Practices](#best-practices)
8. [Examples](#examples)
9. [Troubleshooting](#troubleshooting)

---

## Overview

rhelma-core implements a **unified, type-safe error model** for the Rhelma Platform. Every error:

- ✅ Has a stable category (for observability)
- ✅ Maps to correct HTTP status code
- ✅ Can carry context without leaking secrets
- ✅ Is serializable for APIs
- ✅ Supports error chaining

**Philosophy:** Fail fast with clear, actionable errors.

---

## Error Model

### RhelmaError Enum

```rust
pub enum RhelmaError {
    // Configuration errors
    Config(String),                   // 500 INTERNAL_SERVER_ERROR
    
    // Validation & input
    Validation(String),               // 400 BAD_REQUEST
    BadRequest(String),               // 400 BAD_REQUEST
    
    // Authentication & Authorization
    Auth(String),                     // 401 UNAUTHORIZED
    Authz(String),                    // 403 FORBIDDEN
    
    // Resource state
    NotFound(String),                 // 404 NOT_FOUND
    Conflict(String),                 // 409 CONFLICT
    
    // Rate limiting & availability
    RateLimited(String),              // 429 TOO_MANY_REQUESTS
    CircuitOpen(String),              // 503 SERVICE_UNAVAILABLE
    
    // Distributed systems
    Dependency(String),               // 503 SERVICE_UNAVAILABLE
    DistributedTx(String),            // 500 INTERNAL_SERVER_ERROR
    
    // Security
    SecurityPolicy(String),           // 403 FORBIDDEN
    
    // Data layer
    Cache(String),                    // 500 INTERNAL_SERVER_ERROR
    Database(String),                 // 500 INTERNAL_SERVER_ERROR
    
    // Internal errors
    Internal,                         // 500 INTERNAL_SERVER_ERROR
    
    // Fallback for anyhow errors
    Other(anyhow::Error),             // 500 INTERNAL_SERVER_ERROR
}
```

### Key Properties

Each variant:
- Has clear HTTP status mapping
- Represents a specific failure mode
- Is serializable
- Carries a String message
- Has a stable label (via `.as_str()`)

---

## Error Categories

### Configuration Errors

When configuration is invalid or missing at startup.

```rust
RhelmaError::Config("database URL not set".into())
RhelmaError::Config("invalid region format: US-WEST".into())
RhelmaError::Config("environment must be one of [development, staging, production]".into())
```

**HTTP Status:** 500  
**Action:** Fix configuration, restart service  
**Example:**
```rust
let cfg = AppConfig::from_env_only()?;
cfg.validate_all()?;  // Returns Config error if invalid
```

### Validation Errors

Input data fails validation at boundaries.

```rust
RhelmaError::Validation("email address invalid".into())
RhelmaError::Validation("tenant_id must not be empty".into())
RhelmaError::Validation("password must be at least 8 characters".into())
```

**HTTP Status:** 400  
**Action:** Fix request, try again  
**Example:**
```rust
Email::parse(input)?;           // Returns Validation error
TenantId::parse(input)?;        // Returns Validation error
PasswordPolicy::default().validate(pwd)?;  // Returns Validation error
```

### BadRequest Errors

Generic bad request (malformed input, invalid parameters).

```rust
RhelmaError::BadRequest("missing required field: user_id".into())
RhelmaError::BadRequest("invalid request ID UUID format".into())
RhelmaError::BadRequest("page limit must be between 1 and 1000".into())
```

**HTTP Status:** 400  
**Action:** Fix request, try again  
**When to use:** Use when Validation is too specific

### Authentication Errors

Invalid credentials, expired tokens, or missing auth.

```rust
RhelmaError::Auth("invalid credentials".into())
RhelmaError::Auth("token expired".into())
RhelmaError::Auth("missing authentication header".into())
RhelmaError::Auth("invalid JWT signature".into())
```

**HTTP Status:** 401  
**Action:** Provide valid credentials  
**Never include:** Actual passwords, JWT tokens, API keys

### Authorization Errors

Insufficient permissions or scope.

```rust
RhelmaError::Authz("insufficient scope: write:invoices required".into())
RhelmaError::Authz("cannot access other tenant's data".into())
RhelmaError::Authz("user role 'admin' required".into())
```

**HTTP Status:** 403  
**Action:** Request higher permission level  
**Example:**
```rust
if !ctx.scopes.contains(&"read:invoices".to_string()) {
    return Err(RhelmaError::Authz("scope 'read:invoices' required".into()));
}
```

### NotFound Errors

Resource doesn't exist.

```rust
RhelmaError::NotFound("invoice 12345 not found".into())
RhelmaError::NotFound("user not found".into())
RhelmaError::NotFound("tenant configuration not found".into())
```

**HTTP Status:** 404  
**Action:** Verify resource exists, check ID  
**Example:**
```rust
let invoice = db.find_invoice(&id).await?
    .ok_or_else(|| RhelmaError::NotFound(format!("Invoice {} not found", id)))?;
```

### Conflict Errors

Resource state prevents operation (concurrent modification, duplicate).

```rust
RhelmaError::Conflict("email already registered".into())
RhelmaError::Conflict("invoice already paid".into())
RhelmaError::Conflict("concurrent update detected".into())
```

**HTTP Status:** 409  
**Action:** Resolve conflict, retry  
**Example:**
```rust
if user_exists(&email).await? {
    return Err(RhelmaError::Conflict("email already registered".into()));
}
```

### RateLimited Errors

Too many requests from this client/tenant.

```rust
RhelmaError::RateLimited("rate limit exceeded: 100 requests/minute".into())
RhelmaError::RateLimited("quota exceeded: 1000 API calls/day".into())
```

**HTTP Status:** 429  
**Action:** Retry after delay (exponential backoff)  
**Example:**
```rust
if rate_limiter.is_limited(&key).await? {
    return Err(RhelmaError::RateLimited("rate limit exceeded".into()));
}
```

### CircuitOpen Errors

Downstream service is failing (circuit breaker).

```rust
RhelmaError::CircuitOpen("AI service temporarily unavailable".into())
RhelmaError::CircuitOpen("payment processor circuit open".into())
```

**HTTP Status:** 503  
**Action:** Retry after delay, check dependent service health  
**Example:**
```rust
if breaker.is_open(&service_name).await? {
    return Err(RhelmaError::CircuitOpen(format!("{} unavailable", service_name)));
}
```

### Dependency Errors

External service failed.

```rust
RhelmaError::Dependency("database connection timeout".into())
RhelmaError::Dependency("cache service unavailable".into())
RhelmaError::Dependency("email service failed".into())
```

**HTTP Status:** 503  
**Action:** Retry with backoff, check dependent service  
**Example:**
```rust
db.connect().await
    .map_err(|e| RhelmaError::Dependency(e.to_string()))?
```

### DistributedTx Errors

Distributed transaction failed (saga failure).

```rust
RhelmaError::DistributedTx("saga step failed: payment processing".into())
RhelmaError::DistributedTx("compensation failed: refund rejected".into())
```

**HTTP Status:** 500  
**Action:** Manual intervention required  
**Example:**
```rust
if saga.status() == SagaStatus::Failed {
    return Err(RhelmaError::DistributedTx("saga execution failed".into()));
}
```

### SecurityPolicy Errors

Zero-trust violation or security policy breach.

```rust
RhelmaError::SecurityPolicy("region residency violation: data cannot leave EU".into())
RhelmaError::SecurityPolicy("AI usage not allowed for this tenant".into())
RhelmaError::SecurityPolicy("PII logging not allowed".into())
```

**HTTP Status:** 403  
**Action:** Fix compliance issue  
**Example:**
```rust
profile.validate_residency(&region)?;  // Returns SecurityPolicy error
```

### Cache Errors

Cache operation failed (not critical).

```rust
RhelmaError::Cache("redis connection timeout".into())
RhelmaError::Cache("cache invalidation failed".into())
```

**HTTP Status:** 500  
**Action:** Bypass cache, try primary storage  
**Note:** Cache errors should not fail operation; fallback to primary storage

### Database Errors

Database operation failed.

```rust
RhelmaError::Database("connection pool exhausted".into())
RhelmaError::Database("query timeout: 30s".into())
RhelmaError::Database("transaction rollback".into())
```

**HTTP Status:** 500  
**Action:** Retry with backoff, check database health  
**Example (with sqlx feature):**
```rust
#[cfg(feature = "sqlx")]
impl From<sqlx::Error> for RhelmaError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => 
                Self::NotFound("row not found".into()),
            _ => Self::Database(err.to_string()),
        }
    }
}
```

### Internal Errors

Unexpected internal error (bug).

```rust
RhelmaError::Internal
```

**HTTP Status:** 500  
**Action:** Log, investigate bug  
**Message:** No context (generic)  
**Use sparingly:** Prefer specific error types

---

## Context Propagation

### ErrorExt Trait

Attach context to errors without changing type.

```rust
pub trait ErrorExt: Sized {
    fn rhelma_context<C>(self, context: C) -> Self
    where
        C: std::fmt::Display + Send + Sync + 'static;
    
    fn context<C>(self, context: C) -> Self  // Alias
    where
        C: std::fmt::Display + Send + Sync + 'static;
}
```

### Usage Pattern

Chain operations with context:

```rust
fn load_tenant_config(id: &str) -> RhelmaResult<TenantProfile> {
    TenantProfile::fetch(id)
        .rhelma_context("while loading tenant configuration")
}

fn process_invoice(ctx: &RequestContext) -> RhelmaResult<Invoice> {
    validate_input(&ctx)
        .rhelma_context("during input validation")?;
    
    load_tenant(ctx.tenant_id()?)
        .rhelma_context("while loading tenant profile")?;
    
    create_invoice(ctx)
        .rhelma_context("while creating invoice")?;
    
    Ok(Invoice::default())
}
```

### Output Example

```
Error without context:
  RhelmaError::NotFound("invoice not found")

Error with context chain:
  RhelmaError::NotFound("invoice not found (while creating invoice) (while processing invoice)")
```

### ✅ Best Practice

Always add context at important boundaries:

```rust
// ✅ Good: Clear error context
database::connect()
    .rhelma_context("while connecting to database")?;

config::load()
    .rhelma_context("during configuration loading")?;

authorize_user(&ctx)
    .rhelma_context("during authorization check")?;

// ❌ Bad: No context
database::connect()?;
config::load()?;
authorize_user(&ctx)?;
```

---

## HTTP Mapping

### Status Code Mapping

| RhelmaError | HTTP Status | Semantics |
|-----------|------------|-----------|
| `Config` | 500 | Service misconfiguration |
| `Validation` | 400 | Invalid input data |
| `BadRequest` | 400 | Malformed request |
| `Auth` | 401 | Authentication failed |
| `Authz` | 403 | Permission denied |
| `NotFound` | 404 | Resource doesn't exist |
| `Conflict` | 409 | State conflict |
| `RateLimited` | 429 | Too many requests |
| `CircuitOpen` | 503 | Dependency unavailable |
| `Dependency` | 503 | External service failed |
| `SecurityPolicy` | 403 | Policy violation |
| `Cache` | 500 | Cache failure |
| `Database` | 500 | Database failure |
| `DistributedTx` | 500 | Saga failure |
| `Internal` | 500 | Internal error |
| `Other` | 500 | Unknown error |

### With Axum Feature

Automatic conversion to HTTP response:

```rust
#[cfg(feature = "axum")]
impl axum::response::IntoResponse for RhelmaError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_type, message) = match &self {
            RhelmaError::Validation(_) => (
                StatusCode::BAD_REQUEST,
                "validation",
                "Validation failed",
            ),
            RhelmaError::Auth(_) => (
                StatusCode::UNAUTHORIZED,
                "auth",
                "Authentication failed",
            ),
            // ... more mappings
        };
        
        let body = serde_json::json!({
            "error": {
                "code": status.as_u16(),
                "message": message,
                "type": error_type,
            }
        });
        
        (status, Json(body)).into_response()
    }
}
```

### Example Response Bodies

```json
// 400 Bad Request
{
  "error": {
    "code": 400,
    "type": "validation",
    "message": "Validation failed"
  }
}

// 401 Unauthorized
{
  "error": {
    "code": 401,
    "type": "auth",
    "message": "Authentication failed"
  }
}

// 403 Forbidden
{
  "error": {
    "code": 403,
    "type": "authz",
    "message": "Access forbidden"
  }
}

// 404 Not Found
{
  "error": {
    "code": 404,
    "type": "not_found",
    "message": "Resource not found"
  }
}

// 429 Too Many Requests
{
  "error": {
    "code": 429,
    "type": "rate_limited",
    "message": "Rate limit exceeded"
  }
}

// 503 Service Unavailable
{
  "error": {
    "code": 503,
    "type": "circuit_open",
    "message": "Upstream dependency unavailable"
  }
}
```

---

## Error Labels

### .as_str() Method

Returns stable string label for observability.

```rust
pub fn as_str(&self) -> &'static str {
    match self {
        RhelmaError::Config(_)          => "config",
        RhelmaError::Validation(_)      => "validation",
        RhelmaError::Auth(_)            => "auth",
        RhelmaError::Authz(_)           => "authz",
        RhelmaError::NotFound(_)        => "not_found",
        RhelmaError::Conflict(_)        => "conflict",
        RhelmaError::BadRequest(_)      => "bad_request",
        RhelmaError::Database(_)        => "database",
        RhelmaError::Cache(_)           => "cache",
        RhelmaError::RateLimited(_)     => "rate_limited",
        RhelmaError::Dependency(_)      => "dependency",
        RhelmaError::SecurityPolicy(_)  => "security_policy",
        RhelmaError::CircuitOpen(_)     => "circuit_open",
        RhelmaError::DistributedTx(_)   => "distributed_tx",
        RhelmaError::Internal           => "internal",
        RhelmaError::Other(_)           => "other",
    }
}
```

### Observability Usage

```rust
// Logging
error!(
    error_code = err.as_str(),
    "Operation failed"
);

// Metrics
error_counter
    .with_label_values(&[err.as_str()])
    .inc();

// Tracing
span.set_attribute("error.type", err.as_str());
```

---

## Best Practices

### ✅ Do's

1. **Return Result<T, RhelmaError>** for operations that can fail
   ```rust
   fn load_data(id: &str) -> RhelmaResult<Data> { }
   ```

2. **Use specific error types** (not generic Other)
   ```rust
   RhelmaError::NotFound("user not found".into())  // ✅ Good
   RhelmaError::Other(anyhow!("user not found"))   // ❌ Bad
   ```

3. **Add context** at boundaries
   ```rust
   operation()
       .rhelma_context("while loading data")?
   ```

4. **Never leak PII** in error messages
   ```rust
   RhelmaError::Auth("authentication failed".into())  // ✅ Good
   RhelmaError::Auth(format!("user {} not found", email))  // ❌ Bad
   ```

5. **Use descriptive messages** (user can act)
   ```rust
   RhelmaError::BadRequest("region must be lowercase alphanumeric".into())  // ✅
   RhelmaError::BadRequest("invalid input".into())  // ❌ Too vague
   ```

6. **Log sensitive details separately**
   ```rust
   warn!(
       request_id = %ctx.request_id(),
       user_email = "user@example.com",  // Hashed or redacted
       "Authentication failed"
   );
   Err(RhelmaError::Auth("authentication failed".into()))
   ```

### ❌ Don'ts

1. **Don't panic on recoverable errors**
   ```rust
   let data = load_data().unwrap();  // ❌ Will panic
   let data = load_data()?;          // ✅ Proper error handling
   ```

2. **Don't use String interpolation in error messages**
   ```rust
   format!("{}", error)  // ❌ Message changes per instance
   error.as_str()        // ✅ Stable label
   ```

3. **Don't leak secrets**
   ```rust
   RhelmaError::Config(db_connection_string.clone())  // ❌ Leaks secret
   RhelmaError::Config("database connection failed".into())  // ✅ Safe
   ```

4. **Don't use Other for new code**
   ```rust
   RhelmaError::Other(anyhow!("..."))  // ❌ For migration only
   RhelmaError::Validation("...".into())  // ✅ Use specific type
   ```

5. **Don't swallow errors silently**
   ```rust
   let _ = operation();  // ❌ Silent failure
   operation()?;         // ✅ Propagate error
   ```

---

## Examples

### Example 1: API Handler with Error Handling

```rust
#[cfg(feature = "axum")]
async fn create_invoice(
    Extension(ctx): Extension<RequestContext>,
    Json(req): Json<CreateInvoiceRequest>,
) -> Result<Json<Invoice>, RhelmaError> {
    // Validate tenant
    let tenant = ctx.tenant_id()
        .ok_or_else(|| RhelmaError::Auth("missing tenant".into()))?;
    
    // Validate input
    Email::parse(&req.customer_email)
        .rhelma_context("invalid customer email")?;
    
    if req.amount <= 0.0 {
        return Err(RhelmaError::Validation(
            "amount must be positive".into()
        ));
    }
    
    // Create invoice
    let invoice = Invoice::create(&req, tenant)
        .await
        .rhelma_context("while creating invoice")?;
    
    info!(
        request_id = %ctx.request_id(),
        invoice_id = %invoice.id,
        "Invoice created successfully"
    );
    
    Ok(Json(invoice))
}
```

### Example 2: Database Operation with Error Context

```rust
async fn find_invoice(
    pool: &PgPool,
    ctx: &RequestContext,
    invoice_id: &str,
) -> RhelmaResult<Invoice> {
    let tenant = ctx.tenant_id()
        .ok_or_else(|| RhelmaError::Auth("missing tenant".into()))?;
    
    sqlx::query_as::<_, Invoice>(
        "SELECT * FROM invoices WHERE id = $1 AND tenant_id = $2"
    )
    .bind(invoice_id)
    .bind(tenant.as_str())
    .fetch_optional(pool)
    .await
    .rhelma_context("while querying database")?
    .ok_or_else(|| {
        RhelmaError::NotFound(format!("Invoice {} not found", invoice_id))
    })
}
```

### Example 3: Multi-Step Operation with Context Chain

```rust
fn complete_order(
    ctx: &RequestContext,
    order_id: &str,
) -> RhelmaResult<Order> {
    // Step 1: Load order
    let order = load_order(order_id)
        .rhelma_context("while loading order")?;
    
    // Step 2: Validate state
    validate_order_state(&order)
        .rhelma_context("during order state validation")?;
    
    // Step 3: Charge payment
    charge_payment(&order)
        .rhelma_context("while charging payment")?;
    
    // Step 4: Update status
    update_order_status(order_id, "completed")
        .rhelma_context("while updating order status")?;
    
    // Step 5: Send notification
    send_completion_email(&order)
        .rhelma_context("while sending completion email")?;
    
    Ok(order)
}

// Example error output with context chain:
// RhelmaError::Database("connection timeout (while charging payment) (while completing order)")
```

### Example 4: Custom Error Type Conversion

```rust
// Convert external error to RhelmaError
impl From<stripe::Error> for RhelmaError {
    fn from(err: stripe::Error) -> Self {
        match err {
            stripe::Error::InvalidRequest(_) => 
                RhelmaError::Validation(err.to_string()),
            stripe::Error::RateLimit(_) => 
                RhelmaError::RateLimited(err.to_string()),
            stripe::Error::ApiError(_) => 
                RhelmaError::Dependency(err.to_string()),
            _ => RhelmaError::Internal,
        }
    }
}

// Usage
stripe::charge(amount)
    .map_err(|e| RhelmaError::from(e))
    .rhelma_context("while processing payment")?
```

---

## Troubleshooting

### "error[E0308]: mismatched types"

**Problem:**
```rust
fn operation() -> String {
    RhelmaError::NotFound("not found".into())?  // ❌ Returns RhelmaError, not String
}
```

**Solution:** Return `RhelmaResult<T>` instead
```rust
fn operation() -> RhelmaResult<String> {
    RhelmaError::NotFound("not found".into()).err()
}
```

### "error: this operation will panic at runtime"

**Problem:**
```rust
let data = load_data().unwrap();  // ❌ Will panic if Err
```

**Solution:** Use `?` operator
```rust
let data = load_data()?;  // ✅ Propagates error
```

### "cannot find function `rhelma_context` in this scope"

**Problem:**
```rust
operation()
    .rhelma_context("context")?  // ❌ ErrorExt not imported
```

**Solution:** Import ErrorExt trait
```rust
use rhelma_core::prelude::*;  // Includes ErrorExt

operation()
    .rhelma_context("context")?  // ✅ Works
```

### "error message leaks PII"

**Problem:**
```rust
RhelmaError::Validation(format!("email {} already exists", user_email))
```

**Solution:** Log details separately
```rust
warn!(
    user_email = "user@example.com",  // Log separately
    "Email already registered"
);
Err(RhelmaError::Conflict("email already registered".into()))
```

---

**Last Updated:** December 6, 2025







