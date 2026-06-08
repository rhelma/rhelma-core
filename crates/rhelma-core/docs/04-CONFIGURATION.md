# Configuration Management v5.1

**Document:** 04-CONFIGURATION.md  
**Version:** 5.1.0  
**Status:** Final

---

## Table of Contents

1. [Overview](#overview)
2. [Philosophy](#philosophy)
3. [AppConfig Structure](#appconfig-structure)
4. [Loading](#loading)
5. [Validation](#validation)
6. [Environment Variables](#environment-variables)
7. [Observability Config](#observability-config)
8. [Best Practices](#best-practices)
9. [Examples](#examples)
10. [Troubleshooting](#troubleshooting)

---

## Overview

**Configuration in rhelma-core is:**

- 🔐 **Minimal** — Only what the crate needs
- 📝 **Environment-based** — From environment variables only
- ✅ **Validated** — Strict rules, fail-fast on invalid config
- 🚫 **No secrets** — Secrets come from KMS, not AppConfig
- 🔄 **Two-phase** — Load, then validate (separate concerns)

**Philosophy:** Simple, deterministic, secure.

---

## Philosophy

### No Config Files

❌ **Not supported:**
- YAML config files
- JSON config files
- TOML files
- Environment override files (.env with secrets)

✅ **Only:**
- Environment variables
- Defaults in code
- KMS for secrets (AWS KMS, HashiCorp Vault, etc.)

**Why?**
- Single source of truth (environment)
- No file path confusion
- Easier in containers
- Works in all deployment models
- Git-safe (no accidental secret commits)

### Two-Phase Loading

**Phase 1: Load (No Validation)**
```rust
let cfg = AppConfig::from_env_only()?;
// Returns raw config with defaults, no validation
```

**Phase 2: Validate (Strict Checks)**
```rust
cfg.validate_all()?;  // Aborts if invalid
```

**Why separate?**
- Tests can override without full validation
- Services can inspect raw config
- Clear error point (validation failure = startup abort)
- Logging can see what was loaded

### Fail-Fast Startup

Invalid configuration **aborts service startup**:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let cfg = AppConfig::from_env_only()?;
    cfg.validate_all()?;  // ← Returns Err if invalid
    
    // If we reach here, config is guaranteed valid
    start_service(cfg).await?;
}
```

**No silent fallbacks:**
- ❌ Invalid region → Error (not default)
- ❌ Wrong environment → Error (not "development")
- ❌ Missing required values → Error (not empty string)

---

## AppConfig Structure

### Full Schema

```rust
pub struct AppConfig {
    /// Execution environment: development | staging | production
    pub environment: String,

    /// Deployment region (Rhelma multi-region identity)
    pub region: String,

    /// Whether JSON logs are explicitly enabled
    pub json_logs: Option<bool>,

    /// Logical service name (for observability & auth)
    pub service_name: Option<String>,

    /// Logical version identifier
    pub service_version: Option<String>,

    /// Optional default tenant tier (SaaS override)
    pub default_tenant_tier: Option<String>,
}
```

### Fields Explained

| Field | Type | Required | Format | Example |
|-------|------|----------|--------|---------|
| `environment` | String | ✅ Yes | One of: `development`, `staging`, `production` | `production` |
| `region` | String | ✅ Yes | Lowercase alphanumeric + `-`, min 3 chars | `eu-west-1` |
| `json_logs` | Option<bool> | ❌ No | Boolean | `true` |
| `service_name` | Option<String> | ⚠️ Required in prod | Any string | `api-gateway` |
| `service_version` | Option<String> | ❌ No | Semantic version | `1.2.3` |
| `default_tenant_tier` | Option<String> | ❌ No | Tier name | `tier2` |

---

## Loading

### from_env_only()

```rust
pub fn from_env_only() -> Result<Self, RhelmaError>
```

Loads raw configuration from environment **without validation**.

**Process:**

1. Read environment variables
2. Apply defaults
3. Return config object
4. No validation (deferred to validate_all)

**Default Values:**

| Variable | Default | If Not Set |
|----------|---------|-----------|
| `RHELMA_ENV` / `RHELMA_ENVIRONMENT` | - | "development" |
| `RHELMA_REGION` | - | "local" |
| `RHELMA_JSON_LOGS` | - | None (logs as text) |
| `RHELMA_SERVICE_NAME` | - | None |
| `RHELMA_SERVICE_VERSION` | - | None |
| `RHELMA_DEFAULT_TENANT_TIER` | - | None |

**Example:**

```rust
let cfg = AppConfig::from_env_only()?;

// If RHELMA_ENV not set:
assert_eq!(cfg.environment, "development");

// If RHELMA_REGION not set:
assert_eq!(cfg.region, "local");

// If RHELMA_JSON_LOGS not set:
assert_eq!(cfg.json_logs, None);
```

**Error Cases:**

Returns `RhelmaError::Config` only if:
- Environment variable parsing fails (unlikely)
- Boolean parsing fails (invalid value for RHELMA_JSON_LOGS)

---

## Validation

### validate_all()

```rust
pub fn validate_all(&self) -> Result<(), RhelmaError>
```

Validates entire configuration with **strict rules**.

**Validations:**

#### 1. Environment Validation

```rust
// Must be exactly one of:
"development"
"staging"
"production"

// ❌ Not accepted:
"dev", "prod", "Dev", "PROD", "Development" (with capital D)
```

**Error:**
```
RhelmaError::Config("environment must be one of [development, staging, production], got 'prod'")
```

#### 2. Region Validation

```rust
// Must match: [a-z0-9-]{3,}
// Min 3 characters
// Only lowercase letters, digits, hyphens

✅ Valid:
  "local"           // Exactly 3 chars
  "us-west-2"       // Lowercase, hyphen-separated
  "eu-west-1"
  "ap-southeast-1"

❌ Invalid:
  "us"              // Too short (< 3)
  "US-WEST-2"       // Uppercase
  "us_west_2"       // Underscore (hyphen required)
  "us west 2"       // Space
```

**Error:**
```
RhelmaError::Config("invalid region format: US-WEST-2")
```

#### 3. Service Name in Production

```rust
// In production, service_name is required (cannot be None)

if environment == "production" && service_name.is_none() {
    return Err(RhelmaError::Config("RHELMA_SERVICE_NAME required in production"));
}
```

**Error:**
```
RhelmaError::Config("RHELMA_SERVICE_NAME required in production")
```

**Why?** Production monitoring requires service identification.

### Validation Patterns

**Example 1: Development Environment**

```rust
let cfg = AppConfig {
    environment: "development".into(),
    region: "local".into(),
    service_name: None,  // Optional in dev
    ..Default::default()
};

cfg.validate_all()?;  // ✅ Passes
```

**Example 2: Production Environment**

```rust
let cfg = AppConfig {
    environment: "production".into(),
    region: "eu-west-1".into(),
    service_name: Some("api-gateway".into()),  // Required
    ..Default::default()
};

cfg.validate_all()?;  // ✅ Passes
```

**Example 3: Invalid Environment**

```rust
let cfg = AppConfig {
    environment: "prod".into(),  // ❌ Alias not accepted
    region: "local".into(),
    ..Default::default()
};

cfg.validate_all()?;  // ❌ Error
```

---

## Environment Variables

### Complete Reference

#### Core Configuration

```bash
# Execution environment (required, validated)
RHELMA_ENV=production
# OR
RHELMA_ENVIRONMENT=production

# Deployment region (required, validated)
RHELMA_REGION=eu-west-1

# Service identifier (required in production)
RHELMA_SERVICE_NAME=api-gateway

# Service version (optional)
RHELMA_SERVICE_VERSION=1.2.3
```

#### Logging Configuration

```bash
# Enable JSON logging (boolean)
RHELMA_JSON_LOGS=true

# Log level (for observability)
RHELMA_OBS__LOG_LEVEL=info
```

#### OpenTelemetry Configuration

```bash
# Enable OTLP export
RHELMA_OBS__ENABLE_OTLP=true

# OTLP collector endpoint
RHELMA_OBS__OTLP_ENDPOINT=http://otel-collector:4317
```

#### Custom Configuration (Tenant Tier)

```bash
# Default tenant tier for new tenants
RHELMA_DEFAULT_TENANT_TIER=tier2
```

### Examples

**Development Setup:**
```bash
RHELMA_ENV=development
RHELMA_REGION=local
RHELMA_SERVICE_NAME=my-service
RHELMA_JSON_LOGS=false
```

**Staging Setup:**
```bash
RHELMA_ENV=staging
RHELMA_REGION=us-west-2
RHELMA_SERVICE_NAME=api-gateway
RHELMA_JSON_LOGS=true
RHELMA_OBS__ENABLE_OTLP=true
RHELMA_OBS__OTLP_ENDPOINT=http://otel-collector:4317
```

**Production Setup:**
```bash
RHELMA_ENV=production
RHELMA_REGION=eu-west-1
RHELMA_SERVICE_NAME=api-gateway
RHELMA_SERVICE_VERSION=1.2.3
RHELMA_JSON_LOGS=true
RHELMA_OBS__ENABLE_OTLP=true
RHELMA_OBS__OTLP_ENDPOINT=https://otel.prod.example.com:4317
RHELMA_OBS__LOG_LEVEL=warn
```

**Kubernetes Example:**
```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: app-config
data:
  RHELMA_ENV: "production"
  RHELMA_REGION: "eu-west-1"
  RHELMA_SERVICE_NAME: "api-gateway"
  RHELMA_JSON_LOGS: "true"
  RHELMA_OBS__ENABLE_OTLP: "true"
---
apiVersion: v1
kind: Pod
metadata:
  name: my-service
spec:
  containers:
  - name: app
    image: my-app:1.0
    envFrom:
    - configMapRef:
        name: app-config
```

---

## Observability Config

### UnifiedObservabilityConfig

Derived from AppConfig + observability variables:

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

### Creation

```rust
let cfg = AppConfig::from_env_only()?;
cfg.validate_all()?;

let obs = UnifiedObservabilityConfig::from_app_config(&cfg);

println!("Service: {} in {}/{}", obs.service_name, obs.environment, obs.region);
println!("JSON logs: {}", obs.json_logs);
println!("OTLP enabled: {}", obs.otlp_enabled);
```

### How It's Built

1. **service_name:**
   - From AppConfig.service_name, OR
   - From RHELMA_SERVICE_NAME env var, OR
   - If production: panic!("RHELMA_SERVICE_NAME required")
   - Otherwise: "unknown-service"

2. **environment:**
   - From AppConfig.environment

3. **region:**
   - From AppConfig.region

4. **json_logs:**
   - From AppConfig.json_logs.unwrap_or(false)

5. **otlp_enabled:**
   - From RHELMA_OBS__ENABLE_OTLP env var (default: false)

6. **otlp_endpoint:**
   - From RHELMA_OBS__OTLP_ENDPOINT env var (optional)

7. **log_level:**
   - From RHELMA_OBS__LOG_LEVEL env var (optional)

---

## Best Practices

### ✅ Do's

1. **Validate immediately at startup**
   ```rust
   #[tokio::main]
   async fn main() -> Result<()> {
       let cfg = AppConfig::from_env_only()?;
       cfg.validate_all()?;  // ← Right here
       start_service(cfg).await
   }
   ```

2. **Use environment variables everywhere**
   ```rust
   // ✅ Good: from environment
   RHELMA_ENV=production cargo run
   
   // ❌ Bad: hardcoded
   let env = "production";
   ```

3. **Set region correctly**
   ```bash
   # ✅ Good: lowercase, hyphenated
   RHELMA_REGION=us-west-2
   
   # ❌ Bad: uppercase
   RHELMA_REGION=US-WEST-2
   
   # ❌ Bad: underscore
   RHELMA_REGION=us_west_2
   ```

4. **Provide service name in production**
   ```bash
   # ✅ Good: set in prod
   RHELMA_ENV=production
   RHELMA_SERVICE_NAME=api-gateway
   
   # ❌ Bad: missing in prod
   RHELMA_ENV=production
   # RHELMA_SERVICE_NAME not set → Error!
   ```

5. **Use observability config**
   ```rust
   let obs = UnifiedObservabilityConfig::from_app_config(&cfg);
   init_tracing(&obs)?;
   init_metrics(&obs)?;
   ```

### ❌ Don'ts

1. **Don't use config files**
   ```rust
   // ❌ Bad
   let cfg = toml::from_str(file_content)?;
   
   // ✅ Good
   let cfg = AppConfig::from_env_only()?;
   ```

2. **Don't store secrets in AppConfig**
   ```rust
   // ❌ Bad: leaks secret
   pub database_password: String,
   
   // ✅ Good: from KMS
   let password = kms.get_secret("db-password").await?;
   ```

3. **Don't skip validation**
   ```rust
   // ❌ Bad: no validation
   let cfg = AppConfig::from_env_only()?;
   start_service(cfg);
   
   // ✅ Good: validate first
   let cfg = AppConfig::from_env_only()?;
   cfg.validate_all()?;
   start_service(cfg);
   ```

4. **Don't use aliases for environment**
   ```bash
   # ❌ Bad: alias not accepted
   RHELMA_ENV=prod
   
   # ✅ Good: exact name
   RHELMA_ENV=production
   ```

5. **Don't modify config during runtime**
   ```rust
   // ✅ Config is read at startup, immutable after
   // Changes require restart
   ```

---

## Examples

### Example 1: Minimal Service Startup

```rust
use rhelma_core::prelude::*;

#[tokio::main]
async fn main() -> RhelmaResult<()> {
    // Load config
    let cfg = AppConfig::from_env_only()?;
    
    // Validate immediately
    cfg.validate_all()
        .rhelma_context("during configuration validation")?;
    
    // Setup observability
    let obs = UnifiedObservabilityConfig::from_app_config(&cfg);
    init_tracing(&obs)?;
    
    info!(
        service = obs.service_name,
        environment = obs.environment,
        region = obs.region,
        "Service starting"
    );
    
    // Service is ready
    Ok(())
}

fn init_tracing(obs: &UnifiedObservabilityConfig) -> RhelmaResult<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(obs.log_level.as_deref().unwrap_or("info"))
        .init();
    Ok(())
}
```

### Example 2: Configuration-Dependent Behavior

```rust
async fn start_service(cfg: AppConfig) -> RhelmaResult<()> {
    // Different behavior per environment
    match cfg.environment.as_str() {
        "development" => {
            println!("Running in development mode");
            init_test_data().await?;
        }
        "staging" => {
            println!("Running in staging mode");
            validate_external_services().await?;
        }
        "production" => {
            println!("Running in production mode");
            enable_security_checks().await?;
        }
        _ => unreachable!(),  // Validated already
    }
    
    Ok(())
}
```

### Example 3: Region-Aware Service

```rust
async fn connect_to_regional_service(cfg: &AppConfig) -> RhelmaResult<Client> {
    let endpoint = format!(
        "https://{}.service.internal",
        cfg.region
    );
    
    Client::connect(&endpoint)
        .await
        .rhelma_context(&format!("while connecting to {}", endpoint))
}
```

---

## Troubleshooting

### "environment must be one of..."

**Problem:**
```bash
RHELMA_ENV=prod  # ❌ Alias not accepted
```

**Solution:** Use exact environment name
```bash
RHELMA_ENV=production  # ✅ Correct
```

### "invalid region format"

**Problem:**
```bash
RHELMA_REGION=US-WEST-2  # ❌ Uppercase
RHELMA_REGION=us_west_2  # ❌ Underscore
RHELMA_REGION=us         # ❌ Too short
```

**Solution:** Use correct format
```bash
RHELMA_REGION=us-west-2  # ✅ Lowercase, hyphen, 3+ chars
```

### "RHELMA_SERVICE_NAME required in production"

**Problem:**
```rust
RHELMA_ENV=production
// RHELMA_SERVICE_NAME not set
```

**Solution:** Set service name in production
```bash
RHELMA_ENV=production
RHELMA_SERVICE_NAME=my-service
```

### "service_name is None"

**Problem:**
```rust
let obs = UnifiedObservabilityConfig::from_app_config(&cfg);
println!("{}", obs.service_name);  // Empty or "unknown-service"
```

**Solution:** Set RHELMA_SERVICE_NAME environment variable
```bash
RHELMA_SERVICE_NAME=api-gateway
```

### "Configuration loading failed"

**Problem:**
```rust
let cfg = AppConfig::from_env_only()?;  // Returns Err
```

**Debugging:**
```rust
match AppConfig::from_env_only() {
    Ok(cfg) => println!("Loaded: {:?}", cfg),
    Err(e) => eprintln!("Failed: {}", e),  // Shows what went wrong
}
```

---

**Last Updated:** December 6, 2025







