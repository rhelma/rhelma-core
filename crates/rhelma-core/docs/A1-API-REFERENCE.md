# rhelma-core API Reference v5.1

**Document:** A1-API-REFERENCE.md  
**Version:** 5.1.0  
**Status:** Stable

---

## Quick API Index

| Module | Key Types | Methods |
|--------|-----------|---------|
| **config** | `AppConfig` | `from_env_only()`, `validate_all()` |
| **error** | `RhelmaError` | `as_str()` |
| **request_context** | `RequestContext` | `from_headers()`, `empty()`, builder methods |
| **types** | `TenantId`, `UserId`, `Email`, `RegionId` | `parse()`, `as_str()` |
| **tenancy** | `TenantProfile`, `ResidencyPolicy` | `validate_residency()` |
| **observability** | `UnifiedObservabilityConfig` | |
| **realtime_types** | `ConnectionMetadata`, `PresenceStatus` | `inactivity_seconds()`, `is_stale()` |
| **security** | `PasswordPolicy`, `RateLimitKeyBuilder` | `validate()`, `build()` |

---

## AppConfig

**Module:** `rhelma_core::config`

### Construction

```rust
pub fn from_env_only() -> Result<Self, RhelmaError>
```

Loads configuration from environment variables WITHOUT validation.

**Environment Variables:**
- `RHELMA_ENV` / `RHELMA_ENVIRONMENT` (default: "development")
- `RHELMA_REGION` (default: "local")
- `RHELMA_JSON_LOGS` (default: "false")
- `RHELMA_SERVICE_NAME` (optional)
- `RHELMA_SERVICE_VERSION` (optional)
- `RHELMA_DEFAULT_TENANT_TIER` (optional)

**Example:**
```rust
let cfg = AppConfig::from_env_only()?;
```

### Validation

```rust
pub fn validate_all(&self) -> Result<(), RhelmaError>
```

Validates environment, region, and service name.

**Validation Rules:**
- `environment` ∈ {"development", "staging", "production"}
- `region` matches `[a-z0-9-]{3,}`

**Example:**
```rust
cfg.validate_all()?;  // Abort if invalid
```

### Accessors

```rust
pub fn json_logs_enabled(&self) -> bool
```

Returns whether JSON logging is enabled.

---

## RhelmaError

**Module:** `rhelma_core::error`

### Enum Variants

```rust
pub enum RhelmaError {
    Config(String),          // 500
    Validation(String),      // 400
    BadRequest(String),      // 400
    Auth(String),            // 401
    Authz(String),           // 403
    NotFound(String),        // 404
    Conflict(String),        // 409
    RateLimited(String),     // 429
    Cache(String),           // 500
    Database(String),        // 500
    Dependency(String),      // 503
    CircuitOpen(String),     // 503
    SecurityPolicy(String),  // 403
    DistributedTx(String),   // 500
    Internal,                // 500
    Other(anyhow::Error),    // 500
}
```

### Methods

```rust
pub fn as_str(&self) -> &'static str
```

Returns stable string label for metrics/logging:
- `"config"`, `"validation"`, `"auth"`, `"authz"`
- `"not_found"`, `"conflict"`, `"bad_request"`
- `"rate_limited"`, `"circuit_open"`, `"security_policy"`
- etc.

**Example:**
```rust
let err = RhelmaError::RateLimited("too many requests".into());
assert_eq!(err.as_str(), "rate_limited");  // For prometheus label
```

### ErrorExt Trait

**Module:** `rhelma_core::error`

```rust
pub trait ErrorExt: Sized {
    fn rhelma_context<C: Display>(self, ctx: C) -> Self;
    fn context<C: Display>(self, ctx: C) -> Self;  // Alias
}
```

Attaches context to errors, preserving type.

**Example:**
```rust
validate_id(&id)
    .rhelma_context("while validating invoice ID")?;

fetch_data()
    .rhelma_context("while querying database")?;
```

---

## RequestContext

**Module:** `rhelma_core::request_context`

### Construction

#### From Headers

```rust
pub fn from_headers<'a, H>(headers: H) -> Result<Self, RhelmaError>
where
    H: IntoIterator<Item = (&'a str, &'a str)>
```

Parses RequestContext from HTTP headers.

**Recognized Headers:**
- `x-request-id` (validates UUID format)
- `x-correlation-id` (string)
- `x-tenant-id` (validates format)
- `x-region` (validates format)
- `x-user-id` (UUID)
- `x-user-email` (RFC 5322)
- `x-session-id`, `x-client-ip`, `x-user-agent`, `x-device-id`
- `x-locale`

**Example:**
```rust
let ctx = RequestContext::from_headers(vec![
    ("x-request-id", "550e8400-e29b-41d4-a716-446655440000"),
    ("x-tenant-id", "acme-corp"),
    ("x-region", "eu-west-1"),
])?;
```

#### Empty Context

```rust
pub fn empty() -> Self
```

Creates context with random `request_id`, all other fields None.

**Example:**
```rust
let ctx = RequestContext::empty();
```

### Accessors

```rust
pub fn request_id(&self) -> Uuid
pub fn correlation_id(&self) -> Option<&str>
pub fn tenant_id(&self) -> Option<&TenantId>
pub fn region(&self) -> Option<&RegionId>
pub fn user_id(&self) -> Option<&UserId>
pub fn user_email(&self) -> Option<&Email>
pub fn has_tenant(&self) -> bool
pub fn has_region(&self) -> bool
pub fn locale(&self) -> Option<&str>
```

**Example:**
```rust
if let Some(tenant) = ctx.tenant_id() {
    println!("Processing for tenant: {}", tenant.as_str());
}
```

### Builders

```rust
pub fn with_tenant(self, t: TenantId) -> Self
pub fn with_region(self, r: RegionId) -> Self
pub fn with_user(self, id: UserId, email: Option<Email>) -> Self
pub fn with_locale<S: Into<String>>(self, loc: S) -> Self
pub fn add_scope<S: Into<String>>(self, s: S) -> Self
pub fn add_role<S: Into<String>>(self, r: S) -> Self
```

Build context fluently (immutable).

**Example:**
```rust
let ctx = RequestContext::empty()
    .with_tenant(TenantId::parse("acme-corp")?)
    .with_region(RegionId::parse("eu-west-1")?)
    .with_user(UserId::new(), None)
    .add_scope("read:invoices")
    .add_role("admin");
```

---

## Type System

### UserId

**Module:** `rhelma_core::types`

```rust
pub struct UserId(pub Uuid);

impl UserId {
    pub fn new() -> Self
    pub fn as_uuid(&self) -> Uuid
    pub fn parse(s: &str) -> Option<Self>
}
```

UUID-based user identifier.

**Example:**
```rust
let user = UserId::new();
let parsed = UserId::parse("550e8400-e29b-41d4-a716-446655440000")?;
```

### TenantId

**Module:** `rhelma_core::types`

```rust
pub struct TenantId(pub String);

impl TenantId {
    pub fn parse(s: &str) -> Result<Self, RhelmaError>
    pub fn as_str(&self) -> &str
    pub fn new_unchecked<S: Into<String>>(s: S) -> Self
}
```

Validated tenant identifier (lowercase alphanumeric + `-`, min 3 chars).

**Validation Rules:**
- Must be lowercase
- Only `[a-z0-9-]` allowed
- Minimum 3 characters

**Example:**
```rust
let tenant = TenantId::parse("acme-corp")?;
// ❌ TenantId::parse("ACME-CORP")?;      // Uppercase
// ❌ TenantId::parse("acme corp")?;      // Space
// ✅ TenantId::parse("acme-corp")?;      // Valid
```

### RegionId

**Module:** `rhelma_core::types`

```rust
pub struct RegionId(pub String);

impl RegionId {
    pub fn parse(s: &str) -> Result<Self, RhelmaError>
    pub fn as_str(&self) -> &str
    pub fn new_unchecked<S: Into<String>>(s: S) -> Self
}
```

Validated region identifier (lowercase alphanumeric + `-`, min 3 chars).

**Validation Rules:**
- Must be lowercase
- Only `[a-z0-9-]` allowed
- Minimum 3 characters

**Example:**
```rust
let region = RegionId::parse("us-west-2")?;
let eu = RegionId::parse("eu-west-1")?;
```

### Email

**Module:** `rhelma_core::types`

```rust
pub struct Email(pub String);

impl Email {
    pub fn parse(s: &str) -> Result<Self, RhelmaError>
    pub fn redacted(&self) -> String
}
```

RFC 5322-validated email address.

**Validation:** Uses `validator` crate for RFC 5322 compliance.

**Example:**
```rust
let email = Email::parse("alice@example.com")?;
println!("Redacted: {}", email.redacted());  // "a***@example.com"

// ❌ Email::parse("invalid@")?;       // Invalid
// ❌ Email::parse("@domain.com")?;    // Invalid
// ✅ Email::parse("user@example.com")?;
```

### Pagination

**Module:** `rhelma_core::types`

```rust
pub struct PageRequest {
    pub offset: u64,
    pub limit: u64,
}

impl PageRequest {
    pub fn new(offset: u64, limit: u64) -> Self
    pub fn normalized(&self) -> Self
    pub fn next_offset(&self) -> Option<u64>
}

pub struct Paginated<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
}

impl<T> Paginated<T> {
    pub fn total_pages(&self) -> u64
    pub fn current_page(&self) -> u64
    pub fn has_next(&self) -> bool
    pub fn next_offset(&self) -> Option<u64>
}
```

**Example:**
```rust
let page = PageRequest::new(0, 25);
let results = db.fetch_items(page)?;

println!("Total pages: {}", results.total_pages());
println!("Current page: {}", results.current_page());
if results.has_next() {
    let next_offset = results.next_offset().unwrap();
}
```

---

## TenantProfile

**Module:** `rhelma_core::tenancy`

```rust
pub struct TenantProfile {
    pub tenant_id: TenantId,
    pub name: String,
    pub tier: TenancyTier,
    pub residency: ResidencyPolicy,
    pub primary_region: RegionId,
    pub backup_regions: Vec<RegionId>,
    pub ai_allowed: bool,
    pub logging_pii_allowed: bool,
    pub sla: Option<SlaTarget>,
    pub dr_tier: Option<DrTier>,
    pub metadata: serde_json::Value,
}

impl TenantProfile {
    pub fn validate_residency(&self, region: &RegionId) -> Result<(), RhelmaError>
    pub fn is_isolated(&self) -> bool
    pub fn is_region_sensitive(&self) -> bool
}
```

### Residency Policies

```rust
pub enum ResidencyPolicy {
    GlobalPreferred,      // Data can go anywhere
    RegionalPreferred,    // Primary/backup only
    RegionalRequired,     // Primary only (GDPR)
}
```

### Tenancy Tiers

```rust
pub enum TenancyTier {
    Tier1Shared,                    // Shared infra
    Tier2SharedDbIsolatedSchema,    // Shared DB, isolated schema
    Tier3DedicatedDb,               // Dedicated DB
}
```

**Example:**
```rust
let tenant = TenantProfile {
    tenant_id: TenantId::parse("acme-corp")?,
    residency: ResidencyPolicy::RegionalRequired,
    primary_region: RegionId::parse("eu-west-1")?,
    ..Default::default()
};

tenant.validate_residency(&RegionId::parse("eu-west-1")?)?;  // ✅
// tenant.validate_residency(&RegionId::parse("us-west-2")?)?;  // ❌
```

---

## UnifiedObservabilityConfig

**Module:** `rhelma_core::observability`

```rust
pub struct UnifiedObservabilityConfig {
    pub service_name: String,
    pub environment: String,
    pub region: String,
    pub json_logs: bool,
    pub otlp_enabled: bool,
    pub otlp_endpoint: Option<String>,
    pub log_level: Option<String>,
}

`

**Environment Variables:**
- `RHELMA_OBS__ENABLE_OTLP`
- `RHELMA_OBS__OTLP_ENDPOINT`
- `RHELMA_OBS__LOG_LEVEL`

**Example:**
```rust
let cfg = AppConfig::from_env_only()?;

println!("Service: {} in {}", obs.service_name, obs.environment);
println!("OTLP: {}", obs.otlp_enabled);
```

---

## Realtime Types

**Module:** `rhelma_core::realtime_types`

### RealtimeSessionId

```rust
pub struct RealtimeSessionId(pub Uuid);

impl RealtimeSessionId {
    pub fn new() -> Self
}
```

UUID-based session identifier for WebSocket connections.

### PresenceStatus

```rust
pub enum PresenceStatus {
    Online,
    Away,
    Offline,
}
```

User presence status, JSON-serializable.

### ConnectionMetadata

```rust
pub struct ConnectionMetadata {
    pub session_id: RealtimeSessionId,
    pub user_id: UserId,
    pub tenant_id: Option<TenantId>,
    pub region: Option<RegionId>,
    pub connected_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub user_agent: Option<String>,
    pub ip: Option<String>,
}

impl ConnectionMetadata {
    pub fn inactivity_seconds(&self, now: DateTime<Utc>) -> i64
    pub fn is_stale(&self, now: DateTime<Utc>, timeout_secs: i64) -> bool
}
```

**Example:**
```rust
let meta = ConnectionMetadata {
    session_id: RealtimeSessionId::new(),
    user_id: UserId::new(),
    tenant_id: Some(TenantId::parse("acme")?),
    connected_at: Utc::now(),
    last_seen_at: Utc::now(),
    ..Default::default()
};

if meta.is_stale(Utc::now(), 300) {  // 5 min timeout
    disconnect(&meta.session_id);
}
```

---

## Security

**Module:** `rhelma_core::security`

### PasswordPolicy

```rust
pub struct PasswordPolicy {
    pub min_length: usize,
    pub max_length: usize,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_digit: bool,
    pub require_symbol: bool,
}

impl PasswordPolicy {
    pub fn validate(&self, password: &str) -> Result<(), RhelmaError>
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        // min_length: 8
        // require: uppercase, lowercase, digit, symbol
    }
}
```

**Example:**
```rust
let policy = PasswordPolicy::default();
policy.validate("SecurePass123!")?;  // ✅
// policy.validate("weak")?;         // ❌ Too short
```

### RateLimitKeyBuilder

```rust
pub struct RateLimitKeyBuilder {
    // private
}

impl RateLimitKeyBuilder {
    pub fn new(namespace: impl Into<String>) -> Self
    pub fn with_tenant(self, t: TenantId) -> Self
    pub fn with_user(self, u: UserId) -> Self
    pub fn with_region(self, r: RegionId) -> Self
    pub fn build(&self, operation: &str) -> String
}
```

Builds namespaced rate-limit keys.

**Format:** `rl:namespace:tenant=t:user=u:region=r:operation`

**Example:**
```rust
let key = RateLimitKeyBuilder::new("api")
    .with_tenant(TenantId::parse("acme")?)
    .with_user(user_id)
    .build("login");
// Result: "rl:api:tenant=acme:user=550e8400...:login"
```

---

## Type Aliases

**Module:** `rhelma_core::result`

```rust
pub type RhelmaResult<T> = Result<T, RhelmaError>;
```

Use instead of `Result<T, RhelmaError>`.

**Example:**
```rust
fn my_operation() -> RhelmaResult<String> {
    Ok("success".into())
}
```

---

## Constants

**Module:** `rhelma_core::constants`

```rust
pub const HEADER_CORRELATION_ID: &str = "x-correlation-id";
pub const HEADER_REQUEST_ID: &str = "x-request-id";
pub const HEADER_TENANT_ID: &str = "x-tenant-id";
pub const HEADER_REGION: &str = "x-region";
```

---

## Prelude

**Module:** `rhelma_core::prelude`

```rust
use rhelma_core::prelude::*;

// Includes:
// - AppConfig, RhelmaError, RhelmaResult
// - RequestContext, TenantId, RegionId, Email, UserId
// - TenantProfile, ResidencyPolicy
// - UnifiedObservabilityConfig
// - PasswordPolicy, RateLimitKeyBuilder
// - ErrorExt trait
// - Common re-exports (Uuid, chrono, serde, etc.)
```

---

**Last Updated:** December 6, 2025







