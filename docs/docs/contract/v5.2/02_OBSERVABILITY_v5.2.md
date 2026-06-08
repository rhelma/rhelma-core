# Rhelma Observability v5.2

**Release:** January 2027  
**Status:** Final, Enterprise-Ready  
**Supersedes:** v5.1 Observability

---

## 1. Purpose

This document defines **logs**, **metrics**, **traces**, **audit**, **health**, and **AI observability** for all Rhelma services.

Observability v5.2 ensures:

- Complete request visibility
- Per-tenant monitoring
- Debugging of distributed systems
- AI/LLM safety, cost, and accountability
- Cross-region correlation
- Compliance and audit integrity
- Incident detection and analysis (NEW)

**Core Principle**: *If it is not observed, it does not exist.*

---

## 2. Observability Principles

1. **Every request MUST emit logs, metrics, and traces**
2. **Observability MUST NEVER leak PII or secrets**
3. **Audit events MUST be immutable and tamper-proof**
4. **AI calls require enhanced observability**
5. **All signals MUST link through RequestContext v5.2**
6. **Anomaly detection MUST be observable**
7. **Incidents MUST be traceable end-to-end**

---

## 3. Structured Logging (JSON)

### 3.1 Log Schema v5.2

```yaml
LogEventV5.2:
  # Core Fields
  timestamp: RFC3339              # ISO 8601 with milliseconds
  level: enum                     # DEBUG | INFO | WARN | ERROR | CRITICAL
  message: string                 # Human-readable message
  
  # Service Identity
  service:
    name: string
    version: string               # Semver
    instance_id: string           # Pod/container ID
    region: string
  
  # Request Context (propagated)
  request:
    request_id: uuidv7
    correlation_id: uuidv7
    tenant_id: string?
    user_id: string?
    region: string
    ip: string?
  
  # Distributed Tracing
  trace:
    trace_id: string              # W3C traceparent
    span_id: string
    parent_span_id: string?
  
  # Log-Specific
  context: object                 # Additional structured data
  error: object?                  # Error details (if level=ERROR)
    code: string
    message: string
    stack_trace: string?          # Only in debug mode
  
  # Security & Compliance
  pii_redacted: bool              # MUST be true if PII removed
  tags: [string]                  # Classification tags
  
  # Performance
  duration_ms: int?               # For operation logs
  
  # NEW in v5.2
  incident_id: string?            # Link to incident if applicable
  command_id: string?             # Link to AI command if applicable
```

### 3.2 Required Log Fields

| Field | Required | Notes |
|-------|----------|-------|
| `timestamp` | YES | RFC3339 with milliseconds |
| `level` | YES | DEBUG/INFO/WARN/ERROR/CRITICAL |
| `message` | YES | Human-readable description |
| `service.name` | YES | Service identifier |
| `request_id` | YES | From RequestContext v5.2 |
| `correlation_id` | YES | MUST propagate end-to-end |
| `tenant_id` | YES (if multi-tenant) | Tenant isolation |
| `trace_id` | YES | W3C standard |
| `pii_redacted` | YES | true/false flag |

### 3.3 Log Levels

| Level | Usage | Example |
|-------|-------|---------|
| **DEBUG** | Detailed debugging info | "Cache key lookup: user:123" |
| **INFO** | Normal operations | "Request completed successfully" |
| **WARN** | Warning conditions | "Retry attempt 2/3" |
| **ERROR** | Error conditions | "Database connection failed" |
| **CRITICAL** | System-critical issues | "Data residency violation detected" |

### 3.4 Example Log Entry

```json
{
  "timestamp": "2027-01-15T10:30:45.123Z",
  "level": "ERROR",
  "message": "AI safety check failed: PII detected in prompt",
  "service": {
    "name": "ai-orchestrator",
    "version": "5.2.1",
    "instance_id": "pod-abc-123",
    "region": "eu-central-1"
  },
  "request": {
    "request_id": "01HXYZ123ABC",
    "correlation_id": "01HXYZ000XYZ",
    "tenant_id": "tenant-456",
    "user_id": "user-789",
    "region": "eu-central-1",
    "ip": "10.0.1.50"
  },
  "trace": {
    "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
    "span_id": "00f067aa0ba902b7"
  },
  "context": {
    "safety_module": "pii_detector",
    "violations": ["email", "credit_card"],
    "risk_score": 85
  },
  "error": {
    "code": "AI_SAFETY_BLOCK",
    "message": "PII detected: email, credit_card"
  },
  "pii_redacted": true,
  "tags": ["ai", "safety", "pii"],
  "duration_ms": 120
}
```

---

## 4. PII Redaction Rules (MANDATORY)

### 4.1 Forbidden in Logs

The following MUST NEVER appear in logs:

- ❌ Passwords
- ❌ JWT tokens (full tokens)
- ❌ OAuth access tokens
- ❌ Refresh tokens
- ❌ Credit card numbers
- ❌ Government IDs (SSN, passport)
- ❌ Raw user prompts (unless policy allows)
- ❌ API secrets / keys
- ❌ Embedding vectors
- ❌ Private encryption keys

### 4.2 Allowed with Redaction

- ✅ Email (hashed): `sha256(email)` → `3f8a...bc2d`
- ✅ User ID (anonymized): `user-***789`
- ✅ IP address (last octet masked): `192.168.1.***`
- ✅ Token hash: First 8 chars of hash

### 4.3 PII Detection Trigger

If ANY log contains PII:
- MUST generate `sec.alert` event
- MUST alert security team
- MUST be treated as SEV-2 incident

---

## 5. Distributed Tracing (W3C Standard)

### 5.1 Required Headers

**HTTP**:
```
traceparent: 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01
tracestate: rhelma=tenant:tenant-123,region:eu-central-1
x-rhelma-request-id: 01HXYZ123ABC
x-rhelma-correlation-id: 01HXYZ000XYZ
x-tenant-id: tenant-123
x-region: eu-central-1
```

**gRPC Metadata**:
```
traceparent: <value>
tracestate: <value>
x-rhelma-request-id: <value>
x-rhelma-correlation-id: <value>
```

### 5.2 Span Schema v5.2

```yaml
SpanV5.2:
  # Identity
  name: string                    # Operation name
  trace_id: string                # W3C trace ID
  span_id: string                 # Current span ID
  parent_span_id: string?         # Parent span reference
  
  # Timing
  start_time: timestamp           # Nanosecond precision
  end_time: timestamp
  duration_ns: int                # Calculated duration
  
  # Attributes
  attributes:
    # Service
    service.name: string
    service.version: string
    service.instance.id: string
    
    # Request
    request.id: string
    correlation.id: string
    tenant.id: string?
    user.id: string?
    region: string
    
    # HTTP (if applicable)
    http.method: string
    http.route: string
    http.status_code: int
    http.url: string?
    
    # Database (if applicable)
    db.system: string             # postgresql | redis | qdrant
    db.operation: string          # SELECT | INSERT | SEARCH
    db.statement: string?         # Query (sanitized)
    
    # AI (if applicable)
    ai.model: string
    ai.provider: string
    ai.tokens.input: int
    ai.tokens.output: int
    ai.cost.usd: float
    
    # Vector (if applicable)
    vector.index: string
    vector.top_k: int
    vector.similarity: string
    
    # Incident (NEW v5.2)
    incident.id: string?
    command.id: string?
  
  # Status
  status:
    code: enum                    # OK | ERROR | UNSET
    message: string?
  
  # Events
  events: [SpanEvent]             # Timeline events within span
```

### 5.3 Required Spans

All services MUST generate spans for:

**HTTP Operations**:
- `http.server` - Incoming HTTP request
- `http.client` - Outgoing HTTP request

**Database Operations**:
- `db.query` - Database query
- `redis.operation` - Cache operation
- `cache.get` / `cache.set` - Cache operations

**Event Operations**:
- `event.publish` - Event publishing
- `event.consume` - Event consumption

**AI Operations**:
- `ai.router.decision` - Router v3 decision
- `ai.prompt.load` - Prompt registry lookup
- `ai.rag.retrieval` - RAG retrieval
- `ai.embedding.generate` - Embedding generation
- `ai.llm.call` - LLM API call
- `ai.safety.check` - Safety validation
- `ai.incident.analyze` (NEW) - Incident analysis
- `ai.incident.decision` (NEW) - Decision generation

**Vector Operations**:
- `vector.search` - Vector similarity search
- `vector.embed` - Embedding creation
- `vector.ingest` - Batch ingestion
- `vector.regenerate` - Embedding regeneration

**Saga Operations**:
- `saga.orchestration` - Saga coordination
- `saga.step` - Individual saga step
- `saga.compensation` - Compensation action

### 5.4 Span Example

```json
{
  "name": "ai.incident.analyze",
  "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
  "span_id": "00f067aa0ba902b7",
  "parent_span_id": "a3ce929d0e0e4736",
  "start_time": "2027-01-15T10:30:45.000000000Z",
  "end_time": "2027-01-15T10:30:55.000000000Z",
  "duration_ns": 10000000000,
  "attributes": {
    "service.name": "ai-orchestrator",
    "service.version": "5.2.1",
    "request.id": "01HXYZ123ABC",
    "correlation.id": "01HXYZ000XYZ",
    "tenant.id": "tenant-123",
    "region": "eu-central-1",
    "incident.id": "inc_01HXYZ456",
    "ai.model": "gpt-4o",
    "ai.provider": "openai",
    "ai.tokens.input": 1500,
    "ai.tokens.output": 250,
    "ai.cost.usd": 0.045
  },
  "status": {
    "code": "OK"
  },
  "events": [
    {
      "name": "rag.context.built",
      "timestamp": "2027-01-15T10:30:46.500000000Z",
      "attributes": {
        "documents.retrieved": 5,
        "context.tokens": 800
      }
    }
  ]
}
```

---

## 6. Metrics (Prometheus Compatible)

### 6.1 Metric Naming Convention

```
<namespace>_<subsystem>_<name>_<unit>

Examples:
- http_request_duration_seconds
- db_query_total
- ai_tokens_input_total
```

### 6.2 Required Labels

All metrics MUST include:
- `service` - Service name
- `region` - Region identifier
- `environment` - dev/staging/prod

Multi-tenant metrics MUST include:
- `tenant_id` - Tenant identifier

### 6.3 Core Metrics

#### HTTP Metrics

```
# Histogram: Request duration
http_request_duration_seconds{method, route, status, tenant_id}

# Counter: Total requests
http_request_total{method, route, status, tenant_id}

# Counter: Request errors
http_request_errors_total{method, route, error_code}

# Gauge: Active connections
http_active_connections{service}
```

#### Database Metrics

```
# Histogram: Query duration
db_query_duration_seconds{operation, outcome, tenant_id}

# Counter: Total queries
db_query_total{operation, outcome}

# Counter: Connection errors
db_connection_errors_total

# Gauge: Active connections
db_connections_active{pool}

# Gauge: Connection pool size
db_connections_pool_size{pool}
```

#### Cache Metrics

```
# Counter: Cache hits
cache_hit_total{backend, key_space, tenant_id}

# Counter: Cache misses
cache_miss_total{backend, key_space, tenant_id}

# Counter: Cache errors
cache_error_total{backend, error_type}

# Histogram: Cache operation duration
cache_op_duration_seconds{backend, operation}

# Gauge: Cache size
cache_size_bytes{backend}
```

#### Event Bus Metrics

```
# Counter: Events published
eventbus_publish_total{topic, outcome}

# Histogram: Publish duration
eventbus_publish_duration_seconds{topic, outcome}

# Counter: Publish errors
eventbus_publish_error_total{topic, error_type}

# Counter: Events consumed
eventbus_consume_total{topic, consumer_group}

# Histogram: Consume duration
eventbus_consume_duration_seconds{topic}

# Gauge: Consumer lag
eventbus_consumer_lag_seconds{topic, consumer_group}

# Counter: DLQ events
eventbus_dlq_total{topic}
```

#### AI/LLM Metrics (Enhanced v5.2)

```
# Counter: Input tokens
ai_tokens_input_total{model, provider, tenant_id}

# Counter: Output tokens
ai_tokens_output_total{model, provider, tenant_id}

# Counter: Embedding tokens
ai_tokens_embedding_total{model, provider, tenant_id}

# Counter: Request cost
ai_request_cost_total{model, provider, tenant_id}

# Histogram: Request duration
ai_request_duration_seconds{model, provider, tenant_id}

# Counter: Total requests
ai_request_total{model, provider, status}

# Counter: Fallback attempts
ai_fallback_total{reason}

# Counter: Safety blocks
ai_safety_block_total{category, severity}

# Counter: Tool calls
ai_tool_call_total{tool_name, model}

# NEW in v5.2: Incident metrics
ai_incident_analyzed_total{outcome}
ai_incident_analysis_duration_seconds
ai_incident_confidence_score{bucket}
ai_incident_auto_remediated_total
ai_incident_escalated_total
ai_incident_decision_errors_total
```

#### Vector DB Metrics

```
# Histogram: Lookup duration
vector_lookup_duration_seconds{index, tenant_id}

# Counter: Total lookups
vector_lookup_total{index, tenant_id, outcome}

# Counter: Embedding regeneration
vector_regeneration_total{reason}

# Counter: Vector inserts
vector_insert_total{index}

# Histogram: Index compaction
vector_compaction_duration_seconds{index}

# Gauge: Index size
vector_index_size_bytes{index}

# Gauge: Vector count
vector_count_total{index, tenant_id}
```

#### Saga Metrics

```
# Counter: Saga executions
saga_execution_total{outcome}

# Counter: Failed sagas
saga_failed_total{reason}

# Counter: Compensations
saga_compensation_total{outcome}

# Histogram: Step duration
saga_step_duration_seconds{step_name}

# Counter: Retries
saga_retry_total{step_name}

# Counter: Deadlocks
saga_deadlock_total
```

### 6.4 Custom Application Metrics

Services MAY define custom metrics following conventions:

```
<service>_<feature>_<metric>_<unit>{labels}

Example:
order_processing_items_total{status, tenant_id}
payment_transaction_amount_usd{method, currency}
```

---

## 7. Health & Heartbeat

### 7.1 Health Endpoint

**GET** `/health`

**Response Schema**:

```yaml
HealthV5.2:
  status: enum                    # healthy | degraded | down
  timestamp: RFC3339
  version: string                 # Service version
  environment: string             # dev/staging/prod
  uptime_seconds: int
  
  components:
    database:
      status: enum                # healthy | degraded | down
      latency_ms: int
      message: string?
    
    cache:
      status: enum
      hit_rate: float
      message: string?
    
    eventing:
      status: enum
      lag_seconds: int
      message: string?
    
    vector:
      status: enum
      latency_ms: int
      message: string?
    
    ai_provider:
      status: enum
      available_models: [string]
      message: string?
  
  # NEW in v5.2
  incidents:
    active_count: int
    critical_count: int
```

**Example Response**:

```json
{
  "status": "healthy",
  "timestamp": "2027-01-15T10:30:45Z",
  "version": "5.2.1",
  "environment": "production",
  "uptime_seconds": 86400,
  "components": {
    "database": {
      "status": "healthy",
      "latency_ms": 15
    },
    "cache": {
      "status": "healthy",
      "hit_rate": 0.87
    },
    "eventing": {
      "status": "healthy",
      "lag_seconds": 2
    },
    "vector": {
      "status": "healthy",
      "latency_ms": 45
    },
    "ai_provider": {
      "status": "healthy",
      "available_models": ["gpt-4o", "claude-sonnet-4"]
    }
  },
  "incidents": {
    "active_count": 0,
    "critical_count": 0
  }
}
```

### 7.2 Heartbeat Event

**Topic**: `obs.heartbeat@v1`  
**Frequency**: Every 30 seconds  
**Producer**: All Rhelma services

```yaml
HeartbeatV5.2:
  service: string
  region: string
  status: enum                    # healthy | degraded | down
  timestamp: RFC3339
  version: string
  uptime_seconds: int
  metadata:
    cpu_usage_percent: float
    memory_usage_percent: float
    active_connections: int
    request_rate_per_second: float
```

---

## 8. Audit Trail v5.2

### 8.1 Audit Event Schema

See **04_EVENT_DRIVEN_v5.2.md** for complete `ops.audit@v2` schema.

**Key Requirements**:
- ✅ ed25519 signatures
- ✅ Merkle chain for tamper-proofing
- ✅ Append-only storage
- ✅ Cryptographic verification endpoint

### 8.2 Required Audit Events

All services MUST emit audit events for:

- Authentication (login, logout, token refresh)
- Authorization (permission grants, role changes)
- Data access (sensitive resource access)
- Configuration changes
- AI command execution (NEW)
- Incident decisions (NEW)
- Data exports
- Tenant operations (create, update, delete)
- Security violations

### 8.3 Audit Verification

**Endpoint**: `GET /audit/verify/{event_id}`

**Response**:

```yaml
AuditVerificationV5.2:
  event_id: string
  valid: bool
  chain_valid: bool
  signature_valid: bool
  timestamp: RFC3339
  verified_at: RFC3339
  errors: [string]?
```

---

## 9. AI Observability Extensions (v5.2)

### 9.1 AI Trace Spans (Complete List)

```
ai.router.decision          - Model routing decision
ai.prompt.load              - Prompt registry lookup
ai.rag.chunk                - Document chunking
ai.rag.embed                - Embedding generation
ai.rag.retrieve             - Vector retrieval
ai.rag.rank                 - Result re-ranking
ai.rag.synthesize           - Response synthesis
ai.llm.call                 - LLM API invocation
ai.safety.input             - Input safety check
ai.safety.output            - Output safety check
ai.tool.call                - Function/tool invocation
ai.incident.consume         - Incident event consumption (NEW)
ai.incident.analyze         - LLM-based analysis (NEW)
ai.incident.decision        - Decision generation (NEW)
ai.command.publish          - Command publication (NEW)
```

### 9.2 AI Log Schema

```yaml
AITraceLogV5.2:
  request_id: uuidv7
  model: string
  provider: string
  routing_decision: string
  tokens_in: int
  tokens_out: int
  tokens_embedding: int
  cost_usd: float
  fallback: bool
  safety_status: enum           # ALLOWED | BLOCKED | MODIFIED
  prompt_version: string
  latency_ms: int
  
  # NEW in v5.2
  incident_id: string?
  command_id: string?
  confidence: float?
  reasoning_sanitized: bool?
```

### 9.3 Safety Violation Logs

```yaml
SafetyViolationLog:
  request_id: uuidv7
  tenant_id: string
  violation_type: enum          # PII | TOXICITY | INJECTION | HALLUCINATION
  severity: enum                # LOW | MEDIUM | HIGH | CRITICAL
  details: string               # Sanitized description
  action_taken: enum            # BLOCKED | MODIFIED | ALLOWED_WITH_WARNING
  timestamp: RFC3339
```

---

## 10. Sampling Rules v5.2

### 10.1 Global Sampling

| Level | Sampling Rate | Notes |
|-------|--------------|-------|
| ERROR/CRITICAL | 1.0 (100%) | Never sample errors |
| WARN | 1.0 (100%) | Always log warnings |
| INFO | 1.0 (100%) | Default: full logging |
| DEBUG | 0.1 (10%) | Sample unless debug mode |

### 10.2 Tenant-Aware Sampling

| Tier | Sampling Rate |
|------|---------------|
| Enterprise | 1.0 (100%) |
| Pro | 1.0 (100%) |
| Free | 0.5 (50%) |

### 10.3 Trace Sampling

```yaml
TraceSampling:
  default_rate: 0.1             # 10% of traces
  error_rate: 1.0               # 100% of error traces
  slow_request_rate: 1.0        # 100% of slow requests (>1s)
  ai_request_rate: 0.5          # 50% of AI requests
  incident_rate: 1.0            # 100% of incident traces (NEW)
```

---

## 11. Log Shipping & Latency

### 11.1 Latency Requirements

| Level | Max Latency |
|-------|-------------|
| CRITICAL | 500ms |
| ERROR | 500ms |
| WARN | 1s |
| INFO | 1s |
| DEBUG | 5s |

### 11.2 Shipping Infrastructure

**Supported Backends**:
- Elasticsearch
- Splunk
- DataDog
- CloudWatch Logs
- Google Cloud Logging
- Azure Monitor

**Protocol**: OTLP (OpenTelemetry Protocol)

---

## 12. Log Volume Quotas

### 12.1 Per-Tenant Quotas

| Tier | Daily Quota | Overage Action |
|------|-------------|----------------|
| Enterprise | 10 GB | Soft limit, alert |
| Pro | 1 GB | Hard limit at 1.2x |
| Free | 100 MB | Hard limit |

### 12.2 Quota Exceeded Event

**Topic**: `obs.log.quota_exceeded`

```yaml
Payload:
  tenant_id: string
  current_usage_mb: int
  quota_mb: int
  timestamp: RFC3339
```

---

## 13. Observability Stack Components

### 13.1 Required Components

```
┌─────────────────────────────────────────┐
│           Application Layer              │
│  (Services with OTEL instrumentation)   │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│        OTEL Collector (Agent)           │
│  - Receives telemetry                   │
│  - Processes & enriches                 │
│  - Routes to backends                   │
└──────────────┬──────────────────────────┘
               │
      ┌────────┼────────┐
      ▼        ▼        ▼
┌──────────┬──────────┬──────────────┐
│  Traces  │  Metrics │    Logs      │
│ (Jaeger/ │(Prometheus│(Elasticsearch│
│  Tempo)  │  /Mimir) │   /Loki)     │
└──────────┴──────────┴──────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│         Observability UI                 │
│  - Grafana                               │
│  - Kibana                                │
│  - Custom dashboards                     │
└─────────────────────────────────────────┘
```

### 13.2 OTEL Configuration

```yaml
OTELConfig:
  exporters:
    otlp:
      endpoint: https://collector.rhelma.internal:4317
      protocol: grpc
      compression: gzip
    
    prometheus:
      port: 9090
      path: /metrics
    
    logging:
      level: info
      format: json
  
  processors:
    batch:
      timeout: 1s
      send_batch_size: 1024
    
    resource:
      attributes:
        service.name: ${SERVICE_NAME}
        service.version: ${SERVICE_VERSION}
        deployment.environment: ${ENVIRONMENT}
```

---

## 14. Alerting Rules

### 14.1 Critical Alerts

```yaml
# High error rate
- alert: HighErrorRate
  expr: rate(http_request_errors_total[5m]) > 0.05
  severity: critical
  annotations:
    summary: "Error rate exceeds 5%"

# AI safety violations
- alert: AISafetyViolations
  expr: increase(ai_safety_block_total[5m]) > 10
  severity: high
  annotations:
    summary: "Multiple AI safety violations detected"

# Database connection failures
- alert: DatabaseConnectionFailure
  expr: db_connections_active == 0
  severity: critical
  annotations:
    summary: "No active database connections"

# NEW: Incident analysis failures
- alert: IncidentAnalysisFailures
  expr: rate(ai_incident_decision_errors_total[5m]) > 0.1
  severity: high
  annotations:
    summary: "Incident analysis failing"
```

---

## 15. Compliance Requirements

A service is **Observability v5.2 Compliant** if:

✅ 100% logs follow JSON schema  
✅ 100% requests have trace IDs  
✅ 100% metrics exported  
✅ Audit trail signed (ops.audit@v2)  
✅ Heartbeats active (30s interval)  
✅ AI observability implemented  
✅ PII redaction enforced  
✅ Health endpoint responds  
✅ Incident metrics tracked (NEW)  
✅ Meets log shipping latency requirements  
✅ HTTP Contract v5.2 enforced at ingress (x-rhelma-request-id, x-rhelma-correlation-id, x-residency, traceparent) and guarded by automated conformance tests (Phase 80)  

---

**End of Observability v5.2**