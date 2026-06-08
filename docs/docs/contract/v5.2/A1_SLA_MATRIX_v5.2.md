# Rhelma SLA Matrix v5.2

**Release:** January 2027  
**Status:** Final - Platform-Wide Standard  
**Supersedes:** v5.1 SLA Matrix

This annex defines all mandatory Service-Level Agreements for Rhelma services.

---

## 1. Overview

### 1.1 SLA Categories

- **Latency** (p95/p99 percentiles)
- **Availability** (uptime percentage)
- **Error rates** (failure thresholds)
- **Throughput** (requests/operations per second)
- **Resource constraints** (CPU, memory, disk)
- **AI-specific SLAs**
- **Event streaming SLAs**
- **Vector/RAG SLAs**
- **Storage latency SLAs**

### 1.2 Measurement Methods

All SLAs MUST be measured using:

✅ Prometheus metrics (real-time)  
✅ OTEL traces (distributed tracing)  
✅ Synthetic probes (external monitoring)  
✅ Error budgets (daily/monthly tracking)  

### 1.3 Measurement Window

- **p95/p99**: Rolling 5-minute window
- **Availability**: Monthly calculation
- **Error rates**: Rolling 1-hour window
- **Error budget**: Monthly reset

---

## 2. HTTP API SLAs

### 2.1 Latency Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| **p50 latency** | < 100ms | 50th percentile |
| **p95 latency** | < 250ms | 95th percentile |
| **p99 latency** | < 500ms | 99th percentile |
| **p99.9 latency** | < 1000ms | 99.9th percentile |

### 2.2 Availability & Errors

| Metric | Target | Notes |
|--------|--------|-------|
| **Availability** | 99.95% | Monthly uptime |
| **Error rate (5xx)** | < 0.1% | Server errors |
| **Error rate (4xx)** | < 2% | Client errors (excluding auth) |
| **Timeout rate** | < 0.05% | Request timeouts |

### 2.3 Throughput & Limits

| Metric | Value |
|--------|-------|
| **Max request size** | 10 MB |
| **Request timeout** | 30 seconds (default), 60s (AI) |
| **Max concurrent requests** | 10,000 per instance |
| **Retry budget** | 2 attempts max |

### 2.4 Required Headers

All HTTP requests MUST include:
- `x-rhelma-request-id`
- `traceparent`
- `x-tenant-id` (multi-tenant services)

Requests missing required headers → HTTP 400

---

## 3. Database (L3) SLAs

### 3.1 Query Performance

| Operation | p95 | p99 | p99.9 |
|-----------|-----|-----|-------|
| **SELECT (simple)** | < 25ms | < 50ms | < 100ms |
| **SELECT (complex)** | < 50ms | < 100ms | < 200ms |
| **INSERT** | < 30ms | < 60ms | < 120ms |
| **UPDATE** | < 40ms | < 80ms | < 150ms |
| **DELETE** | < 30ms | < 60ms | < 120ms |
| **Transaction** | < 60ms | < 120ms | < 250ms |

### 3.2 Reliability

| Metric | Target |
|--------|--------|
| **Connection error rate** | < 0.05% |
| **Transaction rollback rate** | < 0.1% |
| **Deadlock rate** | < 0.01% |
| **Replication lag** | < 150ms |
| **Failover time** | < 10 seconds |
| **Data durability** | 99.999999999% (11 nines) |

### 3.3 Connection Pooling

| Setting | Value |
|---------|-------|
| **Min connections** | 5 |
| **Max connections** | 100 |
| **Idle timeout** | 30 seconds |
| **Connection lifetime** | 30 minutes |

---

## 4. Cache (L1/L2) SLAs

### 4.1 Latency

| Layer | p50 | p95 | p99 |
|-------|-----|-----|-----|
| **L1 (in-memory)** | < 0.5ms | < 1ms | < 2ms |
| **L2 (Redis)** | < 2ms | < 10ms | < 20ms |

### 4.2 Performance

| Metric | Target |
|--------|--------|
| **Cache hit rate** | > 85% |
| **Cache miss penalty** | < 50ms |
| **Eviction rate** | < 5% |
| **Memory usage** | < 80% of allocated |

### 4.3 Availability

| Metric | Target |
|--------|--------|
| **L1 availability** | 100% (ephemeral) |
| **L2 availability** | 99.99% |
| **Failover time (L2)** | < 1 second |

---

## 5. Event Streaming SLAs

### 5.1 Publishing Performance

| Metric | Target |
|--------|--------|
| **Publish latency (p95)** | < 50ms |
| **Publish latency (p99)** | < 100ms |
| **Batch publish (1000 events)** | < 500ms |
| **Throughput** | > 50,000 events/sec per partition |

### 5.2 Consumption Performance

| Metric | Target |
|--------|--------|
| **Consume-to-ack (p99)** | < 3 seconds |
| **End-to-end delivery (p99)** | < 5 seconds |
| **Consumer lag** | < 10 seconds |

### 5.3 Reliability

| Metric | Target |
|--------|--------|
| **Message loss rate** | 0% (at-least-once guaranteed) |
| **DLQ rate** | < 0.1% |
| **Ordering violations** | 0 (ZERO tolerated) |
| **Duplicate rate** | < 0.5% (idempotency required) |

### 5.4 Replay & Recovery

| Metric | Target |
|--------|--------|
| **Replay throughput** | > 10,000 events/sec |
| **Offset reset time** | < 5 seconds |
| **Consumer group rebalance** | < 30 seconds |

---

## 6. AI/LLM SLAs

### 6.1 LLM Inference

| Model Type | p50 | p95 | p99 | Timeout |
|------------|-----|-----|-----|---------|
| **Small (< 7B params)** | < 500ms | < 1000ms | < 1500ms | 5s |
| **Medium (7B-30B)** | < 800ms | < 1500ms | < 2000ms | 10s |
| **Large (> 30B)** | < 1200ms | < 2000ms | < 2500ms | 30s |

### 6.2 RAG Pipeline

| Component | p50 | p95 | p99 |
|-----------|-----|-----|-----|
| **Full RAG pipeline** | < 200ms | < 300ms | < 500ms |
| **Vector retrieval** | < 30ms | < 50ms | < 100ms |
| **Embedding generation** | < 50ms | < 100ms | < 150ms |
| **Re-ranking** | < 30ms | < 60ms | < 100ms |
| **Context synthesis** | < 20ms | < 40ms | < 80ms |

### 6.3 Safety & Moderation

| Check | p95 | p99 |
|-------|-----|-----|
| **PII detection** | < 50ms | < 100ms |
| **Toxicity check** | < 60ms | < 120ms |
| **Hallucination scoring** | < 80ms | < 150ms |
| **Full safety pipeline** | < 120ms | < 200ms |

### 6.4 Reliability

| Metric | Target |
|--------|--------|
| **Failure rate** | < 0.5% |
| **Fallback rate** | < 1% |
| **Safety block rate** | < 0.2% (false positives) |
| **Cost violation rate** | < 0.1% |

### 6.5 AI Incident Analysis (NEW v5.2)

| Metric | Target |
|--------|--------|
| **Incident analysis time** | < 10 seconds |
| **Decision generation** | < 5 seconds |
| **Command execution** | < 30 seconds |
| **Analysis failure rate** | < 2% |

---

## 7. Vector Search SLAs

### 7.1 Search Performance

| Operation | p50 | p95 | p99 |
|-----------|-----|-----|-----|
| **Top-10 search** | < 20ms | < 50ms | < 100ms |
| **Top-100 search** | < 40ms | < 80ms | < 150ms |
| **Filtered search** | < 50ms | < 100ms | < 200ms |
| **Hybrid search** | < 60ms | < 120ms | < 250ms |

### 7.2 Write Performance

| Operation | p50 | p95 | p99 |
|-----------|-----|-----|-----|
| **Single insert** | < 10ms | < 20ms | < 30ms |
| **Batch insert (100)** | < 100ms | < 200ms | < 300ms |
| **Batch insert (1000)** | < 500ms | < 1000ms | < 1500ms |
| **Update** | < 15ms | < 30ms | < 50ms |
| **Delete** | < 10ms | < 20ms | < 30ms |

### 7.3 Index Operations

| Operation | Target |
|-----------|--------|
| **Index build (1M vectors)** | < 10 minutes |
| **Index rebuild throughput** | > 5,000 vectors/sec |
| **Index swap (zero downtime)** | < 1 second |
| **Compaction** | < 30 minutes per index |

### 7.4 Replication & HA

| Metric | Target |
|--------|--------|
| **Replication lag** | < 500ms |
| **Failover time** | < 10 seconds |
| **Vector availability** | 99.9% |

---

## 8. Graph Database SLAs

### 8.1 Query Performance

| Query Type | p50 | p95 | p99 |
|------------|-----|-----|-----|
| **Simple path (depth ≤ 3)** | < 30ms | < 60ms | < 80ms |
| **Entity lookup** | < 20ms | < 40ms | < 50ms |
| **Relationship traversal** | < 40ms | < 80ms | < 150ms |
| **Graph ranking** | < 100ms | < 200ms | < 300ms |
| **Complex query (depth > 5)** | < 200ms | < 400ms | < 500ms |

### 8.2 Write Performance

| Operation | p50 | p95 | p99 |
|-----------|-----|-----|-----|
| **Create node** | < 10ms | < 20ms | < 30ms |
| **Create edge** | < 15ms | < 30ms | < 50ms |
| **Update property** | < 10ms | < 20ms | < 40ms |

---

## 9. Object Storage (L4) SLAs

### 9.1 Performance

| Operation | p50 | p95 | p99 |
|-----------|-----|-----|-----|
| **Upload (< 1MB)** | < 50ms | < 100ms | < 150ms |
| **Upload (> 1MB)** | < 200ms | < 400ms | < 600ms |
| **Download (< 1MB)** | < 40ms | < 80ms | < 120ms |
| **Download (> 1MB)** | < 150ms | < 300ms | < 500ms |
| **List objects** | < 100ms | < 200ms | < 300ms |
| **Delete** | < 50ms | < 100ms | < 150ms |

### 9.2 Reliability

| Metric | Target |
|--------|--------|
| **Durability** | 99.999999999% (11 nines) |
| **Availability** | 99.99% |
| **Replication lag** | < 1 second |

---

## 10. Multi-Region SLAs

### 10.1 Cross-Region Performance

| Metric | Target |
|--------|--------|
| **Cross-region latency** | < 120ms (avg) |
| **Replication lag** | < 5 seconds |
| **Failover time (active/active)** | < 5 minutes |
| **Failover time (active/passive)** | < 15 minutes |

### 10.2 Disaster Recovery (See A2)

| Metric | Target |
|--------|--------|
| **RTO (Recovery Time Objective)** | 1 hour |
| **RPO (Recovery Point Objective)** | 15 minutes |
| **Backup frequency** | Every 15 minutes |
| **Restore time (full)** | < 4 hours |

---

## 11. Resource Utilization SLAs

### 11.1 Compute Resources

| Resource | Target | Alert Threshold |
|----------|--------|-----------------|
| **CPU utilization** | < 70% avg | > 85% |
| **Memory utilization** | < 80% avg | > 90% |
| **Disk utilization** | < 75% avg | > 85% |
| **File descriptors** | < 85% limit | > 95% |
| **Network bandwidth** | < 70% capacity | > 85% |

### 11.2 Autoscaling

| Metric | Trigger | Cooldown |
|--------|---------|----------|
| **Scale up (CPU)** | > 70% for 5 min | 3 minutes |
| **Scale up (Memory)** | > 75% for 5 min | 3 minutes |
| **Scale down** | < 30% for 10 min | 10 minutes |

---

## 12. Error Budget Model

### 12.1 Error Budget Formula

```
ErrorBudget = (1 - AvailabilityTarget) × TotalTime

Example (99.95% availability):
Monthly downtime budget = 0.05% × 30 days × 24h × 60m = 21.6 minutes
```

### 12.2 Budget by Tier

| Service Tier | Monthly Availability | Monthly Error Budget |
|--------------|---------------------|----------------------|
| **Critical** | 99.99% | 4.3 minutes |
| **Core** | 99.95% | 21.6 minutes |
| **Standard** | 99.9% | 43.2 minutes |
| **Best Effort** | 99.5% | 3.6 hours |

### 12.3 Budget Enforcement

**If error budget exhausted**:

1. ⛔ **Freeze deployments** (except emergency fixes)
2. 🔧 **Prioritize reliability work**
3. 📊 **Root cause analysis required**
4. 📧 **Escalate to leadership**
5. 📈 **Publish post-mortem**

**Budget reset**: First day of each month

---

## 13. SLA Monitoring

### 13.1 Required Metrics

```prometheus
# Latency
service_latency_seconds{service, operation, quantile}

# Availability
service_availability_percent{service, region}

# Error rate
service_error_rate_percent{service, error_type}

# Throughput
service_throughput_total{service, operation}

# Resource saturation
service_resource_saturation_percent{service, resource_type}

# Error budget
service_error_budget_remaining_percent{service}
```

### 13.2 Alerting Thresholds

| Severity | Condition | Response Time |
|----------|-----------|---------------|
| **CRITICAL** | SLA breach (p99 > 2x target) | < 5 minutes |
| **HIGH** | SLA warning (p99 > 1.5x target) | < 15 minutes |
| **MEDIUM** | Degraded performance | < 1 hour |
| **LOW** | Approaching limits | Next business day |

---

## 14. Tenant-Specific SLAs

### 14.1 Tier Definitions

| Tier | Availability | Support | Features |
|------|--------------|---------|----------|
| **Enterprise** | 99.99% | 24/7 | Dedicated resources |
| **Pro** | 99.95% | Business hours | Priority support |
| **Standard** | 99.9% | Best effort | Standard features |
| **Free** | 99.5% | Community | Limited |

### 14.2 Tier-Specific Performance

| Metric | Enterprise | Pro | Standard | Free |
|--------|------------|-----|----------|------|
| **API p99** | < 400ms | < 500ms | < 750ms | < 1000ms |
| **AI requests/day** | Unlimited | 10,000 | 1,000 | 100 |
| **Vector search p99** | < 80ms | < 100ms | < 150ms | < 200ms |
| **Storage (GB)** | Custom | 1,000 | 100 | 10 |

---

## 15. SLA Violations

### 15.1 Violation Severity

| Breach | Severity | Action |
|--------|----------|--------|
| **p99 > 3x target** | SEV-1 | Immediate page |
| **p99 > 2x target** | SEV-2 | Alert on-call |
| **Availability < target** | SEV-2 | Incident response |
| **Error budget depleted** | SEV-3 | Deployment freeze |

### 15.2 Incident Classification

| SEV Level | Definition | Response | RCA Required |
|-----------|------------|----------|--------------|
| **SEV-1** | Complete outage | < 5 min | Yes |
| **SEV-2** | Major degradation | < 15 min | Yes |
| **SEV-3** | Minor degradation | < 1 hour | Optional |
| **SEV-4** | Informational | Best effort | No |

---

## 16. Service Credits (SLA Guarantees)

### 16.1 Credit Schedule

| Monthly Availability | Service Credit |
|---------------------|----------------|
| **< 99.95%** | 10% |
| **< 99.9%** | 25% |
| **< 99.5%** | 50% |
| **< 99.0%** | 100% |

**Exclusions**:
- Scheduled maintenance (with 72h notice)
- Force majeure events
- Customer-caused issues
- Third-party provider outages (if reasonable redundancy exists)

---

## 17. Compliance Requirements

A service is **SLA v5.2 Compliant** if:

✅ Meets or exceeds all mandatory thresholds  
✅ Emits required Prometheus metrics  
✅ Tracks error budgets monthly  
✅ Provides per-tenant SLA breakdown  
✅ Maintains valid autoscaling config  
✅ Alerts on SLA violations  
✅ Publishes monthly SLA reports  
✅ Conducts quarterly SLA reviews  

---

## 18. SLA Reporting

### 18.1 Required Reports

| Report | Frequency | Audience |
|--------|-----------|----------|
| **SLA Dashboard** | Real-time | Engineering |
| **Monthly SLA Report** | Monthly | Leadership |
| **Quarterly Review** | Quarterly | Executive |
| **Annual Summary** | Annually | Board |

### 18.2 Report Contents

- Availability percentage
- p50/p95/p99 latencies
- Error rates by category
- Error budget consumption
- SLA violations (count, severity)
- Improvement initiatives

---

**End of SLA Matrix v5.2**