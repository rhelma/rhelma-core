# Rhelma Event-Driven Architecture v5.2

**Release:** January 2027  
**Status:** Final  
**Supersedes:** v5.1 Eventing & CQRS

---

## 1. Purpose

Rhelma eventing ensures:

1. **Loose coupling** between microservices
2. **Deterministic state** through event sourcing
3. **Regional autonomy** with global consistency
4. **Replayable history** for debugging and recovery
5. **Full auditability** for compliance
6. **At-least-once delivery** guarantee
7. **Exactly-once logical processing** via idempotency
8. **AI-native workflows** with intelligent event handling

---

## 2. Event Envelope v5.2 ⚠️ BREAKING CHANGE

Every event MUST conform to this canonical envelope:

```yaml
EventEnvelopeV5.2:
  # Event Identity
  event_id: uuidv7                 # Globally unique event ID
  event_version: int               # Schema version
  topic: string                    # Event topic (see taxonomy)
  key: string?                     # Optional partitioning key
  
  # Timestamps
  timestamp: RFC3339               # Event creation time
  published_at: RFC3339            # Publishing timestamp
  
  # Source
  source:
    service: string                # Originating service
    version: string                # Service version
    region: string                 # Source region
  
  # Request Context (propagated)
  request:
    request_id: uuidv7
    correlation_id: uuidv7
    tenant_id: string?
    user_id: string?
  
  # Distributed Tracing
  trace:
    trace_id: string               # W3C trace ID
    span_id: string                # Current span
    parent_span_id: string?        # Parent span
  
  # Payload
  payload: object                  # Event-specific data
  payload_type: string             # Payload schema identifier
  schema_ref: string               # Schema registry reference
  
  # Residency & Security
  residency: enum                  # ⚠️ NEW: GLOBAL | REGIONAL_ONLY | REGION_STRICT
  encryption: object?              # Encryption metadata (if applicable)
  
  # Integrity
  signature: string?               # ⚠️ NEW: ed25519 signature (required for audit events)
  hash: string?                    # Payload hash (sha256)
```

### Mandatory Behaviors

- `event_id` MUST be globally unique (UUIDv7 recommended)
- `event_version` MUST align with schema registry
- `residency` MUST be validated before publishing
- `trace_id` MUST propagate (no exceptions)
- `signature` MUST be present for `ops.audit` events
- Envelope MUST be immutable after creation

---

## 3. Event Topic Taxonomy

### Naming Convention

```
<domain>.<entity>.<action>[@version]

Examples:
- obs.heartbeat@v1
- obs.alert@v1
- ai.incident.proposed@v1
- ai.incident.decision@v1
- ai.command.execute@v1
- ops.audit@v2
- billing.invoice.created@v1
- vector.regenerated@v1
```

### Domain Prefixes

| Prefix | Domain | Examples |
|--------|--------|----------|
| `obs` | Observability | heartbeat, alert, insight |
| `ops` | Operations | audit, deployment, config |
| `ai` | AI/ML | incident, command, decision |
| `billing` | Billing | invoice, payment, usage |
| `tenant` | Tenancy | created, updated, deleted |
| `vector` | Vector DB | regenerated, indexed |
| `saga` | Transactions | started, completed, failed |
| `sec` | Security | violation, threat, alert |

---

## 4. Canonical Event Catalog

### 4.1 Observability Events

#### obs.heartbeat@v1

**Purpose**: Periodic health signal from services

**Producer**: All Rhelma services  
**Consumer**: Observability core, dashboards  
**Key**: `service`

```yaml
Payload:
  service: string                  # Service name
  region: string
  timestamp: RFC3339
  status: enum                     # healthy | degraded | down
  version: string                  # Service version
  uptime_seconds: int
  metadata: object?
```

#### obs.alert@v1

**Purpose**: Anomaly or critical condition detected

**Producer**: Observability-Agent  
**Consumer**: AI-Orchestrator, alerting systems  
**Key**: `service`

```yaml
Payload:
  service: string
  region: string
  detected_at: RFC3339
  kind: string                     # error_rate | latency | anomaly
  message: string
  severity: enum                   # info | warning | critical
  metrics: object                  # PII-free metrics snapshot
  threshold: float?
  actual_value: float?
```

#### obs.insight@v1

**Purpose**: Pattern discovered (non-critical)

**Producer**: Observability-Agent  
**Consumer**: AI-Orchestrator, analytics  
**Key**: `service`

```yaml
Payload:
  service: string
  region: string
  detected_at: RFC3339
  kind: string                     # trend | pattern | correlation
  message: string
  severity: enum                   # info | warning
  confidence: float                # 0.0 - 1.0
  metadata: object?
```

### 4.2 AI Incident Events (NEW in v5.2)

#### ai.incident.proposed@v1

**Purpose**: Agent proposes incident for AI analysis

**Producer**: Observability-Agent  
**Consumer**: AI-Orchestrator  
**Key**: `incident_id`

```yaml
Payload:
  incident_id: uuidv7              # Unique incident ID
  service: string
  service_version: string
  environment: string              # dev | staging | prod
  region: string
  detected_at: RFC3339
  
  kind: string                     # error_spike | latency_degradation | anomaly
  severity: enum                   # info | warning | critical
  message: string
  
  metrics: object                  # MUST be PII-free
  category: string?
  tags: [string]?
  confidence: float?               # Agent confidence 0.0-1.0
  
  dedupe_key: string?              # For deduplication
  candidates: [string]?            # Related incidents
  
  trace_id: string?
  span_id: string?
```

**Idempotency**: `incident_id` ensures single processing

#### ai.incident.decision@v1

**Purpose**: AI Orchestrator's analysis result

**Producer**: AI-Orchestrator  
**Consumer**: Observability-Agent  
**Key**: `incident_id`

```yaml
Payload:
  incident_id: string              # References proposed incident
  
  final_severity: enum             # info | warning | critical
  recommended_action: string?      # Action description
  reasoning: string                # MUST be sanitized
  confidence: float                # LLM confidence 0.0-1.0
  
  category: string?                # root_cause | symptom | correlation
  tags: [string]?
  
  generated_at: RFC3339
  model_used: string               # e.g., gpt-4, claude-3
  processing_time_ms: int
```

**Rules**:
- Orchestrator MUST sanitize `reasoning` (no PII)
- Agent MUST NOT modify past proposed incidents
- Decision guides future severity overrides

### 4.3 AI Command Events (NEW in v5.2)

#### ai.command.execute@v1

**Purpose**: AI-driven remediation command

**Producer**: AI-Orchestrator  
**Consumer**: Observability-Agent  
**Key**: `command_id`

```yaml
Payload:
  command_id: uuidv7               # Unique command ID
  incident_id: string?             # Optional incident linkage
  
  service: string                  # Target service
  region: string
  
  command: string                  # Command identifier
  args: object                     # Command arguments (PII-free)
  
  requested_by: string             # system | user ID
  requested_at: RFC3339
  timeout_ms: int                  # Execution timeout
```

**Allowed Commands**:
- `restart_service` ✅
- `scale_up` ⚠️ (not for STRICT tenants)
- `change_log_level` ✅
- `reduce_sampling` ✅
- `enable_degraded_mode` ✅
- `disable_degraded_mode` ✅

#### ai.command.result@v1

**Purpose**: Command execution result

**Producer**: Observability-Agent  
**Consumer**: AI-Orchestrator  
**Key**: `command_id`

```yaml
Payload:
  command_id: string
  incident_id: string?
  
  service: string
  region: string
  
  success: bool
  message: string                  # MUST be redacted
  output: object?                  # Sanitized output
  
  finished_at: RFC3339
  duration_ms: int
```

### 4.4 Operations Events

#### ops.audit@v2 ⚠️ BREAKING CHANGE

**Purpose**: Cryptographically secure audit trail

**Producer**: All services  
**Consumer**: Audit service, SIEM, compliance  
**Key**: `tenant_id` or `resource_id`

```yaml
Payload:
  # Core Fields
  event_id: uuidv7
  action: string                   # operation performed
  outcome: enum                    # success | failure
  
  # Actor & Resource
  actor: string                    # user ID or system
  resource_type: string            # service | tenant | config
  resource_id: string
  
  # Context
  tenant_id: string?
  region: string
  environment: string
  
  # Command Linkage (NEW)
  incident_id: string?
  command_id: string?
  
  # Timestamp
  timestamp: RFC3339
  
  # Integrity (NEW in v2)
  hash: string                     # sha256 of canonical payload
  chain_hash: string               # sha256(prev.chain_hash + hash)
  signature: string                # ed25519 signature
```

**Signing Requirements**:
- MUST use ed25519
- Keys MUST rotate every 90 days
- Hash computed after removing signature and chain_hash
- Chain break MUST emit `ops.audit.failure`

**Mandatory for**:
- All AI-triggered actions
- Configuration changes
- Access control modifications
- Data exports
- Tenant operations

### 4.5 Saga Events

#### saga.started@v1

```yaml
Payload:
  saga_id: uuidv7
  tenant_id: string
  saga_type: string                # workflow identifier
  steps_count: int
```

#### saga.step.completed@v1

```yaml
Payload:
  saga_id: uuidv7
  step_index: int
  step_name: string
  duration_ms: int
```

#### saga.compensation.started@v1

```yaml
Payload:
  saga_id: uuidv7
  step_index: int
  reason: string
```

See **06_DISTRIBUTED_TRANSACTIONS_v5.2.md** for complete saga events.

### 4.6 Vector Events

#### vector.regenerated@v1

```yaml
Payload:
  embedding_id: uuidv7
  tenant_id: string
  old_version: string
  new_version: string
  reason: string                   # model_upgrade | doc_changed
  regenerated_at: RFC3339
```

See **07_DATA_LAYER_v5.2.md** for complete vector events.

---

## 5. Event Ordering Model

### Ordering Guarantees

| Level | Guarantee | Implementation |
|-------|-----------|----------------|
| **Per-partition** | YES | Kafka partition key |
| **Per-tenant** | SHOULD | Key = tenant_id |
| **Per-key** | YES | Explicit key field |
| **Cross-tenant** | NO | Not required |
| **Cross-region** | NO | Not supported |

### Ordering Rules

1. Events with same `key` MUST preserve order
2. Events for same `tenant_id` SHOULD preserve order
3. Events across different keys have NO ordering guarantee
4. Ordering violations = **SEV-1 incident**

---

## 6. Schema Registry v5.2

### Registry Structure

```
schema://event/<topic>/<version>

Example:
schema://event/ai.incident.proposed/v1
```

### Supported Formats

- JSON Schema 2020-12 (recommended)
- Protobuf v3
- Avro

### Compatibility Rules

| Change | Allowed | Version Bump |
|--------|---------|--------------|
| Add optional field | ✅ | MINOR |
| Add required field | ❌ | MAJOR |
| Remove field | ❌ | MAJOR |
| Change type | ❌ | MAJOR |
| Rename field | ❌ | MAJOR |

### Registry API

```
GET  /schema/event/<topic>/versions
GET  /schema/event/<topic>/v<N>
POST /schema/event/<topic>/register
PUT  /schema/event/<topic>/v<N>/validate
```

---

## 7. Delivery Semantics

### Guarantees

- **At-least-once delivery**: Every event delivered ≥1 times
- **Exactly-once processing**: Via idempotency keys
- **Persistent storage**: Events survive broker restart
- **Ordered delivery**: Within same partition

### Infrastructure Requirements

- Kafka 3.0+ or NATS JetStream
- Minimum 3 broker nodes
- Replication factor ≥ 3
- Min ISR ≥ 2

---

## 8. Idempotency Model v5.2

All consumers MUST guarantee exactly-once logical processing.

### Strategy A: Processed Events Table

```sql
CREATE TABLE processed_events (
  event_id UUID PRIMARY KEY,
  processed_at TIMESTAMP,
  handler_version INT,
  outcome TEXT
);
```

**Logic**:
```
1. Check if event_id exists
2. If exists → skip processing
3. If not exists → process + insert
4. Use transaction to ensure atomicity
```

### Strategy B: State Machine Guard

Only valid transitions allowed. Duplicate events produce no state change.

**Example**: Order state machine
```
PENDING → PROCESSING → COMPLETED
         ↓
       FAILED
```

Re-applying `order.completed` has no effect.

### Strategy C: Upsert Semantics

For materialized views and idempotent operations.

```sql
INSERT INTO user_profile (user_id, name, email)
VALUES ($1, $2, $3)
ON CONFLICT (user_id)
DO UPDATE SET name = $2, email = $3;
```

---

## 9. Dead-Letter Queue (DLQ)

### DLQ Policy

After `N` failed delivery attempts, event MUST move to DLQ.

**Default**: N = 5 retries with exponential backoff

### DLQ Event Schema

```yaml
DLQEventV5.2:
  original_event: EventEnvelopeV5.2
  failed_attempts: int
  last_error: string
  failure_reason: string
  moved_to_dlq_at: RFC3339
  retry_after: RFC3339?           # When to retry
```

### DLQ Monitoring

- Alert on DLQ events (Slack/PagerDuty)
- Dashboard: DLQ depth per topic
- Manual replay process via API

---

## 10. Replay Model

### Replay Requirements

- MUST be idempotent
- MUST preserve original `event_id`
- MUST produce deterministic state
- MUST NOT re-trigger side effects (emails, payments, etc.)

### Replay Metadata

```yaml
ReplayMetadata:
  replay_id: uuidv7
  started_at: RFC3339
  ended_at: RFC3339?
  source_region: string
  from_offset: int
  to_offset: int
  tenant_id: string?
  reason: string
```

### Replay API

```
POST /events/replay
{
  "topic": "ai.incident.proposed",
  "from_offset": 1000,
  "to_offset": 2000,
  "tenant_id": "tenant-123"
}
```

---

## 11. CQRS Pattern

### Command Schema

```yaml
CommandV5.2:
  command_id: uuidv7
  timestamp: RFC3339
  tenant_id: string
  user_id: string?
  correlation_id: uuidv7
  type: string                     # CREATE_ORDER | UPDATE_USER
  payload: object
```

### Command Processing

```
Command → Validation → Business Logic → 0..N Events
```

Each command produces events representing state changes.

### Query Model

Read models are built from events:

```
Events → Projection → Read Model (Materialized View)
```

---

## 12. Regional Event Streams (NEW)

### Local Streams

- Strong ordering within region
- Low latency (< 50ms publish)
- Local SLAs apply

### Global Streams

- Eventual consistency across regions
- Cross-region replication for GLOBAL tenants
- Higher latency (< 500ms cross-region)

### Residency Enforcement

- `STRICT` tenants MUST NOT publish events outside region
- `PREFERRED` tenants MAY read from primary region
- Violations emit `residency.violation`

---

## 13. Large Payload Handling

### Size Limits

- **Maximum inline payload**: 1 MB
- **Over 1MB**: Store in L4 (object storage)

### External Payload Reference

```yaml
payload_ref:
  type: external
  storage: s3
  bucket: string
  key: string
  version: string
  size_bytes: int
  content_type: string
```

---

## 14. Observability Requirements

### Metrics (Prometheus)

```
eventbus_publish_total{topic, outcome}
eventbus_publish_duration_seconds{topic, outcome}
eventbus_consume_total{topic}
eventbus_consume_duration_seconds{topic}
eventbus_dlq_total{topic}
eventbus_replay_total{topic}
eventbus_lag_seconds{topic, consumer_group}
```

### Traces

- `event.publish`
- `event.consume`
- `event.replay`
- `event.dlq.move`

### Logs

```yaml
EventLogV5.2:
  event_id: string
  topic: string
  tenant_id: string?
  key: string?
  retry: int
  dlq: bool
  handler_version: int
  duration_ms: int
```

---

## 15. Security Requirements

### Access Control

- Per-topic ACLs
- Per-tenant isolation
- Mutual TLS for producers/consumers

### Encryption

- Events with PII MUST be encrypted
- Encryption metadata in envelope
- Keys from KMS

### Signature Verification

- Audit events (`ops.audit@v2`) MUST be signed
- Consumers MUST verify signatures
- Invalid signatures → reject + alert

---

## 16. SLA Requirements

| Metric | Target | Reference |
|--------|--------|-----------|
| Publish latency | < 50ms | A1_SLA_MATRIX |
| End-to-end p99 | < 5s | A1_SLA_MATRIX |
| DLQ rate | < 0.1% | A1_SLA_MATRIX |
| Replay throughput | > 10k events/sec | A1_SLA_MATRIX |
| Ordering violations | ZERO | A1_SLA_MATRIX |

---

## 17. Compliance Checklist

A service is **Event-Driven v5.2 Compliant** if:

✅ Uses EventEnvelope v5.2  
✅ Validates schema before publish  
✅ Implements idempotency  
✅ Handles DLQ  
✅ Supports replay  
✅ Preserves ordering guarantees  
✅ Enforces residency  
✅ Emits observability signals  
✅ Signs audit events  
✅ Meets SLA targets  

---

**End of Event-Driven Architecture v5.2**