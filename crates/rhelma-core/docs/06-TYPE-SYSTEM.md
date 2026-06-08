# Type System v5.1 — Strong Identifiers & Validation

**Document:** 06-TYPE-SYSTEM.md  
**Version:** 5.1.0  
**Status:** Final

---

## Table of Contents

1. [Philosophy](#philosophy)
2. [Strong Identifiers](#strong-identifiers)
3. [TenantId](#tenantid)
4. [RegionId](#regionid)
5. [UserId](#userid)
6. [Email](#email)
7. [Validation Strategy](#validation-strategy)
8. [Preventing Category Errors](#preventing-category-errors)
9. [Best Practices](#best-practices)
10. [Examples](#examples)

---

## Philosophy

### Problem: Stringly Typed Code

```rust
// ❌ Easy to mix up identifiers
fn process_order(user_id: String, tenant_id: String, region_id: String) {
    db.query("SELECT * FROM orders WHERE user_id = $1", tenant_id)?;
    // Bug! Passed tenant_id to user_id parameter
}
```

### Solution: Strong Types

```rust
// ✅ Compiler prevents mixing
fn process_order(user: UserId, tenant: TenantId, region: RegionId) {
    db.query("SELECT * FROM orders WHERE user_id = $1", tenant)?;
    // ❌ Compile error: type mismatch (tenant is TenantId, not UserId)
}
```

### Benefits

- **Compile-Time Safety** — Wrong types caught before runtime
- **Self-Documenting** — Types clearly show intent
- **No Runtime Overhead** — Zero-cost abstraction
- **Refactoring Safe** — Change type, fix all call sites
- **Prevents Category Errors** — Entire class of bugs eliminated

---

## Strong Identifiers

All identifiers in rhelma-core are **strongly typed**:

| Type | Usage | Format | Storage |
|------|-------|--------|---------|
| `TenantId` | Tenant identification | Lowercase alphanumeric + `-` | String |
| `RegionId` | Region identification | Lowercase alphanumeric + `-` | String |
| `UserId` | User identification | UUID | Uuid |
| `Email` | Email address | RFC 5322 | String |

### Creation Methods

**Safe construction (validated):**
```rust
TenantId::parse("acme-corp")?;
RegionId::parse("us-west-2")?;
UserId::new();
Email::parse("user@example.com")?;
```

**Unsafe construction (unchecked):**
```rust
TenantId::new_unchecked("acme-corp");
RegionId::new_unchecked("us-west-2");
UserId(uuid);
```

**Rule:** Always use `parse()` for user input. Use `new_unchecked()` only internally when you're certain of the value.

---

## TenantId

### Definition

```rust
pub struct TenantId(pub String);
```

Strongly-typed tenant identifier.

### Validation Rules

```
Format:     [a-z0-9-]{1,}
Min length: 1 character
Max length: Unlimited (practical: 256)
Case:       Lowercase only
Chars:      Lowercase letters, digits, hyphens
Hyphens:    Allowed anywhere (even start/end)
```

### Creation

**From user input:**
```rust
let tenant = TenantId::parse("acme-corp")?;
// ✅ Valid: lowercase, alphanumeric, hyphen
```

**From internal source (trusted):**
```rust
let tenant = TenantId::new_unchecked("acme-corp");
// ✅ Safe: we verified it ourselves
```

**Direct construction:**
```rust
TenantId("acme-corp".to_string())  // Not recommended
```

### Validation Examples

```rust
// ✅ Valid
TenantId::parse("acme-corp")?;
TenantId::parse("a")?;               // Min 1 char
TenantId::parse("123")?;             // All digits OK
TenantId::parse("acme-corp-1")?;     // Hyphens OK
TenantId::parse("-acme-")?;          // Hyphens at edges OK

// ❌ Invalid
TenantId::parse("ACME-CORP")?;       // Uppercase rejected
TenantId::parse("acme corp")?;       // Space rejected
TenantId::parse("acme_corp")?;       // Underscore rejected (hyphen only)
TenantId::parse("acme.corp")?;       // Dot rejected
TenantId::parse("")?;                // Empty rejected
```

### Methods

```rust
pub fn parse(s: &str) -> Result<Self, RhelmaError>
pub fn as_str(&self) -> &str

pub fn new_unchecked<S: Into<String>>(s: S) -> Self
```

### Usage

```rust
let tenant = TenantId::parse("acme-corp")?;

println!("{}", tenant.as_str());  // "acme-corp"

// In database queries
db.query("SELECT * FROM data WHERE tenant_id = $1", tenant.as_str())?;

// In logs
info!(tenant_id = tenant.as_str(), "Processing");

// For API responses
json!({ "tenant_id": tenant.as_str() })
```

---

## RegionId

### Definition

```rust
pub struct RegionId(pub String);
```

Strongly-typed region identifier.

### Validation Rules

```
Format:     [a-z0-9-]{3,}
Min length: 3 characters
Max length: Unlimited (practical: 256)
Case:       Lowercase only
Chars:      Lowercase letters, digits, hyphens
```

### Creation

```rust
let region = RegionId::parse("us-west-2")?;
let region = RegionId::parse("eu-west-1")?;
let region = RegionId::parse("local")?;      // ✅ Exactly 3 chars
```

### Validation Examples

```rust
// ✅ Valid
RegionId::parse("us-west-2")?;
RegionId::parse("eu-central-1")?;
RegionId::parse("local")?;                   // Min 3 chars
RegionId::parse("ap-southeast-1")?;

// ❌ Invalid
RegionId::parse("us")?;                      // Too short (< 3)
RegionId::parse("US-WEST-2")?;              // Uppercase rejected
RegionId::parse("us_west_2")?;              // Underscore rejected
RegionId::parse("us west 2")?;              // Space rejected
```

### Methods

```rust
pub fn parse(s: &str) -> Result<Self, RhelmaError>
pub fn as_str(&self) -> &str

pub fn new_unchecked<S: Into<String>>(s: S) -> Self
```

### Usage

```rust
let region = RegionId::parse("eu-west-1")?;

println!("{}", region.as_str());  // "eu-west-1"

// In RequestContext
let ctx = RequestContext::empty()
    .with_region(region)?;

// In database queries
db.query("SELECT * FROM data WHERE region = $1", region.as_str())?;

// For validation
profile.validate_residency(&region)?;
```

---

## UserId

### Definition

```rust
pub struct UserId(pub Uuid);
```

Strongly-typed user identifier (UUID-based).

### Characteristics

- **Format:** UUID v4/v7
- **Storage:** 128-bit UUID
- **Uniqueness:** Cryptographically unique
- **Immutable:** Cannot be changed

### Creation

**Generate new:**
```rust
let user = UserId::new();
// Generates random UUID v4
```

**From existing UUID:**
```rust
let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000")?;
let user = UserId(uuid);
```

**Parse from string:**
```rust
let user = UserId::parse("550e8400-e29b-41d4-a716-446655440000")?;
```

### Methods

```rust
pub fn new() -> Self
pub fn as_uuid(&self) -> Uuid
pub fn parse(s: &str) -> Option<Self>
```

### Usage

```rust
let user = UserId::new();

println!("{}", user.as_uuid());  // UUID value

// In RequestContext
let ctx = RequestContext::empty()
    .with_user(user, None);

// In database
db.query("SELECT * FROM users WHERE user_id = $1", user.as_uuid())?;

// For logging
info!(user_id = %user.as_uuid(), "User created");
```

---

## Email

### Definition

```rust
pub struct Email(pub String);
```

Strongly-typed email address with RFC 5322 validation.

### Validation

Uses `validator` crate for RFC 5322 compliance.

**Valid formats:**
```rust
Email::parse("alice@example.com")?;              // ✅
Email::parse("bob.smith@example.co.uk")?;       // ✅
Email::parse("user+tag@sub.example.com")?;      // ✅
Email::parse("123@example.com")?;               // ✅
```

**Invalid formats:**
```rust
Email::parse("not-an-email")?;                   // ❌ No @
Email::parse("user@")?;                          // ❌ Missing domain
Email::parse("@example.com")?;                   // ❌ Missing local part
Email::parse("user @example.com")?;              // ❌ Space
Email::parse("user@example")?;                   // ❌ No TLD
Email::parse("user@@example.com")?;              // ❌ Double @
```

### Methods

```rust
pub fn parse(s: &str) -> Result<Self, RhelmaError>
pub fn redacted(&self) -> String
```

### Redaction for Safe Logging

```rust
let email = Email::parse("alice@example.com")?;

// Safe to log without exposing full email
info!(email = %email.redacted(), "User signup");
// Logs: "a***@example.com"
```

**Redaction Pattern:**
```
alice@example.com → a***@example.com
bob.smith@example.co.uk → b***@example.co.uk
```

### Usage

```rust
let email = Email::parse(user_input)?;

// Store in database
db.insert_user(&user_id, &email).await?;

// In logs (redacted)
info!(email = %email.redacted(), "User created");

// For API responses (full email if allowed)
json!({ "email": email.as_str() })
```

---

## Validation Strategy

### Two-Level Validation

**Level 1: Format Validation (at parse-time)**

```rust
let tenant = TenantId::parse(user_input)?;
// Validates format: [a-z0-9-]{1,}
// Returns Err if invalid
```

**Level 2: Semantic Validation (at use-time)**

```rust
let profile = load_tenant_profile(&tenant).await?;
profile.validate_residency(&region)?;
// Validates business rules: tenant exists, region allowed, etc.
```

### Parse-Time (Format)

Validates **format only**:
- ✅ String format (regex)
- ✅ Length constraints
- ✅ Character set
- ✅ RFC compliance (email)

**Does NOT validate:**
- ❌ Existence (does tenant exist in DB?)
- ❌ Permissions (can user access this?)
- ❌ Business rules (is this allowed by SLA?)

### Use-Time (Semantic)

Validates **business logic**:

```rust
// Format validated ✅
let region = RegionId::parse(user_region)?;

// Semantic validation
let profile = load_tenant_profile(&tenant).await?;
profile.validate_residency(&region)?;  // Checks: is region allowed for this tenant?
```

---

## Preventing Category Errors

### Problem

```rust
// ❌ Easy to pass wrong type
fn charge_user(user_id: String, tenant_id: String) {
    // Oops! Swapped them
    charge(&tenant_id, &user_id)?;
}
```

### Solution: Strong Types

```rust
// ✅ Impossible to mix up
fn charge_user(user: UserId, tenant: TenantId) {
    // ❌ Compile error if swapped
    charge(&tenant, &user)?;  // Type mismatch!
}

// ✅ Correct
fn charge_user(user: UserId, tenant: TenantId) {
    charge(&user, &tenant)?;  // ✅ Type match
}
```

### Type Mismatches Caught

**Function signature changes:**
```rust
// Old signature
fn process(user: String, tenant: String) { }

// New signature (strong types)
fn process(user: UserId, tenant: TenantId) { }

// All call sites must be updated
process(user_id, tenant_id)?;  // ❌ Won't compile until fixed
```

**Vector/Map operations:**
```rust
// ❌ Type mismatch caught
let users: Vec<UserId> = vec![];
let user: TenantId = get_user()?;
users.push(user)?;  // ❌ Compile error

// ✅ Correct
let users: Vec<UserId> = vec![];
let user: UserId = get_user()?;
users.push(user)?;  // ✅ Works
```

---

## Best Practices

### ✅ Do's

1. **Always use parse() for user input**
   ```rust
   let tenant = TenantId::parse(user_input)
       .rhelma_context("invalid tenant ID")?;
   ```

2. **Use new_unchecked() only for internal sources**
   ```rust
   // Only when you're certain
   let tenant = TenantId::new_unchecked(database_value);
   ```

3. **Pass types, not strings**
   ```rust
   // ✅ Good
   fn process(tenant: TenantId) { }
   
   // ❌ Bad
   fn process(tenant: String) { }
   ```

4. **Validate at boundaries**
   ```rust
   // API endpoint: validate immediately
   let tenant = TenantId::parse(&req.tenant_id)
       .rhelma_context("invalid tenant ID")?;
   ```

5. **Use as_str() for storage/logs**
   ```rust
   db.query("...", tenant.as_str())?;
   info!(tenant_id = tenant.as_str(), "...");
   ```

### ❌ Don'ts

1. **Don't use unwrap() on parse()**
   ```rust
   // ❌ Will panic on invalid input
   let tenant = TenantId::parse(user_input).unwrap();
   
   // ✅ Proper error handling
   let tenant = TenantId::parse(user_input)?;
   ```

2. **Don't skip validation**
   ```rust
   // ❌ Trusts input without validation
   let tenant = TenantId(user_input.to_string());
   
   // ✅ Validates format
   let tenant = TenantId::parse(user_input)?;
   ```

3. **Don't mix types**
   ```rust
   // ❌ Wrong types
   fn process(user_id: String, tenant_id: String) { }
   
   // ✅ Strong types
   fn process(user_id: UserId, tenant_id: TenantId) { }
   ```

4. **Don't store types in wrong containers**
   ```rust
   // ❌ Type confusion
   let ids: Vec<String> = vec![];
   let user = UserId::new();
   ids.push(user.as_uuid().to_string())?;  // Loses type info
   
   // ✅ Maintain types
   let ids: Vec<UserId> = vec![];
   ids.push(user)?;
   ```

---

## Examples

### Example 1: Type-Safe Handler

```rust
async fn create_invoice(
    Extension(ctx): Extension<RequestContext>,
    Json(req): Json<CreateInvoiceRequest>,
) -> Result<Json<Invoice>, RhelmaError> {
    // Type-safe extraction
    let tenant = ctx.tenant_id()
        .ok_or_else(|| RhelmaError::Auth("missing tenant".into()))?;
    
    let user = ctx.user_id()
        .ok_or_else(|| RhelmaError::Auth("missing user".into()))?;
    
    // Type-safe validation
    let customer_email = Email::parse(&req.customer_email)
        .rhelma_context("invalid customer email")?;
    
    // Type-safe query
    let invoice = Invoice::create(
        &tenant,      // TenantId (not String)
        &user,        // UserId (not String)
        &customer_email,  // Email (RFC 5322 validated)
    )
    .await?;
    
    Ok(Json(invoice))
}
```

### Example 2: Type-Safe Database Layer

```rust
async fn find_user_invoices(
    pool: &PgPool,
    tenant: &TenantId,
    user: &UserId,
) -> RhelmaResult<Vec<Invoice>> {
    sqlx::query_as::<_, Invoice>(
        "SELECT * FROM invoices WHERE tenant_id = $1 AND user_id = $2"
    )
    .bind(tenant.as_str())      // TenantId → &str
    .bind(user.as_uuid())        // UserId → Uuid
    .fetch_all(pool)
    .await
    .rhelma_context("while querying invoices")
}
```

### Example 3: Preventing Category Errors

```rust
// Type mismatch caught at compile time
fn transfer_payment(
    from_tenant: TenantId,
    to_tenant: TenantId,
    amount: f64,
) -> RhelmaResult<()> {
    // ✅ Correct: same type
    if from_tenant == to_tenant {
        return Err(RhelmaError::Conflict(
            "cannot transfer within same tenant".into()
        ));
    }
    
    Ok(())
}

// Wrong types → compile error
let from = UserId::new();
let to = TenantId::parse("other")?;
transfer_payment(from, to, 100.0)?;  // ❌ Type mismatch!
```

---

**Last Updated:** December 6, 2025







