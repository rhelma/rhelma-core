# Tenancy & Residency Governance v5.1

**Document:** 05-TENANCY-RESIDENCY.md  
**Version:** 5.1.0  
**Status:** Final

---

## Table of Contents

1. [Overview](#overview)
2. [Core Concepts](#core-concepts)
3. [TenantProfile](#tenantprofile)
4. [Tenancy Tiers](#tenancy-tiers)
5. [Residency Policies](#residency-policies)
6. [Validation](#validation)
7. [Isolation Patterns](#isolation-patterns)
8. [GDPR Compliance](#gdpr-compliance)
9. [Best Practices](#best-practices)
10. [Examples](#examples)

---

## Overview

**Multi-tenant governance** ensures:

- 🏢 **Tenant Isolation** — Data never leaks between tenants
- 🌍 **Data Residency** — GDPR/regulatory compliance
- 📊 **SLA Management** — Different tiers, different SLAs
- 🛡️ **Security** — Zero-Trust tenant validation
- 💰 **Cost Attribution** — Per-tenant billing

Every rhelma-core service MUST implement multi-tenant governance.

---

## Core Concepts

### Tenant

A tenant is a **customer/organization** with:
- Unique tenant_id (validated string)
- Configuration & preferences
- Data isolation requirements
- SLA/pricing tier
- Residency policy
- Security settings

### TenantId

```rust
pub struct TenantId(pub String);
```

- ✅ Validated format (lowercase alphanumeric + `-`)
- ✅ Unique per customer
- ✅ Immutable once created
- ✅ Always included in queries/logs

### Multi-Tenancy Model

Every service is **multi-tenant by default**:

```
API Request
    ↓
Extract tenant_id from RequestContext
    ↓
Load TenantProfile
    ↓
Enforce isolation rules
    ↓
All data filtered by tenant_id
    ↓
Log with tenant_id
```

### Key Rule

**No request operates on data from another tenant.**

```rust
// ✅ Correct: filters by tenant
SELECT * FROM invoices 
WHERE tenant_id = $1 AND user_id = $2

// ❌ Wrong: missing tenant filter
SELECT * FROM invoices 
WHERE user_id = $1
```

---

## TenantProfile

### Structure

```rust
pub struct TenantProfile {
    // Identity
    pub tenant_id: TenantId,
    pub name: String,

    // Isolation
    pub tier: TenancyTier,
    
    // Residency
    pub residency: ResidencyPolicy,
    pub primary_region: RegionId,
    pub backup_regions: Vec<RegionId>,

    // SLA & DR
    pub sla: Option<SlaTarget>,
    pub dr_tier: Option<DrTier>,

    // Features
    pub ai_allowed: bool,
    pub logging_pii_allowed: bool,

    // Extensibility
    pub metadata: serde_json::Value,
}
```

### Methods

```rust
impl TenantProfile {
    // Check isolation level
    pub fn is_isolated(&self) -> bool {
        matches!(
            self.tier,
            TenancyTier::Tier2SharedDbIsolatedSchema | 
            TenancyTier::Tier3DedicatedDb
        )
    }

    // Check if region-sensitive
    pub fn is_region_sensitive(&self) -> bool {
        !matches!(self.residency, ResidencyPolicy::GlobalPreferred)
    }

    // Enforce residency
    pub fn validate_residency(&self, region: &RegionId) -> Result<(), RhelmaError> {
        match self.residency {
            ResidencyPolicy::GlobalPreferred => Ok(()),
            ResidencyPolicy::RegionalPreferred => {
                if region == &self.primary_region 
                    || self.backup_regions.contains(region) {
                    Ok(())
                } else {
                    Err(RhelmaError::SecurityPolicy(
                        format!("region {} not allowed", region.as_str())
                    ))
                }
            }
            ResidencyPolicy::RegionalRequired => {
                if region == &self.primary_region {
                    Ok(())
                } else {
                    Err(RhelmaError::SecurityPolicy(
                        "region residency violation".into()
                    ))
                }
            }
        }
    }
}
```

---

## Tenancy Tiers

### Tier 1: Shared (Maximum Cost Efficiency)

```rust
TenancyTier::Tier1Shared
```

**Database Architecture:**
- Single database
- Single schema
- Single row-level isolation (schema_id column)

**Characteristics:**
- ✅ Cheapest
- ✅ Fast provisioning
- ❌ Maximum blast radius on failure
- ❌ Slowest query isolation

**Use Case:** Startups, free tier, non-critical data

**Isolation:**
```sql
-- Must include tenant_id in WHERE clause
SELECT * FROM data 
WHERE schema_id = $1  -- Tenant identifier
AND customer_id = $2
```

### Tier 2: Schema-Isolated (Balanced)

```rust
TenancyTier::Tier2SharedDbIsolatedSchema
```

**Database Architecture:**
- Single database (PostgreSQL)
- Separate schema per tenant
- Schema-level permissions

**Characteristics:**
- ✅ Good isolation
- ✅ Moderate cost
- ✅ Better performance
- ❌ Database-level failure still affects all

**Use Case:** Mid-market, growing customers

**Isolation:**
```sql
-- PostgreSQL schema per tenant
CREATE SCHEMA "tenant-acme";
CREATE TABLE "tenant-acme".invoices (...);

SELECT * FROM "tenant-acme".invoices;
```

**Connection Management:**
```rust
// Connection pool per tenant
let pool = connect_to_schema(&tenant_id).await?;
// Automatically scoped to tenant schema
```

### Tier 3: Database-Isolated (Maximum Security)

```rust
TenancyTier::Tier3DedicatedDb
```

**Database Architecture:**
- Separate database per tenant (or cluster)
- Full physical isolation
- Independent backup & restore

**Characteristics:**
- ✅ Maximum isolation
- ✅ Full GDPR compliance
- ✅ Easiest disaster recovery
- ❌ Highest cost
- ❌ Operational complexity

**Use Case:** Enterprise, regulated, high-sensitivity data

**Isolation:**
```rust
// Each tenant has separate database
let db_url = format!(
    "postgresql://user:pass@host/tenant-{}",
    tenant_id.as_str()
);
let pool = PgPool::connect(&db_url).await?;
```

### Tier Comparison

| Aspect | Tier 1 | Tier 2 | Tier 3 |
|--------|--------|--------|--------|
| **Cost** | $$ | $$$ | $$$$ |
| **Isolation** | Row-level | Schema | DB |
| **Blast Radius** | All tenants | Shared DB | Single tenant |
| **Query Isolation** | Application | Database | Database + Network |
| **GDPR** | Difficult | Good | Easy |
| **Provisioning** | Instant | Minutes | Hours |
| **Query Performance** | Good | Better | Best |

---

## Residency Policies

### Global Preferred (Default)

```rust
ResidencyPolicy::GlobalPreferred
```

**Rules:**
- Data can reside in ANY region
- No residency constraints
- Optimized for latency (serve from nearest region)

**Use Case:** Non-regulated, global services

**Validation:**
```rust
profile.validate_residency(&any_region)?;  // ✅ Always passes
```

**Example:**
```rust
let profile = TenantProfile {
    residency: ResidencyPolicy::GlobalPreferred,
    primary_region: RegionId::parse("us-west-2")?,
    backup_regions: vec![],
    // ...
};

// Can serve from ANY region
profile.validate_residency(&RegionId::parse("eu-west-1")?)?;  // ✅ OK
profile.validate_residency(&RegionId::parse("ap-south-1")?)?;  // ✅ OK
```

### Regional Preferred (Data Gravity)

```rust
ResidencyPolicy::RegionalPreferred
```

**Rules:**
- PRIMARY region is preferred (best SLA)
- BACKUP regions allowed (fallback only)
- OTHER regions forbidden

**Use Case:** Regulated data, cost optimization

**Validation:**
```rust
profile.validate_residency(&region)?;
// ✅ if region == primary_region
// ✅ if region in backup_regions
// ❌ otherwise
```

**Example:**
```rust
let profile = TenantProfile {
    residency: ResidencyPolicy::RegionalPreferred,
    primary_region: RegionId::parse("eu-west-1")?,
    backup_regions: vec![
        RegionId::parse("eu-central-1")?,
    ],
    // ...
};

// Allowed
profile.validate_residency(&RegionId::parse("eu-west-1")?)?;      // ✅ Primary
profile.validate_residency(&RegionId::parse("eu-central-1")?)?;   // ✅ Backup

// Forbidden
profile.validate_residency(&RegionId::parse("us-west-2")?)?;      // ❌ Error
```

### Regional Required (Strict Residency)

```rust
ResidencyPolicy::RegionalRequired
```

**Rules:**
- Data MUST stay in PRIMARY region only
- NO cross-region replication
- NO backup regions
- Strictest GDPR compliance

**Use Case:** EU GDPR, regulated data, strict residency

**Validation:**
```rust
profile.validate_residency(&region)?;
// ✅ if region == primary_region
// ❌ otherwise (error: SecurityPolicy)
```

**Example:**
```rust
let profile = TenantProfile {
    residency: ResidencyPolicy::RegionalRequired,
    primary_region: RegionId::parse("eu-west-1")?,
    backup_regions: vec![],  // Must be empty!
    // ...
};

// Allowed
profile.validate_residency(&RegionId::parse("eu-west-1")?)?;  // ✅ Primary only

// Forbidden
profile.validate_residency(&RegionId::parse("eu-central-1")?)?;  // ❌ Error
profile.validate_residency(&RegionId::parse("us-west-2")?)?;     // ❌ Error
```

---

## Validation

### validate_residency()

```rust
pub fn validate_residency(&self, region: &RegionId) -> Result<(), RhelmaError>
```

Enforces residency policy at every data operation.

**Enforcement Points:**

1. **API Gateway**
   ```rust
   profile.validate_residency(&ctx.region()?)?;
   ```

2. **Database Write**
   ```rust
   profile.validate_residency(&user_region)?;
   db.insert(data).await?;
   ```

3. **Cache Write**
   ```rust
   profile.validate_residency(&region)?;
   cache.set(key, value).await?;
   ```

4. **Event Publishing**
   ```rust
   profile.validate_residency(&event.region)?;
   event_bus.publish(&event).await?;
   ```

5. **Vector DB**
   ```rust
   profile.validate_residency(&region)?;
   vector_db.insert(&embedding).await?;
   ```

### Error Handling

```rust
match profile.validate_residency(&region) {
    Ok(()) => {
        // Proceed with operation
    }
    Err(RhelmaError::SecurityPolicy(msg)) => {
        // Log compliance violation
        error!("Residency violation: {}", msg);
        
        // Return 403 Forbidden
        return Err(RhelmaError::SecurityPolicy(msg));
    }
    Err(e) => {
        // Other errors
        return Err(e);
    }
}
```

---

## Isolation Patterns

### Pattern 1: Row-Level Isolation (Tier 1)

```rust
// All queries include tenant_id filter
async fn get_user_invoices(
    ctx: &RequestContext,
    pool: &PgPool,
) -> RhelmaResult<Vec<Invoice>> {
    let tenant = ctx.tenant_id()
        .ok_or_else(|| RhelmaError::Auth("missing tenant".into()))?;
    
    sqlx::query_as::<_, Invoice>(
        "SELECT * FROM invoices WHERE tenant_id = $1"
    )
    .bind(tenant.as_str())
    .fetch_all(pool)
    .await?
}
```

### Pattern 2: Schema-Level Isolation (Tier 2)

```rust
// Different schema per tenant
async fn get_schema_pool(tenant_id: &TenantId) -> RhelmaResult<PgPool> {
    let schema = format!("tenant_{}", tenant_id.as_str());
    let url = format!("postgresql://user:pass@host/app?search_path={}", schema);
    
    PgPool::connect(&url).await?
}

// No need to filter by tenant_id; database handles it
async fn get_invoices(pool: &PgPool) -> RhelmaResult<Vec<Invoice>> {
    sqlx::query_as::<_, Invoice>("SELECT * FROM invoices")
        .fetch_all(pool)
        .await?
}
```

### Pattern 3: Database-Level Isolation (Tier 3)

```rust
// Each tenant has completely separate database
async fn get_tenant_pool(tenant_id: &TenantId) -> RhelmaResult<PgPool> {
    let url = format!(
        "postgresql://user:pass@host-{}/app",
        tenant_id.as_str()
    );
    
    PgPool::connect(&url).await?
}

// Complete isolation; no cross-tenant query risk
async fn get_invoices(pool: &PgPool) -> RhelmaResult<Vec<Invoice>> {
    sqlx::query_as::<_, Invoice>("SELECT * FROM invoices")
        .fetch_all(pool)
        .await?
}
```

---

## GDPR Compliance

### Data Subject Rights

**Right to Access:**
```rust
// Retrieve all data for tenant
SELECT * FROM all_tables WHERE tenant_id = $1
```

**Right to Deletion:**
```rust
// Delete all tenant data (Tier 3: drop entire database)
DELETE FROM all_tables WHERE tenant_id = $1
```

**Right to Portability:**
```rust
// Export all tenant data in standard format
let data = export_tenant_data(&tenant_id).await?;
```

### Residency Enforcement for GDPR

```rust
// GDPR: EU citizens' data must stay in EU
let tenant = TenantProfile {
    residency: ResidencyPolicy::RegionalRequired,
    primary_region: RegionId::parse("eu-west-1")?,
    // ...
};

// Any attempt to move data to US fails
tenant.validate_residency(&RegionId::parse("us-west-2")?)?;
// ❌ RhelmaError::SecurityPolicy
```

### Compliance Checklist

- ✅ Tenant data isolated (no cross-tenant access)
- ✅ Residency policy enforced (no geographic violations)
- ✅ Data deletion capability (right to be forgotten)
- ✅ Data export (portability)
- ✅ Audit trail (RequestContext.request_id in all logs)
- ✅ Encryption (at-rest and in-transit)

---

## Best Practices

### ✅ Do's

1. **Always filter by tenant_id**
   ```rust
   WHERE tenant_id = $1  // ✅ Required
   ```

2. **Extract tenant from context**
   ```rust
   let tenant = ctx.tenant_id()
       .ok_or_else(|| RhelmaError::Auth("missing tenant".into()))?;
   ```

3. **Validate residency before operations**
   ```rust
   profile.validate_residency(&region)?;
   ```

4. **Include tenant_id in logs**
   ```rust
   info!(tenant_id = tenant.as_str(), "Operation");
   ```

5. **Test cross-tenant isolation**
   ```rust
   // Verify user from tenant A cannot access tenant B data
   ```

### ❌ Don'ts

1. **Don't skip tenant filter**
   ```rust
   SELECT * FROM invoices  // ❌ Missing WHERE tenant_id
   ```

2. **Don't trust user input for tenant**
   ```rust
   SELECT * FROM invoices WHERE tenant_id = req.tenant_id
   // ❌ User can claim any tenant
   
   SELECT * FROM invoices WHERE tenant_id = ctx.tenant_id()
   // ✅ From RequestContext (validated)
   ```

3. **Don't replicate strict-residency data**
   ```rust
   // ❌ GDPR violation
   tenant.residency == RegionalRequired && replicate_to_us?
   ```

4. **Don't hardcode region assumptions**
   ```rust
   store_in_region("us-west-2")  // ❌ Ignores residency
   
   profile.validate_residency(&region)?;
   store_in_region(&region)  // ✅ Respects policy
   ```

---

## Examples

### Example 1: Multi-Tenant Query

```rust
async fn get_tenant_invoices(
    ctx: &RequestContext,
    pool: &PgPool,
    page: PageRequest,
) -> RhelmaResult<Paginated<Invoice>> {
    // Extract tenant (required)
    let tenant = ctx.tenant_id()
        .ok_or_else(|| RhelmaError::Auth("missing tenant".into()))?;
    
    // Load tenant profile
    let profile = load_tenant_profile(tenant).await?;
    
    // Validate residency
    let region = ctx.region()
        .ok_or_else(|| RhelmaError::BadRequest("missing region".into()))?;
    profile.validate_residency(region)
        .rhelma_context("during residency validation")?;
    
    // Query with tenant filter
    let page = page.normalized();
    
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM invoices WHERE tenant_id = $1"
    )
    .bind(tenant.as_str())
    .fetch_one(pool)
    .await?;
    
    let items = sqlx::query_as::<_, Invoice>(
        "SELECT * FROM invoices WHERE tenant_id = $1 \
         ORDER BY created_at DESC LIMIT $2 OFFSET $3"
    )
    .bind(tenant.as_str())
    .bind(page.limit as i64)
    .bind(page.offset as i64)
    .fetch_all(pool)
    .await?;
    
    Ok(Paginated {
        items,
        total: total as u64,
        offset: page.offset,
        limit: page.limit,
    })
}
```

### Example 2: GDPR Data Deletion

```rust
async fn delete_tenant_data(
    pool: &PgPool,
    tenant_id: &TenantId,
) -> RhelmaResult<()> {
    // Delete in order (respect foreign keys)
    sqlx::query("DELETE FROM invoices WHERE tenant_id = $1")
        .bind(tenant_id.as_str())
        .execute(pool)
        .await?;
    
    sqlx::query("DELETE FROM users WHERE tenant_id = $1")
        .bind(tenant_id.as_str())
        .execute(pool)
        .await?;
    
    sqlx::query("DELETE FROM tenant_profiles WHERE tenant_id = $1")
        .bind(tenant_id.as_str())
        .execute(pool)
        .await?;
    
    info!(
        tenant_id = tenant_id.as_str(),
        "Tenant data deleted (GDPR right to be forgotten)"
    );
    
    Ok(())
}
```

### Example 3: Residency-Aware Data Replication

```rust
async fn replicate_tenant_data(
    ctx: &RequestContext,
    profile: &TenantProfile,
    data: &Data,
) -> RhelmaResult<()> {
    let region = ctx.region()
        .ok_or_else(|| RhelmaError::BadRequest("missing region".into()))?;
    
    // Validate residency BEFORE any replication
    profile.validate_residency(region)
        .rhelma_context("during residency validation")?;
    
    // Now safe to replicate
    match profile.residency {
        ResidencyPolicy::GlobalPreferred => {
            // Can replicate globally
            replicate_globally(data).await?;
        }
        ResidencyPolicy::RegionalPreferred => {
            // Replicate to primary and backup regions only
            replicate_to_regions(data, &[
                &profile.primary_region,
                &profile.backup_regions[..],
            ]).await?;
        }
        ResidencyPolicy::RegionalRequired => {
            // Stay in primary region ONLY
            replicate_to_region(data, &profile.primary_region).await?;
        }
    }
    
    Ok(())
}
```

---

**Last Updated:** December 6, 2025







