# Contract v6.0 — Distributed transactions

**Release:** January 2027  
**Status:** Final  
**Supersedes:** v5.1 Saga Pattern

---

## 1. Purpose

Saga v6.0 ensures:

- ✅ Integrity of multi-step workflows
- ✅ Idempotent, retryable execution
- ✅ Deterministic compensation
- ✅ No partial failures
- ✅ Strong observability & auditability
- ✅ Safe integration (DB, AI, Vector, Events)
- ✅ AI-integrated workflows (Enhanced v6.0)

**Rhelma Forbids**:
- ❌ Distributed 2PC (Two-Phase Commit)
- ❌ Blocking transactions
- ❌ Cross-service ACID transactions

**Saga is the ONLY permitted model.**

---

## 2. Saga Model v6.0

### 2.1 Core Components

```
Saga = Sequence of Steps + Compensations

Steps (Forward):     S1 → S2 → S3 → S4
Compensations:       C1 ← C2 ← C3 ← C4
```

A saga consists of:
- **Steps**: Forward operations
- **Compensations**: Undo operations (executed in reverse)
- **State machine**: Tracks execution status
- **Retries**: Automatic retry with backoff
- **Timeouts**: Step and saga-level timeouts
- **Locks**: Distributed locking for resource coordination
- **Audit**: Complete execution trail
- **Replay**: Deterministic replay capability

### 2.2 Saga Object Schema

```yaml
SagaV5.2:
  # Identity
  saga_id: uuidv7
  saga_type: string               # Workflow identifier
  
  # Context
  tenant_id: string
  user_id: string?
  correlation_id: uuidv7
  trace_id: string
  
  # Status
  status: enum                    # PENDING | RUNNING | COMPLETED | FAILED | 
                                  # COMPENSATING | COMPENSATED | CANCELLED
  
  # Steps Definition
  steps: [SagaStepV5.2]
  compensations: [SagaStepV5.2]
  current_step: int               # Current execution index
  
  # Configuration
  retry_policy: RetryPolicyV5.2
  timeout_ms: int                 # Total saga timeout
  
  # Timestamps
  created_at: RFC3339
  started_at: RFC3339?
  updated_at: RFC3339
  completed_at: RFC3339?
  
  # Execution State
  execution_state: object         # Saga-specific data
  metadata: object                # Additional context
  
  # NEW in v6.0
  ai_integrated: bool             # Contains AI operations
  incident_id: string?            # Linked incident (if auto-remediation)
```

### 2.3 Step Schema

```yaml
SagaStepV5.2:
  # Identity
  name: string                    # Step identifier
  index: int                      # Step order
  
  # Actions
  action: object
    type: enum                    # URL | internal | lambda
    handler: string               # Handler identifier
    payload: object               # Input data
  
  compensation: object
    type: enum
    handler: string
    payload: object
  
  # Configuration
  timeout_ms: int
  max_retries: int
  retry_backoff_ms: int
  
  # Status
  status: enum                    # PENDING | RUNNING | COMPLETED | FAILED | 
                                  # COMPENSATING | COMPENSATED | SKIPPED
  attempts: int
  last_error: string?
  
  # Flags
  is_terminal: bool               # Cannot be compensated
  is_idempotent: bool
  requires_lock: bool
  
  # Timestamps
  started_at: RFC3339?
  completed_at: RFC3339?
```

---

## 3. State Machine

### 3.1 State Transitions

```
          ┌──────────┐
          │ PENDING  │
          └────┬─────┘
               │ start
               ▼
          ┌──────────┐
          │ RUNNING  │
          └────┬─────┘
               │
       ┌───────┼───────┐
       │ success       │ failure/timeout
       ▼               ▼
  ┌──────────┐   ┌──────────────┐
  │COMPLETED │   │ COMPENSATING │
  └──────────┘   └──────┬───────┘
                        │
                 ┌──────┴──────┐
          success│             │failure
                 ▼             ▼
          ┌────────────┐  ┌────────┐
          │COMPENSATED │  │ FAILED │
          └────────────┘  └────────┘
          
          (CANCELLED can occur from PENDING or RUNNING)
```

### 3.2 Terminal States

- ✅ **COMPLETED**: All steps succeeded
- ✅ **COMPENSATED**: Rolled back successfully
- ❌ **FAILED**: Compensation failed (manual intervention required)
- ⚠️ **CANCELLED**: User/system cancelled

---

## 4. Execution Rules

### 4.1 Forward Execution

```rust
for step in saga.steps {
    loop {
        match execute_step(step).await {
            Ok(_) => break,
            Err(e) if step.attempts < step.max_retries => {
                step.attempts += 1;
                backoff(step.retry_backoff_ms).await;
            }
            Err(e) => {
                // Max retries exhausted
                trigger_compensation().await;
                return Err(SagaFailed);
            }
        }
    }
}
```

### 4.2 Compensation Rules

Compensation MUST:
- ✅ Execute in **reverse order** of forward steps
- ✅ Be **idempotent** (safe to retry)
- ✅ Have **no side effects** beyond reversal
- ✅ Be **deterministic**
- ❌ NOT depend on external state (unless versioned)

**Example**:

```
Forward:       A → B → C
Compensation:  C' → B' → A'

If B fails:
Execute:       A (success) → B (fail) → A' (compensate A)
```

### 4.3 Compensation Ordering

```yaml
CompensationOrder:
  mode: reverse_sequential       # One at a time, reverse order
  
  # Alternative (if safe):
  mode: parallel                 # All compensations in parallel
  max_parallel: 3
```

**Default**: `reverse_sequential` (safest)

---

## 5. Timeout Semantics

### 5.1 Step Timeout

If step exceeds `timeout_ms`:
1. Mark step as FAILED
2. Emit `saga.step.timeout` event
3. Trigger compensation
4. Log timeout details

### 5.2 Saga Timeout

If total execution exceeds `saga.timeout_ms`:
1. Cancel remaining steps
2. Emit `saga.timeout` event
3. Trigger compensation
4. Move to COMPENSATING state

**Default Recommendations**:
- Step timeout: 5 seconds
- Saga timeout: 60 seconds
- AI step timeout: 30 seconds
- Database step timeout: 10 seconds

---

## 6. Retry Policy

### 6.1 Retry Configuration

```yaml
RetryPolicyV5.2:
  max_retries: int                # Max attempts
  backoff_ms: int                 # Initial backoff
  multiplier: float               # Backoff multiplier
  max_backoff_ms: int             # Max backoff cap
  jitter: bool                    # Add randomness
  
  # Conditional retries
  retryable_errors: [string]      # Error codes to retry
  non_retryable_errors: [string]  # Never retry
```

**Default Policy**:

```yaml
max_retries: 5
backoff_ms: 200
multiplier: 2.0
max_backoff_ms: 10000
jitter: true
```

**Backoff Calculation**:

```
backoff = min(
  backoff_ms * (multiplier ^ attempt) + jitter,
  max_backoff_ms
)

Example:
Attempt 1: 200ms + jitter
Attempt 2: 400ms + jitter
Attempt 3: 800ms + jitter
Attempt 4: 1600ms + jitter
Attempt 5: 3200ms + jitter
```

---

## 7. Distributed Locking

### 7.1 Lock Requirements

When multiple sagas may conflict:

```yaml
SagaLock:
  lock_key: string                # Resource identifier
  saga_id: uuidv7                 # Lock owner
  acquired_at: RFC3339
  expires_at: RFC3339             # Auto-release
  region: string
```

**Lock Storage**: Redis or Etcd

**Lock Expiry**: 30 seconds (default)

### 7.2 Deadlock Detection

Deadlock occurs when:
- Saga A holds lock L1, waits for L2
- Saga B holds lock L2, waits for L1

**Detection Strategy**:
1. Monitor lock wait times
2. If wait > threshold (10s), check for cycles
3. Abort one saga (random or priority-based)
4. Emit `saga.deadlock.detected`

**Resolution**:
```
1. Detect cycle
2. Select victim saga (lowest priority or random)
3. Cancel victim saga
4. Trigger victim compensation
5. Allow winner to retry
```

---

## 8. Saga Storage (Durable State)

### 8.1 Saga State Table

```sql
CREATE TABLE sagas (
  saga_id UUID PRIMARY KEY,
  saga_type VARCHAR(255) NOT NULL,
  tenant_id VARCHAR(255) NOT NULL,
  correlation_id UUID NOT NULL,
  
  status VARCHAR(50) NOT NULL,
  current_step INT NOT NULL DEFAULT 0,
  
  definition JSONB NOT NULL,      -- Saga steps & config
  execution_state JSONB,          -- Runtime state
  
  created_at TIMESTAMP NOT NULL,
  started_at TIMESTAMP,
  updated_at TIMESTAMP NOT NULL,
  completed_at TIMESTAMP,
  
  -- NEW in v6.0
  ai_integrated BOOLEAN DEFAULT FALSE,
  incident_id VARCHAR(255),
  
  INDEX idx_tenant_status (tenant_id, status),
  INDEX idx_correlation (correlation_id),
  INDEX idx_incident (incident_id)
);
```

### 8.2 Step Execution Table

```sql
CREATE TABLE saga_steps (
  saga_id UUID NOT NULL,
  step_index INT NOT NULL,
  step_name VARCHAR(255) NOT NULL,
  
  status VARCHAR(50) NOT NULL,
  attempts INT NOT NULL DEFAULT 0,
  last_error TEXT,
  
  started_at TIMESTAMP,
  completed_at TIMESTAMP,
  duration_ms INT,
  
  PRIMARY KEY (saga_id, step_index),
  FOREIGN KEY (saga_id) REFERENCES sagas(saga_id)
);
```

### 8.3 Saga Snapshots

For fast recovery:

```yaml
SagaSnapshotV5.2:
  saga_id: uuidv7
  step: int
  state_hash: string              # sha256 of execution state
  timestamp: RFC3339
  metadata: object
```

**Snapshot Frequency**: Every 5 steps or 30 seconds

---

## 9. High Availability & Failover

### 9.1 Leader Election

Saga orchestrator uses leader election:

**Implementation**:
- etcd (recommended)
- Consul
- Kubernetes leases

**Behavior**:
- Only leader executes sagas
- Followers monitor leader health
- Failover time: < 5 seconds

### 9.2 Failover Process

When leader fails:

```
1. Followers detect leader timeout (6s)
2. Elect new leader
3. New leader:
   a. Load active sagas from DB
   b. Resume RUNNING sagas
   c. Retry PENDING sagas
   d. Complete COMPENSATING sagas
4. Emit saga.orchestrator.failover event
```

### 9.3 Orphan Saga Detection

Orphan = saga stuck without progress

**Detection**:
- Saga in RUNNING but no updates > 2 minutes
- No corresponding orchestrator heartbeat

**Recovery**:
- Recover from last snapshot + step log
- Resume execution or trigger timeout

---

## 10. AI-Integrated Sagas (Enhanced v6.0)

### 10.1 AI Workflow Example

```yaml
SagaType: ai_incident_remediation

Steps:
  1. validate_incident
     action: Check incident exists
     compensation: None (idempotent)
  
  2. deduct_ai_budget
     action: Reserve AI tokens
     compensation: Refund tokens
  
  3. retrieve_context
     action: RAG retrieval from vector DB
     compensation: None (read-only)
  
  4. analyze_with_llm
     action: Call LLM for decision
     compensation: None (stateless)
     timeout_ms: 30000
  
  5. publish_decision
     action: Publish ai.incident.decision
     compensation: Publish rollback event
  
  6. execute_command (optional)
     action: Execute remediation command
     compensation: Revert command
     requires_lock: true
  
  7. update_incident_store
     action: Mark incident as resolved
     compensation: Mark as failed
```

### 10.2 AI Step Compensation

```yaml
# Example: AI Token Deduction Compensation
step: deduct_ai_budget
action:
  type: internal
  handler: ai_billing.deduct
  payload:
    tenant_id: "tenant-123"
    amount_tokens: 1500
    cost_usd: 0.045

compensation:
  type: internal
  handler: ai_billing.refund
  payload:
    tenant_id: "tenant-123"
    amount_tokens: 1500
    cost_usd: 0.045
    reason: "saga_compensation"
```

**Rules for AI Steps**:
- LLM calls MUST be compensatable (refund cost)
- Vector writes MUST be reversible
- RAG retrievals are read-only (no compensation needed)
- Safety violations MUST fail fast (no compensation)

### 10.3 AI Saga Observability

```
# Metrics
saga_ai_execution_total{outcome}
saga_ai_cost_usd{model, outcome}
saga_ai_step_duration_seconds{step, model}
saga_ai_compensation_total{reason}

# Events
saga.ai.started
saga.ai.llm.called
saga.ai.rag.performed
saga.ai.safety.blocked
saga.ai.completed
saga.ai.compensation.refunded
```

---

## 11. Event Integration

### 11.1 Required Saga Events

**Topic Prefix**: `saga.*`

```yaml
saga.started:
  saga_id: uuidv7
  saga_type: string
  tenant_id: string
  steps_count: int

saga.step.started:
  saga_id: uuidv7
  step_index: int
  step_name: string

saga.step.completed:
  saga_id: uuidv7
  step_index: int
  step_name: string
  duration_ms: int

saga.step.failed:
  saga_id: uuidv7
  step_index: int
  step_name: string
  error: string
  retry_attempt: int

saga.step.timeout:
  saga_id: uuidv7
  step_index: int
  timeout_ms: int

saga.compensation.started:
  saga_id: uuidv7
  step_index: int
  reason: string

saga.compensation.completed:
  saga_id: uuidv7
  step_index: int

saga.compensation.failed:
  saga_id: uuidv7
  step_index: int
  error: string

saga.deadlock.detected:
  saga_id: uuidv7
  conflicting_saga_id: uuidv7
  lock_key: string

saga.completed:
  saga_id: uuidv7
  total_duration_ms: int

saga.timeout:
  saga_id: uuidv7
  timeout_ms: int
  current_step: int

saga.cancelled:
  saga_id: uuidv7
  reason: string
```

### 11.2 Event Propagation

All saga events MUST include:
- `saga_id`
- `tenant_id`
- `correlation_id`
- `trace_id`
- Step name (where applicable)

---

## 12. Observability

### 12.1 Metrics

```
# Execution metrics
saga_execution_total{saga_type, outcome}
saga_step_duration_seconds{saga_type, step_name}
saga_duration_seconds{saga_type}

# Failure metrics
saga_failed_total{saga_type, reason}
saga_compensation_total{saga_type, outcome}
saga_retry_total{saga_type, step_name}

# Resource metrics
saga_active_total{saga_type}
saga_pending_total{saga_type}
saga_deadlock_total

# NEW: AI metrics
saga_ai_cost_total_usd{saga_type, model}
```

### 12.2 Traces

```
saga.orchestration (parent span)
  ├─ saga.step.1
  ├─ saga.step.2
  │   ├─ db.query
  │   └─ ai.llm.call
  ├─ saga.step.3
  └─ saga.compensation.2 (if failed)
```

### 12.3 Debug Endpoint

**GET** `/saga/{saga_id}`

```yaml
Response:
  saga_id: string
  status: string
  current_step: int
  
  steps:
    - name: string
      status: string
      attempts: int
      started_at: timestamp
      completed_at: timestamp
      duration_ms: int
      error: string?
  
  timeline: [object]              # Event timeline
  snapshot: object                # Latest snapshot
  
  execution_trace:
    trace_id: string
    spans: [object]
```

---

## 13. Testing & Simulation

### 13.1 Saga Testing

**Unit Tests**:
- Test each step in isolation
- Test compensation logic
- Test idempotency

**Integration Tests**:
- Test full saga execution
- Test failure scenarios
- Test compensation chains

**Chaos Tests**:
- Random step failures
- Timeout injection
- Deadlock simulation
- Network partitions

### 13.2 Saga Simulation API

```
POST /saga/simulate
{
  "saga_type": "ai_incident_remediation",
  "failure_injection": {
    "step": 3,
    "error": "TIMEOUT"
  }
}
```

---

## 14. Failure Modes & Handling

| Failure Mode | Detection | Recovery |
|--------------|-----------|----------|
| Step timeout | Timer expiry | Retry → Compensate |
| Network error | Connection failure | Retry with backoff |
| Database error | Query failure | Retry → Compensate |
| LLM timeout | API timeout | Retry → Fallback → Compensate |
| Deadlock | Lock wait timeout | Abort one saga |
| Orchestrator crash | Heartbeat loss | Leader election → Resume |
| Compensation failure | Retry exhaustion | Manual intervention |

---

## 15. Best Practices

### 15.1 Saga Design

✅ **DO**:
- Keep sagas small (3-7 steps)
- Make steps idempotent
- Design compensations carefully
- Use locks sparingly
- Set realistic timeouts
- Test compensation paths

❌ **DON'T**:
- Create long-running sagas (>5 minutes)
- Chain sagas unnecessarily
- Ignore compensation failures
- Use shared state without locks
- Rely on eventual consistency within saga

### 15.2 Error Handling

```rust
match step_result {
    Ok(_) => proceed_to_next_step(),
    
    Err(TransientError) => retry_with_backoff(),
    
    Err(PermanentError) => {
        log_error();
        trigger_compensation();
    }
}
```

---

## 16. Compliance Checklist

A system is **Saga v6.0 Compliant** if:

✅ Uses canonical state machine  
✅ Implements compensation ordering  
✅ Supports retries with backoff  
✅ Implements timeouts (step + saga)  
✅ Detects deadlocks  
✅ Supports HA/failover  
✅ Persists durable state  
✅ Emits all required events  
✅ Provides debug endpoint  
✅ Traces all executions  
✅ Handles AI workflows correctly (NEW)  
✅ Supports saga simulation  

---

**End of Distributed Transactions v6.0**