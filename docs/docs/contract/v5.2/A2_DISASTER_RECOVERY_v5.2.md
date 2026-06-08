# Rhelma Disaster Recovery v5.2

**Release:** January 2027  
**Status:** Final, Enterprise-Ready  
**Supersedes:** v5.1 DR/BCP

This document defines disaster recovery strategy, RTO/RPO, failover rules, backup policies, and recovery procedures for Rhelma systems in multi-region, multi-tenant environments.

---

## 1. DR Principles

1. **No single region failure may cause platform downtime**
2. **RTO/RPO MUST be predictable and measurable**
3. **Data residency MUST NOT be violated during DR**
4. **Backups MUST be encrypted and integrity-verified**
5. **Failover MUST be automatable**
6. **AI, vector search, event streams MUST remain recoverable**
7. **DR testing MUST be periodic and auditable**
8. **AI incident systems MUST recover gracefully** (NEW v5.2)

---

## 2. Service Tiering for DR

### 2.1 Tier Definitions

| Tier | Services | RTO | RPO | DR Requirement |
|------|----------|-----|-----|----------------|
| **Tier 1 (Critical)** | Core API, AI Orchestrator, Event Bus, Vector DB, Config Store, Identity | **1 hour** | **15 min** | Active/active multi-region |
| **Tier 2 (Core)** | Caching, Analytics, Monitoring | **4 hours** | **1 hour** | Active/passive or multi-AZ |
| **Tier 3 (Supporting)** | Batch jobs, Reporting, Archival | **24 hours** | **12 hours** | Backups only |

### 2.2 Tier 1 Requirements

**Mandatory**:
- ✅ Active/active in ≥2 regions
- ✅ Real-time replication (where residency allows)
- ✅ Automated failover
- ✅ Load balancing across regions
- ✅ Health checks every 10 seconds
- ✅ Quarterly DR drills

---

## 3. RTO / RPO Definitions

### 3.1 Recovery Time Objective (RTO)

**Definition**: Maximum acceptable downtime

```
RTO = Detection Time + Decision Time + Recovery Time + Verification Time

Example Tier 1:
- Detection: 30 seconds
- Decision: 2 minutes
- Recovery: 15 minutes
- Verification: 5 minutes
Total RTO: ~22 minutes (target: < 1 hour)
```

### 3.2 Recovery Point Objective (RPO)

**Definition**: Maximum tolerable data loss

```
RPO = Time between last successful backup and failure

Target RPO: 15 minutes (Tier 1)
Implementation: Incremental backups every 15 minutes
```

### 3.3 Residency Constraints

| Tenant Type | Cross-Region Replication | DR Strategy |
|-------------|--------------------------|-------------|
| **GLOBAL** | ✅ Allowed | Multi-region active/active |
| **REGIONAL_PREFERRED** | ⚠️ With warning | Multi-region with priority |
| **REGIONAL_STRICT** | ❌ Forbidden | Multi-AZ within region only |

**CRITICAL**: STRICT tenants MUST have regional DR with NO cross-region data movement.

---

## 4. Backup Strategy

### 4.1 Backup Frequency

| Storage Layer | Backup Type | Frequency | Retention |
|---------------|-------------|-----------|-----------|
| **L3 DB (SQL)** | Incremental | Every 15 min | 30 days |
| **L3 DB (SQL)** | Full | Daily | 90 days |
| **L3 NoSQL** | Snapshot | Every 30 min | 30 days |
| **Vector DB** | Snapshot | Every 30 min | 30 days |
| **Graph DB** | Export | Hourly | 30 days |
| **L4 Object Storage** | Versioning | Continuous | 90 days |
| **Config Store** | On change | Immediate | 180 days |
| **Audit Store** | Append-only | Continuous | 7 years |
| **AI Models** | Versioned | On update | Indefinite |

### 4.2 Backup Requirements

**All backups MUST**:
- ✅ Be stored in separate region (if residency allows)
- ✅ Be encrypted with AES-256
- ✅ Include integrity checksum (SHA-256)
- ✅ Be tested quarterly (restore validation)
- ✅ Have documented restore procedures

### 4.3 Backup Validation

```yaml
BackupValidation:
  backup_id: uuidv7
  backup_type: incremental | full | snapshot
  created_at: RFC3339
  size_bytes: int
  checksum: string              # SHA-256
  validation_status: enum       # PENDING | VALID | INVALID
  validated_at: RFC3339?
  restore_tested_at: RFC3339?
```

**Validation Process**:
1. Verify checksum
2. Attempt partial restore to test environment
3. Verify data integrity
4. Document result

**Testing Schedule**: Every backup MUST be tested within 90 days of creation.

---

## 5. Multi-Region Deployment Models

### 5.1 Mode A: Active/Active (Tier 1)

**Architecture**:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Region A  │────│  Global DNS │────│   Region B  │
│  (Primary)  │     │ Load Balancer│    │ (Secondary) │
└─────────────┘     └─────────────┘     └─────────────┘
      │                                        │
      │          Bi-directional                │
      │◄───────────Replication────────────────►│
      │                                        │
```

**Requirements**:
- Both regions serve traffic simultaneously
- Global traffic router (GeoDNS, Cloudflare, Route53)
- Conflict-free replication (CRDTs or last-write-wins)
- Event stream alignment
- Vector DB replication (GLOBAL tenants only)

**Failover**: Traffic automatically re-routes to healthy region

### 5.2 Mode B: Active/Passive (Tier 2)

**Architecture**:

```
┌─────────────┐          ┌─────────────┐
│   Primary   │─────────►│  Standby    │
│   Region    │  Async   │   Region    │
│  (Active)   │  Repl    │  (Passive)  │
└─────────────┘          └─────────────┘
```

**Requirements**:
- Primary handles all traffic
- Standby is hot-standby (data replicated, services idle)
- Manual or automatic failover
- Replication lag < 5 seconds

**Failover**: Promote standby → redirect traffic

### 5.3 Mode C: Multi-AZ (STRICT Residency)

**Architecture**:

```
        ┌────────────Region─────────────┐
        │                               │
┌───────┴─────┐   ┌──────────┐   ┌─────┴─────┐
│   Zone A    │   │  Zone B  │   │  Zone C   │
│  (Active)   │   │ (Active) │   │ (Standby) │
└─────────────┘   └──────────┘   └───────────┘
        │               │               │
        └───────────────┴───────────────┘
              Intra-region replication
```

**Requirements**:
- No cross-region data movement
- Multi-AZ for HA within region
- Quorum-based writes (2 of 3)

**Failover**: Zone-level only

---

## 6. Failover Procedures

### 6.1 Automated Failover

**Trigger Conditions**:

| Condition | Detection | Action |
|-----------|-----------|--------|
| Region outage | Health check fails for 60s | Promote secondary |
| AZ failure | 2 AZs unhealthy | Evacuate to healthy AZ |
| Database unrecoverable | Corruption detected | Restore from backup |
| Event stream corruption | Checksum mismatch | Replay from snapshot |
| Vector DB failure | Query timeout > 30s | Switch to replica |
| AI provider regional outage | 5xx errors > 50% | Route to alternate region |

### 6.2 Failover Workflow

```
1. Detection
   └─ Health monitors detect failure
   
2. Validation
   └─ Confirm failure (not false positive)
   
3. Decision
   ├─ Check failover policy
   ├─ Verify standby health
   └─ Authorize failover (auto or manual)
   
4. Execution
   ├─ Update DNS/load balancer
   ├─ Promote standby to primary
   ├─ Resume writes
   └─ Sync lagging data
   
5. Verification
   ├─ Test critical paths
   ├─ Verify data integrity
   └─ Monitor for anomalies
   
6. Communication
   ├─ Status page update
   ├─ Tenant notification
   └─ Incident log
```

### 6.3 Failover Time Requirements

| Component | Detection | Promotion | Total RTO |
|-----------|-----------|-----------|-----------|
| **Active/active** | < 30s | < 2 min | < 5 min |
| **Active/passive** | < 1 min | < 10 min | < 15 min |
| **Multi-AZ** | < 10s | < 30s | < 1 min |

### 6.4 Rollback Procedures

If failover causes issues:

```
1. Assess impact
2. Decision: Rollback vs. Fix-forward
3. If rollback:
   a. Reverse DNS changes
   b. Demote promoted region
   c. Restore original primary
   d. Verify consistency
4. Document lessons learned
```

---

## 7. Component-Specific DR

### 7.1 Event Bus DR

**Requirements**:
- Preserve offsets
- Maintain ordering within partitions
- Replay capability from snapshots
- Cross-region offset sync (GLOBAL tenants)

**DR Process**:

```
1. Snapshot offsets to L4 (every 30s)
2. On failover:
   a. Load offset snapshot
   b. Resume from last committed offset
   c. Rebuild consumer groups
   d. Resume processing
3. Verify no message loss
```

**Events**:

```yaml
dr.event.failover.started:
  event_bus_id: string
  from_region: string
  to_region: string
  offset_snapshot_id: string

dr.event.offset.restored:
  consumer_group: string
  partition: int
  restored_offset: int
```

### 7.2 Vector DB DR

**Snapshot Strategy**:

```yaml
VectorSnapshot:
  snapshot_id: uuidv7
  index_name: string
  tenant_id: string?
  vector_count: int
  created_at: RFC3339
  size_bytes: int
  storage_location: string      # S3/GCS URI
  checksum: string
```

**DR Process**:

```
1. Take snapshot every 30 minutes
2. Store in L4 (cross-region if allowed)
3. On failover:
   a. Load latest snapshot
   b. Replay WAL (Write-Ahead Log)
   c. Rebuild indexes
   d. Verify vector count
4. Switch traffic to recovered index
```

**Recovery Time**: < 10 minutes for 1M vectors

### 7.3 AI Orchestrator DR

**State to Preserve**:

- Prompt registry versions
- Cost budgets (per tenant)
- Safety rules
- Routing decisions cache
- AI model availability
- Incident analysis state (NEW v5.2)

**DR Process**:

```
1. Replicate prompt registry to standby
2. Sync cost budgets every minute
3. On failover:
   a. Resume AI routing with standby
   b. Reconcile costs (may have lag)
   c. Resume incident analysis
   d. Verify safety rules active
4. Emit dr.ai.failover event
```

**Incidents During DR** (NEW v5.2):

```yaml
IncidentDRStrategy:
  # In-progress incidents
  in_progress:
    - Save current state to durable storage
    - Resume analysis in new region
    - Link to original incident_id
  
  # Pending commands
  pending_commands:
    - Replay from event log
    - Verify idempotency
    - Resume execution
```

### 7.4 Saga Engine DR

**Saga Recovery**:

```
1. Load active sagas from database
2. Determine saga state:
   a. PENDING → Restart
   b. RUNNING → Resume from last step
   c. COMPENSATING → Complete compensation
   d. FAILED → Manual review
3. Emit saga.recovery events
4. Verify no orphaned sagas
```

**Orphan Detection**:

```sql
SELECT saga_id, status, updated_at
FROM sagas
WHERE status IN ('RUNNING', 'COMPENSATING')
  AND updated_at < NOW() - INTERVAL '5 minutes';
```

### 7.5 Configuration DR

**Config Snapshot**:

```yaml
ConfigSnapshot:
  snapshot_id: uuidv7
  environment: string
  service: string?
  tenant_id: string?
  version: semver
  config_bundle: object
  created_at: RFC3339
```

**DR Process**:

```
1. Version all configs with git
2. Replicate to standby config store
3. On failover:
   a. Load last-known-good config
   b. Verify schema
   c. Apply to services
   d. Emit config.dr.loaded
```

---

## 8. Backup & Restore Procedures

### 8.1 Database Restore

**Full Restore Process**:

```
1. Identify backup to restore
   └─ Check backup_id, timestamp, checksum

2. Validate backup integrity
   └─ Verify checksum matches

3. Stop writes to database
   └─ Put database in read-only mode

4. Restore full backup
   └─ Time: ~1 hour per 100GB

5. Apply incremental backups
   └─ Replay transaction logs

6. Verify data integrity
   ├─ Row counts
   ├─ Key samples
   └─ Foreign key constraints

7. Resume writes
   └─ Remove read-only mode

8. Monitor for anomalies
   └─ Check error rates, latency
```

**Estimated Times**:

| Database Size | Full Restore | Incremental | Total |
|---------------|--------------|-------------|-------|
| < 10 GB | 10 min | 5 min | 15 min |
| 10-100 GB | 1 hour | 15 min | 1.25 hours |
| 100-500 GB | 3 hours | 30 min | 3.5 hours |
| > 500 GB | 6+ hours | 1 hour | 7+ hours |

### 8.2 Point-in-Time Recovery (PITR)

**Use Case**: Restore to specific timestamp (e.g., before data corruption)

**Process**:

```
1. Restore full backup before target time
2. Replay transaction logs up to target time
3. Verify data state at target time
4. Optionally fork to new database
```

**Example**:

```sql
-- PostgreSQL PITR
pg_restore --target-time='2027-01-15 10:30:00'
```

---

## 9. DR Testing

### 9.1 Test Types

| Test Type | Frequency | Scope | Downtime |
|-----------|-----------|-------|----------|
| **GameDay** | Monthly | Single service | 0 (chaos in prod) |
| **Failover Drill** | Quarterly | Full region | Scheduled |
| **Backup Restore** | Quarterly | All backups | Test env only |
| **Full DR Exercise** | Annually | Entire platform | Scheduled window |

### 9.2 GameDay Chaos Tests

**Scenarios**:

- ☁️ Region outage (AWS/GCP/Azure)
- 🗄️ Database primary failure
- 📡 Event bus partition
- 🧠 AI provider outage
- 🔍 Vector DB corruption
- 🌐 Network partition
- 💾 Disk full
- 🔐 Certificate expiry

**Execution**:

```yaml
ChaosExperiment:
  experiment_id: uuidv7
  scenario: string
  target:
    service: string
    region: string
    component: string
  
  blast_radius: enum            # SINGLE_INSTANCE | SERVICE | REGION
  
  duration_seconds: int
  started_at: RFC3339
  
  expected_behavior:
    - Automatic failover within 5 min
    - No data loss
    - Error rate < 0.5%
  
  actual_behavior:
    - ...
  
  lessons_learned: [string]
```

### 9.3 DR Drill Checklist

**Pre-Drill**:
- [ ] Notify stakeholders (72h advance)
- [ ] Update status page
- [ ] Prepare runbooks
- [ ] Set up monitoring dashboards
- [ ] Brief on-call team

**During Drill**:
- [ ] Trigger failover
- [ ] Monitor metrics
- [ ] Verify data integrity
- [ ] Test critical user flows
- [ ] Document timeline
- [ ] Measure RTO/RPO

**Post-Drill**:
- [ ] Rollback or keep failover
- [ ] Update status page
- [ ] Conduct debrief
- [ ] Document improvements
- [ ] Update runbooks
- [ ] Share report

---

## 10. Communication & Escalation

### 10.1 Status Page

**Real-time DR Status**:

```
GET /status/dr

Response:
{
  "status": "active" | "failover_in_progress" | "recovered",
  "primary_region": "us-east-1",
  "active_region": "eu-central-1",
  "failover_started_at": "2027-01-15T10:30:00Z",
  "affected_services": ["api", "ai-orchestrator"],
  "eta_recovery": "2027-01-15T11:00:00Z"
}
```

### 10.2 Tenant Notification

**Email Template**:

```
Subject: [Rhelma] Disaster Recovery Event - Region Failover

Dear [Tenant],

We have initiated a disaster recovery procedure due to [reason].

Status: Failover in progress
Affected Region: [region]
New Active Region: [region]
Expected Recovery: [time]

Your data is safe and replicated. No action is required.

We will update you upon completion.

- Rhelma Operations Team
```

### 10.3 Escalation Path

| Time Elapsed | Action | Notify |
|--------------|--------|--------|
| **0-5 min** | On-call engineer investigates | Team lead |
| **5-15 min** | Initiate automated failover | Engineering manager |
| **15-30 min** | Manual intervention if needed | VP Engineering |
| **30-60 min** | Full incident response | CTO, CEO |
| **> 60 min** | External communication | PR team, customers |

---

## 11. Observability During DR

### 11.1 DR Metrics

```prometheus
# Failover metrics
dr_failover_total{from_region, to_region, component}
dr_failover_duration_seconds{component}

# Recovery metrics
dr_recovery_time_seconds{component}
dr_data_loss_bytes{component}

# Backup metrics
dr_backup_success_total{component}
dr_backup_duration_seconds{component}
dr_restore_duration_seconds{component}

# Replication lag
dr_replication_lag_seconds{from_region, to_region}
```

### 11.2 DR Events

```yaml
dr.failover.initiated:
  component: string
  from_region: string
  to_region: string
  reason: string
  initiated_by: string          # system | human

dr.failover.completed:
  component: string
  to_region: string
  duration_seconds: int
  data_loss: bool
  rto_met: bool
  rpo_met: bool

dr.backup.created:
  backup_id: uuidv7
  component: string
  size_bytes: int
  duration_seconds: int

dr.restore.completed:
  backup_id: uuidv7
  component: string
  duration_seconds: int
  target_region: string
```

---

## 12. Runbooks

### 12.1 Required Runbooks

Every Tier 1 service MUST have:

1. **DR Overview**
   - Architecture diagram
   - Dependencies
   - RTO/RPO targets

2. **Failover Decision Matrix**
   - Trigger conditions
   - Impact assessment
   - Go/no-go criteria

3. **Manual Failover Instructions**
   - Step-by-step commands
   - Verification steps
   - Rollback procedure

4. **Automated Failover Config**
   - Health check parameters
   - Failover thresholds
   - Notification settings

5. **Verification Checklist**
   - Critical paths to test
   - Data integrity checks
   - Performance baselines

6. **Rollback Procedure**
   - Conditions for rollback
   - Rollback steps
   - Risk assessment

7. **Escalation Paths**
   - Contact information
   - Decision authority
   - Communication plan

**Update Frequency**: Every 6 months or after major changes

### 12.2 Example Runbook Snippet

```markdown
# AI Orchestrator Failover Runbook

## Trigger
- AI API error rate > 50% for 5 minutes
- All providers unreachable
- Vector DB unresponsive

## Pre-Checks
1. Verify standby region health
2. Check replication lag (< 1 minute)
3. Confirm no ongoing deployments

## Failover Steps
1. Update DNS: `aws route53 change-resource-record-sets ...`
2. Promote standby: `kubectl scale --replicas=10 ai-orchestrator`
3. Verify health: `curl https://ai.rhelma.internal/health`
4. Resume traffic: 10% → 50% → 100%

## Verification
- [ ] Health check passes
- [ ] LLM requests succeed
- [ ] Vector search responds
- [ ] Cost tracking active
- [ ] Incident analysis working (NEW)

## Rollback
If issues detected within 30 minutes:
1. Revert DNS
2. Scale down promoted region
3. Investigate root cause
```

---

## 13. Compliance & Auditing

### 13.1 DR Audit Trail

All DR events MUST be logged in `ops.audit@v2`:

```yaml
AuditEvent:
  action: dr_failover_initiated
  outcome: success
  actor: system/dr-controller
  resource_type: service
  resource_id: ai-orchestrator
  
  context:
    from_region: us-east-1
    to_region: eu-central-1
    reason: region_outage
    rto_met: true
    rpo_met: true
  
  timestamp: RFC3339
  signature: <ed25519>
```

### 13.2 DR Compliance Checklist

A system is **DR v5.2 Compliant** if:

✅ Meets RTO/RPO targets for tier  
✅ Automated failover configured  
✅ Backups encrypted & validated  
✅ Quarterly DR drills conducted  
✅ Cross-region replication (where allowed)  
✅ Residency respected during DR  
✅ Runbooks updated & tested  
✅ Observability signals active  
✅ Communication plan documented  
✅ AI systems recover correctly (NEW)  
✅ Incident analysis resumes (NEW)  

---

## 14. Future Enhancements

### 14.1 Planned Improvements

**Q2 2027**:
- Chaos engineering platform (Gremlin, Chaos Mesh)
- Automated DR testing suite
- Multi-cloud DR (AWS + GCP)

**Q3 2027**:
- Predictive failover (ML-based)
- Zero-downtime failover for all Tier 1
- Real-time DR simulation environment

**Q4 2027**:
- Blockchain-based audit trail
- AI-driven DR decision making
- Global load balancing with smart routing

---

## 15. Lessons Learned Repository

**Post-Incident Reviews**:

```yaml
DRLessonLearned:
  incident_id: string
  date: RFC3339
  component: string
  trigger: string
  
  what_worked:
    - Automated failover triggered correctly
    - RTO target met (45 minutes vs 60 minute target)
  
  what_failed:
    - Manual verification took 15 minutes (slow)
    - Some tenant notifications delayed
  
  action_items:
    - Automate verification checks
    - Improve notification system
    - Update runbook with new steps
  
  owner: string
  due_date: RFC3339
```

---

**End of Disaster Recovery v5.2**