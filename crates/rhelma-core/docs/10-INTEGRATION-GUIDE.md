# rhelma-core Integration Guide v5.1

**Document:** 10-INTEGRATION-GUIDE.md  
**Version:** 5.1.0  
**Status:** Stable

---

## Table of Contents

1. [Installation](#installation)
2. [Service Initialization](#service-initialization)
3. [HTTP Middleware](#http-middleware)
4. [Database Integration](#database-integration)
5. [Error Handling](#error-handling)
6. [Event Integration](#event-integration)
7. [Observability](#observability)
8. [Real-Time Features](#real-time-features)
9. [Multi-Tenant Patterns](#multi-tenant-patterns)
10. [Security Best Practices](#security-best-practices)
11. [Testing](#testing)
12. [Troubleshooting](#troubleshooting)

---

## Installation

### 1. Add Dependency

**Cargo.toml:**
```toml
[dependencies]
rhelma-core = "5.1"

# Optional: web framework integration
rhelma-core = { version = "5.1", features = ["full"] }

# Or specific features:
rhelma-core = { version = "5.1", features = ["axum", "sqlx"] }
```

**Features:**
- `axum` — HTTP error mapping to Axum responses
- `sqlx` — Database error conversion
- `full` — All features enabled

### 2. Update .env

```bash
# Required
RHELMA_ENV=development        # or staging, production
RHELMA_REGION=local           # or us-west-2, eu-west-1, etc.
RHELMA_SERVICE_NAME=my-service

# Optional
RHELMA_JSON_LOGS=true
RHELMA_OBS__ENABLE_OTLP=true
RHELMA_OBS__OTLP_ENDPOINT=http://localhost:4317
RHELMA_OBS__LOG_LEVEL=debug
```

---

## Service Initialization

### Basic Setup

```rust
use rhelma_core::prelude::*;

#[tokio::main]
async fn main() -> RhelmaResult<()> {
    // 1. Load and validate configuration
    let cfg = AppConfig::from_env_only()?;
    cfg.validate_all()
        .rhelma_context("during config validation")?;
    
    // 2. Setup observability
    let obs = UnifiedObservabilityConfig::from_app_config(&cfg);
    init_tracing(&obs)?;
    init_metrics(&obs)?;
    
    info!(
        service = obs.service_name,
        region = obs.region,
        environment = obs.environment,
        "Service starting"
    );
    
    // 3. Initialize database, cache, etc.
    let db = db::connect(&cfg)
        .await
        .rhelma_context("during database connection")?;
    
    // 4. Start service
    start_server(cfg, db).await?;
    
    Ok(())
}

fn init_tracing(obs: &UnifiedObservabilityConfig) -> RhelmaResult<()> {
    // Use tracing_subscriber + OTLP exporter
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(obs.log_level.as_deref().unwrap_or("info"))
        .init();
    
    Ok(())
}

fn init_metrics(_obs: &UnifiedObservabilityConfig) -> RhelmaResult<()> {
    // Setup Prometheus metrics
    Ok(())
}
```

---

## HTTP Middleware

### Axum Extract RequestContext

```rust
use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use rhelma_core::prelude::*;

/// Middleware to extract RequestContext from headers
pub async fn extract_context(
    mut req: Request,
    next: Next,
) -> Response {
    match parse_request_context(&req) {
        Ok(ctx) => {
            // Store in request extensions for handlers
            req.extensions_mut().insert(ctx.clone());
            
            // Log incoming request
            debug!(
                request_id = %ctx.request_id(),
                tenant_id = ?ctx.tenant_id().map(|t| t.as_str()),
                "Request received"
            );
            
            next.run(req).await
        }
        Err(err) => {
            error!(error = ?err, "Failed to parse request context");
            (axum::http::StatusCode::BAD_REQUEST, err.to_string()).into_response()
        }
    }
}

fn parse_request_context(req: &Request) -> RhelmaResult<RequestContext> {
    let headers: Vec<(&str, &str)> = req
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            let name_str = name.as_str();
            let value_str = value.to_str().ok()?;
            Some((name_str, value_str))
        })
        .collect();
    
    RequestContext::from_headers(headers)
        .rhelma_context("while parsing request headers")
}

/// Extract RequestContext from Axum extensions
pub async fn with_context<H, T>(
    extract::Extension(ctx): extract::Extension<RequestContext>,
    handler: H,
) -> impl Response
where
    H: Fn(RequestContext) -> impl std::future::Future<Output = RhelmaResult<T>>,
{
    handler(ctx).await
}
```

### Full Router Setup

```rust
use axum::{
    routing::{get, post},
    Router,
    Extension,
    middleware,
};

fn create_router(ctx: Arc<AppContext>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/invoices", post(create_invoice))
        .route("/invoices/:id", get(get_invoice))
        
        // Extract RequestContext middleware
        .layer(middleware::from_fn(extract_context))
        
        // Add context as extension
        .layer(Extension(ctx))
}

async fn health_check() -> RhelmaResult<Json<HealthStatus>> {
    Ok(Json(HealthStatus {
        status: "healthy".into(),
        ..Default::default()
    }))
}

async fn create_invoice(
    Extension(ctx): Extension<RequestContext>,
    Json(req): Json<CreateInvoiceRequest>,
) -> Result<Json<Invoice>, RhelmaError> {
    // ctx is guaranteed to exist
    
    let tenant = ctx.tenant_id()
        .ok_or_else(|| RhelmaError::Auth("missing tenant".into()))?;
    
    // Validate request
    validate_invoice_request(&req)
        .rhelma_context("while validating invoice request")?;
    
    // Create invoice
    let invoice = Invoice::create(&req, tenant)
        .await
        .rhelma_context("while creating invoice")?;
    
    info!(
        request_id = %ctx.request_id(),
        invoice_id = %invoice.id,
        tenant_id = tenant.as_str(),
        "Invoice created"
    );
    
    Ok(Json(invoice))
}
```

---

## Database Integration

### With sqlx

```rust
use sqlx::postgres::PgPool;
use rhelma_core::prelude::*;

/// Create database pool with tenant isolation
pub async fn create_pool(cfg: &AppConfig) -> RhelmaResult<PgPool> {
    let db_url = std::env::var("DATABASE_URL")
        .map_err(|_| RhelmaError::Config("DATABASE_URL not set".into()))?;
    
    PgPool::connect(&db_url)
        .await
        .map_err(|e| RhelmaError::Database(e.to_string()))
        .rhelma_context("while connecting to database")
}

/// Example query with tenancy enforcement
pub async fn find_invoice(
    pool: &PgPool,
    ctx: &RequestContext,
    invoice_id: &str,
) -> RhelmaResult<Invoice> {
    let tenant = ctx.tenant_id()
        .ok_or_else(|| RhelmaError::Auth("missing tenant".into()))?;
    
    // Database enforces tenant_id in WHERE clause
    let invoice = sqlx::query_as::<_, Invoice>(
        "SELECT * FROM invoices WHERE id = $1 AND tenant_id = $2"
    )
    .bind(invoice_id)
    .bind(tenant.as_str())
    .fetch_optional(pool)
    .await
    .rhelma_context("while querying invoices")?
    .ok_or_else(|| RhelmaError::NotFound(format!("Invoice {} not found", invoice_id)))?;
    
    Ok(invoice)
}

/// List invoices with pagination
pub async fn list_invoices(
    pool: &PgPool,
    ctx: &RequestContext,
    page_req: PageRequest,
) -> RhelmaResult<Paginated<Invoice>> {
    let tenant = ctx.tenant_id()
        .ok_or_else(|| RhelmaError::Auth("missing tenant".into()))?;
    
    let page = page_req.normalized();
    
    // Get total count
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM invoices WHERE tenant_id = $1"
    )
    .bind(tenant.as_str())
    .fetch_one(pool)
    .await
    .rhelma_context("while counting invoices")?;
    
    // Fetch page
    let items = sqlx::query_as::<_, Invoice>(
        "SELECT * FROM invoices WHERE tenant_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
    )
    .bind(tenant.as_str())
    .bind(page.limit as i64)
    .bind(page.offset as i64)
    .fetch_all(pool)
    .await
    .rhelma_context("while fetching invoices")?;
    
    Ok(Paginated {
        items,
        total: total as u64,
        offset: page.offset,
        limit: page.limit,
    })
}
```

---

## Error Handling

### Error Context Pattern

```rust
fn complex_operation(ctx: &RequestContext) -> RhelmaResult<String> {
    // Validate input
    validate_input()
        .rhelma_context("during input validation")?;
    
    // Load dependencies
    let tenant = load_tenant(ctx)
        .rhelma_context("while loading tenant config")?;
    
    // Enforce residency
    tenant.validate_residency(ctx.region().unwrap_or(&RegionId::parse("local")?)?)
        .rhelma_context("during residency validation")?;
    
    // Execute business logic
    execute_logic(&tenant)
        .rhelma_context("during business logic execution")?;
    
    Ok("success".into())
}
```

### Custom Error Messages

```rust
// ✅ Good: No PII leakage
RhelmaError::Validation("email format invalid".into())

// ❌ Bad: Leaks information
RhelmaError::Validation(format!("email {} already exists", user_email))

// ✅ Good: Log details separately
warn!(
    request_id = %ctx.request_id(),
    email = "user@example.com",  // Redacted or hashed
    "User registration failed: email already exists"
);
Err(RhelmaError::Conflict("email already registered".into()))
```

### Handler with Error Logging

```rust
#[cfg(feature = "axum")]
async fn handler(
    Extension(ctx): Extension<RequestContext>,
) -> Result<Json<Response>, RhelmaError> {
    match internal_operation(&ctx).await {
        Ok(data) => {
            info!(request_id = %ctx.request_id(), "Operation succeeded");
            Ok(Json(data))
        }
        Err(err) => {
            error!(
                request_id = %ctx.request_id(),
                error_code = err.as_str(),
                "Operation failed"
            );
            Err(err)
        }
    }
}
```

---

## Event Integration

### Publishing Events with RequestContext

```rust
use rhelma_core::prelude::*;

async fn publish_event(
    ctx: &RequestContext,
    event_type: &str,
    payload: serde_json::Value,
) -> RhelmaResult<()> {
    let event = serde_json::json!({
        "event_id": uuid::Uuid::new_v7().to_string(),
        "event_type": event_type,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "request": {
            "request_id": ctx.request_id().to_string(),
            "correlation_id": ctx.correlation_id(),
            "tenant_id": ctx.tenant_id().map(|t| t.as_str()),
        },
        "payload": payload,
    });
    
    // Publish to event bus
    event_bus.publish(event_type, event)
        .await
        .rhelma_context("while publishing event")?;
    
    Ok(())
}
```

### Consuming Events with Tenant Isolation

```rust
async fn process_event(
    event: EventEnvelope,
    db: &PgPool,
) -> RhelmaResult<()> {
    // Extract tenant from event
    let tenant_id = event.tenant_id
        .ok_or_else(|| RhelmaError::Validation("missing tenant_id".into()))?;
    
    // All database operations scoped to tenant
    match event.event_type {
        "invoice.created" => {
            let invoice = serde_json::from_value(event.payload)?;
            insert_invoice(db, &tenant_id, invoice)
                .await
                .rhelma_context("while inserting invoice")?;
        }
        "tenant.deleted" => {
            delete_tenant_data(db, &tenant_id)
                .await
                .rhelma_context("while deleting tenant data")?;
        }
        _ => {}
    }
    
    Ok(())
}
```

---

## Observability

### Structured Logging

```rust
use tracing::{info, warn, error, debug};

fn process_request(ctx: &RequestContext, data: &Data) -> RhelmaResult<()> {
    debug!(
        request_id = %ctx.request_id(),
        tenant_id = ?ctx.tenant_id().map(|t| t.as_str()),
        "Processing request"
    );
    
    match validate(data) {
        Ok(_) => {
            info!(
                request_id = %ctx.request_id(),
                "Validation passed"
            );
        }
        Err(err) => {
            warn!(
                request_id = %ctx.request_id(),
                error = %err,
                "Validation failed"
            );
        }
    }
    
    Ok(())
}
```

### Metrics

```rust
use prometheus::{histogram_vec, counter_vec};

lazy_static::lazy_static! {
    static ref OPERATION_DURATION: HistogramVec = histogram_vec!(
        "operation_duration_seconds",
        "Operation duration in seconds",
        &["operation", "status"]
    ).unwrap();
}

async fn timed_operation(ctx: &RequestContext) -> RhelmaResult<()> {
    let timer = OPERATION_DURATION
        .with_label_values(&["create_invoice", "started"])
        .start_timer();
    
    match do_work(ctx).await {
        Ok(()) => {
            timer.observe_duration();
            OPERATION_DURATION
                .with_label_values(&["create_invoice", "success"])
                .inc();
            Ok(())
        }
        Err(err) => {
            timer.stop_and_discard();
            OPERATION_DURATION
                .with_label_values(&["create_invoice", "error"])
                .inc();
            Err(err)
        }
    }
}
```

---

## Real-Time Features

### WebSocket with Presence Tracking

```rust
use rhelma_core::realtime_types::*;

async fn handle_websocket(
    ws: WebSocketUpgrade,
    Extension(ctx): Extension<RequestContext>,
) -> impl Response {
    ws.on_upgrade(|socket| handle_socket(socket, ctx))
}

async fn handle_socket(
    socket: WebSocket,
    ctx: RequestContext,
) {
    let session = RealtimeSessionId::new();
    
    let meta = ConnectionMetadata {
        session_id: session,
        user_id: ctx.user_id().cloned().unwrap_or_else(UserId::new),
        tenant_id: ctx.tenant_id().cloned(),
        region: ctx.region().cloned(),
        connected_at: Utc::now(),
        last_seen_at: Utc::now(),
        user_agent: None,
        ip: None,
    };
    
    // Track session
    SESSION_TRACKER.register(&meta).await;
    
    // Handle messages
    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            let mut meta = meta.clone();
            meta.last_seen_at = Utc::now();
            
            if meta.is_stale(Utc::now(), 300) {
                break;  // Disconnect if stale
            }
            
            handle_message(msg, &meta).await;
        }
    }
    
    // Cleanup
    SESSION_TRACKER.unregister(&meta).await;
}
```

---

## Multi-Tenant Patterns

### Tenant-Scoped Database Queries

```rust
pub async fn get_user_invoices(
    pool: &PgPool,
    ctx: &RequestContext,
    user_id: UserId,
) -> RhelmaResult<Vec<Invoice>> {
    let tenant = ctx.tenant_id()
        .ok_or_else(|| RhelmaError::Auth("missing tenant".into()))?;
    
    // Database enforces tenant_id
    sqlx::query_as::<_, Invoice>(
        "SELECT * FROM invoices \
         WHERE tenant_id = $1 AND user_id = $2 \
         ORDER BY created_at DESC"
    )
    .bind(tenant.as_str())
    .bind(user_id.as_uuid())
    .fetch_all(pool)
    .await
    .rhelma_context("while fetching user invoices")
}
```

### Residency-Aware Operations

```rust
async fn store_sensitive_data(
    ctx: &RequestContext,
    profile: &TenantProfile,
    data: SensitiveData,
) -> RhelmaResult<()> {
    let region = ctx.region()
        .ok_or_else(|| RhelmaError::BadRequest("missing region".into()))?;
    
    // Enforce residency policy
    profile.validate_residency(region)
        .rhelma_context("during residency validation")?;
    
    // Store in allowed region only
    store_in_region(region, data)
        .await
        .rhelma_context("while storing data")
}
```

---

## Security Best Practices

### Validate Input

```rust
fn create_invoice(req: CreateInvoiceRequest) -> RhelmaResult<()> {
    // Validate email
    Email::parse(&req.customer_email)
        .rhelma_context("invalid customer email")?;
    
    // Validate tenant ID format
    TenantId::parse(&req.tenant_id)
        .rhelma_context("invalid tenant ID")?;
    
    // Validate amounts
    if req.amount <= 0.0 {
        return Err(RhelmaError::Validation("amount must be positive".into()));
    }
    
    Ok(())
}
```

### Enforce Authentication

```rust
async fn require_auth(
    Extension(ctx): Extension<RequestContext>,
) -> RhelmaResult<()> {
    // Ensure we have authenticated user
    if ctx.user_id().is_none() {
        return Err(RhelmaError::Auth("authentication required".into()));
    }
    
    Ok(())
}
```

### Enforce Authorization

```rust
fn require_permission(
    ctx: &RequestContext,
    required_scope: &str,
) -> RhelmaResult<()> {
    if !ctx.scopes.contains(&required_scope.to_string()) {
        return Err(RhelmaError::Authz(
            format!("scope '{}' required", required_scope)
        ));
    }
    
    Ok(())
}
```

---

## Testing

### Unit Tests with Mocked Context

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rhelma_core::prelude::*;
    
    fn mock_context() -> RequestContext {
        RequestContext::empty()
            .with_tenant(TenantId::parse("test-tenant").unwrap())
            .with_region(RegionId::parse("test-region").unwrap())
    }
    
    #[tokio::test]
    async fn test_create_invoice() {
        let ctx = mock_context();
        let result = create_invoice(&ctx, InvoiceRequest::default()).await;
        
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_invalid_email() {
        let result = Email::parse("not-an-email");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_invalid_tenant_id() {
        let result = TenantId::parse("INVALID");
        assert!(result.is_err());
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_full_request_flow() {
    let cfg = AppConfig::from_env_only().unwrap();
    let pool = create_pool(&cfg).await.unwrap();
    
    let ctx = RequestContext::from_headers(vec![
        ("x-request-id", "550e8400-e29b-41d4-a716-446655440000"),
        ("x-tenant-id", "test-tenant"),
        ("x-region", "us-west-2"),
    ]).unwrap();
    
    let result = create_invoice(&pool, &ctx, test_invoice_request()).await;
    assert!(result.is_ok());
}
```

---

## Troubleshooting

### "RequestContext has private fields"

**Problem:**
```rust
let mut ctx = RequestContext::empty();
ctx.tenant_id = Some(...);  // ❌ Error
```

**Solution:** Use builder methods:
```rust
let ctx = RequestContext::empty()
    .with_tenant(TenantId::parse("acme")?);
```

### "from_headers returns Result"

**Problem:**
```rust
let ctx = RequestContext::from_headers(headers);  // ❌ Type mismatch
```

**Solution:** Handle the Result:
```rust
let ctx = RequestContext::from_headers(headers)?;
```

### "Invalid tenant ID format"

**Problem:**
```rust
TenantId::parse("MY-COMPANY")?;  // ❌ Uppercase not allowed
```

**Solution:** Use lowercase:
```rust
TenantId::parse("my-company")?;
```

### "Email validation fails"

**Problem:**
```rust
Email::parse("user@")?;  // ❌ Invalid format
```

**Solution:** Use valid RFC 5322 email:
```rust
Email::parse("user@example.com")?;
```

---

**Last Updated:** December 6, 2025







