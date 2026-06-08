# Observability v5.1 — Logs, Metrics & Traces

**Document:** 07-OBSERVABILITY.md  
**Version:** 5.1.0  
**Status:** Final

---

## Table of Contents

1. [Overview](#overview)
2. [Philosophy](#philosophy)
3. [Structured Logging](#structured-logging)
4. [Metrics](#metrics)
5. [Distributed Tracing](#distributed-tracing)
6. [Configuration](#configuration)
7. [Best Practices](#best-practices)
8. [Examples](#examples)
9. [Integration Checklist](#integration-checklist)

---

## Overview

**Observability** is how we understand system behavior:

- 📊 **Logs** — What happened?
- 📈 **Metrics** — How often? How fast?
- 🔗 **Traces** — Why did it happen? (causality)

**Together:** Complete visibility into production systems.

---

## Philosophy

### Three Pillars

**1. Logs (Events)**
- What happened at specific points
- Immutable historical record
- Human-readable messages
- Searchable by timestamp, service, tenant

**2. Metrics (Time-Series)**
- Aggregated measurements
- How often, how long, how many?
- Real-time dashboards
- Alerting & SLA tracking

**3. Traces (Causality)**
- How requests flow through system
- Request ID ties everything together
- Service-to-service calls
- Error paths & latency hotspots

### Correlation via RequestContext

All three pillar are tied together via **RequestContext**:

```
┌─────────────────────────────────────────────────────┐
│ HTTP Request with RequestContext                    │
│ request_id: "abc-123"                              │
│ correlation_id: "xyz-789"                          │
│ tenant_id: "acme-corp"                             │
└─────────────────────────────────────────────────────┘
         │
         ├─→ [LOG] request_id=abc-123, message="Request started"
         ├─→ [METRIC] request_count{service="api"}++
         ├─→ [TRACE] request_id=abc-123, span="handle_request"
         │
         ├─→ DB Query
         │   ├─→ [LOG] query_duration_ms=45
         │   ├─→ [METRIC] db_query_duration{operation="select"}
         │   └─→ [TRACE] span="db.query"
         │
         └─→ [LOG] request_id=abc-123, message="Request completed"
```

---

## Structured Logging

### JSON Format (Required)

All logs MUST be JSON:

```json
{
  "timestamp": "2025-12-06T10:30:45.123Z",
  "level": "info",
  "message": "Invoice created",
  "service": {
    "name": "billing-service",
    "version": "1.2.3"
  },
  "request": {
    "request_id": "550e8400-e29b-41d4-a716-446655440000",
    "correlation_id": "abc-123",
    "tenant_id": "acme-corp",
    "user_id": "user-123"
  },
  "trace": {
    "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
    "span_id": "00f067aa0ba902b7"
  },
  "fields": {
    "invoice_id": "inv-456",
    "amount": 99.99
  }
}
```

### Log Levels

| Level | When to Use | Example |
|-------|------------|---------|
| **DEBUG** | Internal diagnostic info | "Variable x = 42" |
| **INFO** | User-visible operations | "Invoice created" |
| **WARN** | Warning, but continuing | "Retry attempt 2/3" |
| **ERROR** | Error, operation failed | "Database connection failed" |

### Using Tracing Crate

```rust
use tracing::{debug, info, warn, error};

// Info level
info!(request_id = %ctx.request_id(), "Processing request");

// Warn level
warn!(
    request_id = %ctx.request_id(),
    retry_count = 2,
    "Retrying operation"
);

// Error level
error!(
    request_id = %ctx.request_id(),
    error = ?err,
    "Operation failed"
);

// Debug level (can be disabled in production)
debug!(
    tenant_id = tenant.as_str(),
    "Loaded tenant profile"
);
```

### Mandatory Fields

Every log MUST include:

```rust
info!(
    request_id = %ctx.request_id(),
    tenant_id = ?ctx.tenant_id().map(|t| t.as_str()),
    correlation_id = ctx.correlation_id(),
    "Operation completed"
);
```

**Minimum fields:**
- `timestamp` (automatic)
- `level` (automatic)
- `message` (from string)
- `request_id` (from RequestContext)
- `service.name` (from config)

### PII Protection

**NEVER log:**
- ❌ Passwords
- ❌ API keys
- ❌ OAuth tokens
- ❌ Full credit card numbers
- ❌ SSN/government IDs
- ❌ Raw user-provided content

**Instead:**
```rust
// ❌ Bad: leaks email
warn!(email = user_email, "User not found");

// ✅ Good: safe logging
warn!(email = email.redacted(), "User not found");
```

**Safe approach:**
```rust
// Hash or redact sensitive data
let email_hash = hash(&email);

warn!(
    request_id = %ctx.request_id(),
    email_hash = email_hash,  // Hashed, safe to log
    "User not found"
);
```

---

## Metrics

### Prometheus Format

All metrics MUST be Prometheus-compatible:

```
# Counter (increases only)
request_total{service="api", status="200"} 1523

# Gauge (can increase/decrease)
active_connections{service="api"} 42

# Histogram (distribution)
request_duration_seconds_bucket{service="api", le="0.1"} 100
request_duration_seconds_bucket{service="api", le="0.5"} 150
request_duration_seconds_bucket{service="api", le="1.0"} 180
```

### Standard Metrics

**HTTP Metrics:**
```
http_request_total{service, method, route, status}
http_request_duration_seconds{service, method, route, status}
```

**Database Metrics:**
```
db_query_duration_seconds{service, operation, status}
db_query_total{service, operation, status}
db_errors_total{service, operation}
```

**Error Metrics:**
```
errors_total{service, error_type}
```

### Using Prometheus Client

```rust
use prometheus::{Counter, Histogram};

lazy_static::lazy_static! {
    static ref REQUEST_COUNT: Counter = Counter::new(
        "http_request_total",
        "Total HTTP requests"
    ).unwrap();
    
    static ref REQUEST_DURATION: Histogram = Histogram::new(
        "http_request_duration_seconds",
        "HTTP request duration"
    ).unwrap();
}

// Increment counter
REQUEST_COUNT.inc();

// Record histogram
let start = Instant::now();
// ... do work ...
REQUEST_DURATION.observe(start.elapsed().as_secs_f64());
```

### Tenant-Aware Metrics

Always include tenant_id in metrics:

```rust
REQUEST_COUNT
    .with_label_values(&["acme-corp", "200"])
    .inc();
```

---

## Distributed Tracing

### W3C Traceparent Header

Format: `00-trace_id-span_id-trace_flags`

Example: `00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01`

**Parts:**
- `00` — Version (always 00)
- `4bf92f35...` — Trace ID (128-bit hex)
- `00f067aa...` — Span ID (64-bit hex)
- `01` — Trace flags (01 = sampled)

### Span Types

| Span | Service | Purpose |
|------|---------|---------|
| **http.server** | API Gateway | Incoming HTTP request |
| **http.client** | Service A | Outgoing HTTP call to Service B |
| **db.query** | Service | Database operation |
| **cache.get** | Service | Cache lookup |
| **message.process** | Consumer | Event processing |

### Span Attributes

```rust
span.set_attribute("service.name", "api-gateway");
span.set_attribute("http.method", "POST");
span.set_attribute("http.route", "/invoices");
span.set_attribute("http.status_code", 200);
span.set_attribute("db.operation", "select");
span.set_attribute("db.statement", "SELECT * FROM invoices");
span.set_attribute("tenant_id", "acme-corp");
span.set_attribute("error", true);
span.set_attribute("error.message", err.to_string());
```

### OTLP Export

Export traces to OpenTelemetry collector:

```rust
let tracer = opentelemetry_jaeger::new_pipeline()
    .install_simple()
    .unwrap();

let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

tracing_subscriber::registry()
    .with(telemetry)
    .init();
```

---

## Configuration

### UnifiedObservabilityConfig

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
```

### Creating from AppConfig

```rust
let cfg = AppConfig::from_env_only()?;
let obs = UnifiedObservabilityConfig::from_app_config(&cfg);

println!("Service: {} in {}", obs.service_name, obs.environment);
println!("JSON logs: {}", obs.json_logs);
println!("OTLP enabled: {}", obs.otlp_enabled);
```

### Environment Variables

```bash
# Enable JSON logging
RHELMA_JSON_LOGS=true

# Enable OTLP
RHELMA_OBS__ENABLE_OTLP=true

# OTLP endpoint
RHELMA_OBS__OTLP_ENDPOINT=http://otel-collector:4317

# Log level
RHELMA_OBS__LOG_LEVEL=debug
```

---

## Best Practices

### ✅ Do's

1. **Always include request_id**
   ```rust
   info!(request_id = %ctx.request_id(), "Message");
   ```

2. **Use appropriate log levels**
   ```rust
   debug!("Variable state");     // Developer info
   info!("User action");          // User-visible events
   warn!("Degraded behavior");    // Warning
   error!("Operation failed");    // Errors
   ```

3. **Include context in all logs**
   ```rust
   info!(
       request_id = %ctx.request_id(),
       tenant_id = ?ctx.tenant_id(),
       "Operation"
   );
   ```

4. **Emit metrics for operations**
   ```rust
   REQUEST_DURATION.observe(elapsed.as_secs_f64());
   ```

5. **Use spans for tracing**
   ```rust
   let span = info_span!("db_query", request_id = %ctx.request_id());
   let _enter = span.enter();
   // ... query ...
   ```

### ❌ Don'ts

1. **Don't log passwords/secrets**
   ```rust
   // ❌ Bad
   error!(password = pwd, "Auth failed");
   
   // ✅ Good
   error!("Authentication failed");
   ```

2. **Don't skip request_id**
   ```rust
   // ❌ Bad
   info!("Processing");
   
   // ✅ Good
   info!(request_id = %ctx.request_id(), "Processing");
   ```

3. **Don't use println! in production code**
   ```rust
   // ❌ Bad
   println!("Debug: {:?}", data);
   
   // ✅ Good
   debug!("Data: {:?}", data);
   ```

4. **Don't forget tenant_id**
   ```rust
   // ❌ Bad
   info!("User action");
   
   // ✅ Good
   info!(tenant_id = tenant.as_str(), "User action");
   ```

---

## Examples

### Example 1: Structured Logging Setup

```rust
use tracing_subscriber;

fn init_logging(obs: &UnifiedObservabilityConfig) -> Result<()> {
    let env_filter = tracing_subscriber::EnvFilter::new(
        obs.log_level.as_deref().unwrap_or("info")
    );

    tracing_subscriber::fmt()
        .json()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_level(true)
        .init();

    Ok(())
}
```

### Example 2: Request Logging

```rust
async fn handle_request(
    Extension(ctx): Extension<RequestContext>,
    Json(req): Json<Request>,
) -> Result<Json<Response>, RhelmaError> {
    info!(
        request_id = %ctx.request_id(),
        tenant_id = ?ctx.tenant_id().map(|t| t.as_str()),
        "Request started"
    );

    let start = Instant::now();

    match process(&ctx, &req).await {
        Ok(resp) => {
            let elapsed = start.elapsed();
            info!(
                request_id = %ctx.request_id(),
                duration_ms = elapsed.as_millis(),
                "Request completed successfully"
            );
            
            // Record metric
            REQUEST_DURATION.observe(elapsed.as_secs_f64());
            
            Ok(Json(resp))
        }
        Err(err) => {
            error!(
                request_id = %ctx.request_id(),
                error = %err,
                error_code = err.as_str(),
                "Request failed"
            );
            
            // Record error metric
            ERRORS_TOTAL
                .with_label_values(&[err.as_str()])
                .inc();
            
            Err(err)
        }
    }
}
```

### Example 3: Database Operation Tracing

```rust
async fn query_invoices(
    ctx: &RequestContext,
    pool: &PgPool,
) -> RhelmaResult<Vec<Invoice>> {
    let span = info_span!(
        "db.query",
        request_id = %ctx.request_id(),
        operation = "select"
    );

    async {
        let start = Instant::now();

        let invoices = sqlx::query_as::<_, Invoice>(
            "SELECT * FROM invoices WHERE tenant_id = $1"
        )
        .bind(ctx.tenant_id()?.as_str())
        .fetch_all(pool)
        .await
        .rhelma_context("while querying invoices")?;

        let elapsed = start.elapsed();

        info!(
            duration_ms = elapsed.as_millis(),
            count = invoices.len(),
            "Query completed"
        );

        DB_QUERY_DURATION
            .with_label_values(&["select"])
            .observe(elapsed.as_secs_f64());

        Ok(invoices)
    }
    .instrument(span)
    .await
}
```

---

## Integration Checklist

- ✅ JSON logging configured (structured)
- ✅ Request ID propagated (in all logs)
- ✅ Tenant ID included (in all logs)
- ✅ Log levels appropriate (debug/info/warn/error)
- ✅ PII not logged (redacted or excluded)
- ✅ Metrics emitted (counter, histogram)
- ✅ Traces exported (OTLP enabled)
- ✅ Span context propagated (W3C traceparent)
- ✅ Error handling logged (with error code)
- ✅ Latency tracked (histogram)

---

**Last Updated:** December 6, 2025







