# Rhelma Architecture Core v5.2

**Release:** January 2027  
**Status:** Final / Stable  
**Supersedes:** v5.1 Main Architecture

---

## 1. Architectural Principles

Rhelma is a **multi-region**, **multi-tenant**, **AI-native**, **zero-trust**, **event-driven**, **observability-first** cloud architecture.

### Core Principles

1. **Stateless compute** - No local state dependencies
2. **Event-driven workflows** - Async, loosely-coupled
3. **Zero-Trust at every layer** - Never trust, always verify
4. **AI-first orchestration** - Intelligence embedded in platform
5. **Deterministic observability** - Complete system visibility
6. **Multi-region active/active** - No single point of failure
7. **Storage L1–L4 consistency** - Layered data strategy
8. **Strong tenant isolation** - Security and privacy by design
9. **Formal SLAs & DR** - Predictable reliability
10. **Schema-governed evolution** - Safe, versioned changes

---

## 2. RequestContext v5.2 ⚠️ BREAKING CHANGE

All requests MUST propagate a canonical context object.

```yaml
RequestContextV5.2:
  # Identity
  request_id: uuidv7              # Unique per request
  correlation_id: uuidv7           # Groups related requests
  trace_id: string                 # W3C trace format
  span_id: string                  # Current span
  parent_span_id: string?          # Parent span reference
  
  # Tenancy & User
  tenant_id: string?               # Multi-tenant identifier
  user_id: string?                 # End user identifier
  
  # Location & Residency
  region: string                   # AWS region, GCP zone, etc.
  residency: enum                  # GLOBAL | REGIONAL_PREFERRED | REGIONAL_STRICT
  ip: string                       # Client IP (for audit)
  device: string?                  # Device fingerprint
  
  # Authentication & Authorization
  auth_context:
    roles: [string]                # RBAC roles
    permissions: [string]          # PBAC permissions
    session_id: string?            # Session tracking
    token_hash: string?            # JWT hash (never full token)
  
  # Metadata
  timestamp: RFC3339               # Request creation time
  service:
    name: string                   # Originating service
    version: string                # Service version (semver)
  
  # Flags (NEW in v5.2)
  flags:
    read_only: bool                # Read-only mode
    dry_run: bool                  # Simulation mode
    ai_safe_mode: bool             # ⚠️ NEW: Enhanced AI safety
    debug_mode: bool               # Verbose logging
```

### Mandatory Behaviors

- MUST be **immutable** once created
- MUST propagate across:
  - HTTP headers
  - gRPC metadata
  - Async task queues
  - Event envelopes
  - All logs and metrics
- MUST validate on entry points
- MUST redact sensitive fields in logs

### Required HTTP Headers

```
x-rhelma-request-id: <uuidv7>
x-rhelma-correlation-id: <uuidv7>
traceparent: <W3C format>
tracestate: <W3C format>
x-tenant-id: <tenant identifier>
x-region: <region code>
x-residency: <GLOBAL|REGIONAL_PREFERRED|REGIONAL_STRICT>
```

### gRPC Metadata Keys

```
x-rhelma-request-id
x-rhelma-correlation-id
x-tenant-id
x-region
```

---

## 3. Error Model v5.2

Every service MUST return structured errors following this schema:

```yaml
ErrorV5.2:
  # Core Fields
  error_code: string               # UPPER_SNAKE_CASE
  http_status: int                 # Standard HTTP status
  message: string                  # Human-readable description
  
  # Retry & Severity
  retryable: bool                  # Can client retry?
  severity: enum                   # LOW | MEDIUM | HIGH | CRITICAL
  retry_after_ms: int?             # Suggested retry delay
  
  # Context
  context: object                  # Additional error details
  request_id: uuidv7               # Request identifier
  correlation_id: uuidv7           # Correlation identifier
  
  # Timestamps
  timestamp: RFC3339               # Error occurrence time
  
  # Stack (optional, debug mode only)
  stack_trace: string?             # Never in production logs
```

### Required Error Categories

| Error Code | HTTP Status | Retryable | Description |
|------------|-------------|-----------|-------------|
| `VALIDATION_ERROR` | 400 | false | Invalid input |
| `NOT_FOUND` | 404 | false | Resource not found |
| `UNAUTHORIZED` | 401 | false | Authentication failed |
| `FORBIDDEN` | 403 | false | Authorization failed |
| `RATE_LIMIT` | 429 | true | Rate limit exceeded |
| `RESIDENCY_VIOLATION` | 451 | false | Data residency breach |
| `AI_COST_EXCEEDED` | 429 | false | AI budget exceeded |
| `AI_SAFETY_BLOCK` | 403 | false | AI safety filter triggered |
| `EVENT_REPLAY` | 409 | false | Event duplicate detected |
| `SAGA_COMPENSATION_FAILED` | 500 | false | Transaction rollback failed |
| `TIMEOUT` | 504 | true | Request timeout |
| `SERVICE_UNAVAILABLE` | 503 | true | Temporary unavailability |
| `INTERNAL_ERROR` | 500 | false | Unexpected error |

### Error Response Format

```json
{
  "error": {
    "error_code": "AI_COST_EXCEEDED",
    "http_status": 429,
    "message": "Monthly AI budget of $1000 exceeded",
    "retryable": false,
    "severity": "HIGH",
    "context": {
      "tenant_id": "tenant-123",
      "current_spend": 1050.23,
      "limit": 1000.00
    },
    "request_id": "01HXYZ123ABC",
    "correlation_id": "01HXYZ000XYZ",
    "timestamp": "2027-01-15T10:30:00Z"
  }
}
```

---

## 4. Tenancy Model v5.2

### Tenant Metadata Schema

```yaml
TenantV5.2:
  # Identity
  id: string                       # Unique tenant identifier
  name: string                     # Display name
  
  # Location & Residency
  region_primary: string           # Primary region
  regions_allowed: [string]        # Allowed regions list
  residency_policy: enum           # GLOBAL | REGIONAL_PREFERRED | REGIONAL_STRICT
  
  # Features & Limits
  tier: enum                       # FREE | PRO | ENTERPRISE
  features: map<string, bool>      # Feature flags
  rate_limits: object              # Rate limiting config
  ai_budget_usd: float             # Monthly AI spending cap
  
  # Quotas
  quotas:
    storage_gb: int
    api_calls_per_day: int
    ai_tokens_per_month: int
    vector_embeddings: int
  
  # Metadata
  created_at: RFC3339
  updated_at: RFC3339
  status: enum                     # ACTIVE | SUSPENDED | DELETED
```

### Tenant Isolation Guarantees

1. **Data isolation**: All data MUST include `tenant_id`
2. **Compute isolation**: Resource limits enforced per tenant
3. **Network isolation**: Optional VPC/VNet per tenant
4. **Storage isolation**: 
   - L1/L2 cache: Namespaced by tenant
   - L3 DB: Tenant column indexed
   - L4 object storage: Bucket prefixes
   - Vector DB: Separate indexes per tenant
5. **Event isolation**: Kafka partitions by tenant_id
6. **Logging isolation**: All logs tagged with tenant_id

### Residency Enforcement Rules

| Policy | Description | Cross-Region Replication |
|--------|-------------|--------------------------|
| **GLOBAL** | Data can flow anywhere | ✅ Allowed |
| **REGIONAL_PREFERRED** | Prefer local, allow fallback | ⚠️ With warning |
| **REGIONAL_STRICT** | Never leave region | ❌ Forbidden |

**Violation handling**:
- Return HTTP 451 `RESIDENCY_VIOLATION`
- Emit security event `residency.violation`
- Log CRITICAL entry with full context

---

## 5. Storage Architecture: L1–L4

Rhelma uses a **four-layer storage hierarchy** for optimal performance and cost.

### Layer Overview

| Layer | Technology | Purpose | Latency | Durability |
|-------|-----------|---------|---------|-----------|
| **L1** | In-memory cache (local) | Ultra-fast node cache | < 1ms | Ephemeral |
| **L2** | Redis/KeyDB (distributed) | Shared cache layer | < 10ms | Replicated |
| **L3** | DB/Vector/Graph | Primary data storage | < 100ms | Persistent |
| **L4** | Object storage (S3/GCS/Azure) | Archival & large objects | < 500ms | 11 nines |

### L1: In-Memory Cache

**Requirements**:
- MUST be ephemeral (no persistence)
- TTL MUST be < 60 seconds
- MUST NOT store tenant-critical data
- Size limit: 100MB per service instance

**Use cases**: Hot keys, session data, computed values

### L2: Distributed Cache

**Requirements**:
- MUST replicate cross-zone
- MAY replicate cross-region (GLOBAL tenants only)
- MUST encrypt in transit (TLS 1.3)
- MUST support tenant namespacing

**Technologies**: Redis Cluster, KeyDB, Memcached

**Use cases**: Session storage, rate limiting, feature flags

### L3: Primary Storage

**Components**:
- **Relational DB**: PostgreSQL, MySQL, Aurora
- **NoSQL DB**: DynamoDB, Cosmos DB, Cassandra
- **Vector DB**: Qdrant, Weaviate, Milvus (see Document 07)
- **Graph DB**: Neo4j, Memgraph (optional)

**Requirements**:
- MUST enforce residency policies
- MUST support ACID transactions (relational)
- MUST provide WAL for durability
- MUST encrypt at rest (AES-256)
- MUST support point-in-time recovery

### L4: Object Storage

**Requirements**:
- MUST encrypt at rest (AES-256)
- MUST version large objects
- MUST support lifecycle policies
- MUST integrate with DR strategy

**Use cases**: Backups, logs, embeddings archives, media files

---

## 6. Configuration Management v5.2

### Configuration Hierarchy

```
1. Tenant-level overrides (highest priority)
2. Environment-level overrides
3. Region-level defaults
4. Service defaults (lowest priority)
```

### Configuration Schema

```yaml
ConfigBundleV5.2:
  # Identity
  bundle_id: string
  version: semver                  # e.g., 1.2.3
  
  # Scope
  service: string
  environment: string              # dev | staging | prod
  region: string?
  tenant_id: string?
  
  # Configuration
  config: object                   # Actual config data
  schema_ref: string               # JSON Schema reference
  
  # Metadata
  created_at: RFC3339
  created_by: string
  signature: string                # Config integrity signature
```

### Dynamic Configuration Features

- ✅ Hot reload (no restart required)
- ✅ Override precedence
- ✅ Per-tenant customization
- ✅ Versioned bundles
- ✅ Schema validation (JSON Schema)
- ✅ Rollback support
- ✅ Audit trail for changes

### Config Source Backends

- AWS Systems Manager Parameter Store
- HashiCorp Consul
- etcd
- Google Cloud Secret Manager
- Azure App Configuration

---

## 7. Versioning & Schema Governance

### Semantic Versioning

All schemas, APIs, and services MUST follow **semver**:

```
MAJOR.MINOR.PATCH

MAJOR: Breaking changes (incompatible API changes)
MINOR: New features (backward-compatible additions)
PATCH: Bug fixes (backward-compatible fixes)
```

### Schema Registry Requirements

Every event, API contract, and data model MUST:

1. Be registered in schema registry
2. Have explicit version
3. Pass compatibility checks
4. Include changelog

### Backward Compatibility Rules

| Change Type | Allowed | Version Bump |
|-------------|---------|--------------|
| Add optional field | ✅ Yes | MINOR |
| Add required field | ❌ No | MAJOR |
| Remove field | ❌ No | MAJOR |
| Change field type | ❌ No | MAJOR |
| Rename field | ❌ No | MAJOR |
| Change enum values | ⚠️ Depends | MAJOR/MINOR |

### Deprecation Policy

**Announcement requirements**:
1. Changelog entry
2. Event `contract.deprecation` emitted
3. API documentation updated
4. Email to stakeholders

**Removal timeline**:
- Deprecated features MUST remain for 1 MINOR release
- Minimum 30 days notification
- Migration guide provided

---

## 8. Performance & SLA Requirements

### Core Performance Targets

| Metric | Target | Reference |
|--------|--------|-----------|
| API p95 latency | < 250ms | A1_SLA_MATRIX |
| API p99 latency | < 500ms | A1_SLA_MATRIX |
| DB query p99 | < 100ms | A1_SLA_MATRIX |
| Cache hit rate | > 85% | A1_SLA_MATRIX |
| Event delivery p99 | < 5s | A1_SLA_MATRIX |
| AI LLM p99 | < 2500ms | A1_SLA_MATRIX |
| Vector search p99 | < 100ms | A1_SLA_MATRIX |

See **A1_SLA_MATRIX_v5.2.md** for complete specifications.

---

## 9. Compliance Requirements

A service is **Architecture Core v5.2 Compliant** if:

✅ Implements RequestContext v5.2  
✅ Uses structured error model v5.2  
✅ Enforces tenant isolation  
✅ Respects residency policies  
✅ Uses L1-L4 storage correctly  
✅ Supports dynamic configuration  
✅ Follows semantic versioning  
✅ Registers all schemas  
✅ Meets SLA targets  
✅ Passes security audit  

---

## 10. Migration from v5.1

### Breaking Changes

1. **RequestContext**: Add `flags.ai_safe_mode` field
2. **Error Model**: Add `retry_after_ms` field
3. **Tenant Model**: Add `tier` and `ai_budget_usd`

### Migration Steps

1. Update RequestContext propagation logic
2. Add new error fields to error handlers
3. Update tenant metadata schemas
4. Test residency enforcement
5. Validate SLA monitoring

**Deadline**: Q3 2027

---

**End of Architecture Core v5.2**