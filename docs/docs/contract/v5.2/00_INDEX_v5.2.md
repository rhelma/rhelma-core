# Rhelma Contract v5.2 – Consolidated Document Index

**Release Date:** January 2027  
**Status:** Final, Enterprise-Ready  
**Supersedes:** v5.1 (all documents consolidated and harmonized)

This suite defines the complete architecture and operational contract for Rhelma-based distributed systems with AI-native, zero-trust, event-driven capabilities.

---

## Navigation

- Rules / policies: `docs/contract/v5.2/rules/README.md`
- Specs & protocols: `docs/contract/v5.2/specs/README.md`
- Main documentation: `docs/README.md`

## Core Architecture Documents

### 1. 01_ARCHITECTURE_CORE_v5.2.md
**Unified system architecture** covering:
- RequestContext v5.2 (harmonized)
- Error model v5.2
- Storage layers (L1-L4)
- Tenancy & residency
- Configuration management
- Versioning & governance

### 2. 02_OBSERVABILITY_v5.2.md
**Complete observability specification** including:
- Structured logging (JSON)
- Distributed tracing (W3C)
- Metrics (Prometheus)
- Audit & tamper-proof logs
- Health & heartbeat
- PII redaction rules
- AI observability extensions

### 3. 03_AI_ORCHESTRATION_v5.2.md
**AI/LLM orchestration standard** covering:
- Router v3 architecture
- RAG pipeline (8-stage)
- Prompt registry
- Cost governance
- Safety & moderation
- Multi-modal support
- AI incident decision engine (NEW)
- Tool/function calling

### 4. 04_EVENT_DRIVEN_v5.2.md
**Event-driven architecture** including:
- Event envelope v5.2
- CQRS patterns
- Idempotency guarantees
- Schema registry
- DLQ handling
- Replay model
- AI event streams (NEW)
- Topic taxonomy

### 5. 05_SECURITY_v5.2.md
**Zero-trust security** covering:
- Identity (SPIFFE/X.509)
- Authentication (mTLS, OIDC)
- Authorization (RBAC/PBAC)
- Encryption (transit & rest)
- Secrets management
- Runtime security
- Supply chain security
- AI safety integration

### 6. 06_DISTRIBUTED_TRANSACTIONS_v5.2.md
**Saga pattern implementation** including:
- State machine
- Compensation rules
- Timeout semantics
- Deadlock detection
- HA & failover
- Distributed locks
- AI-integrated sagas

### 7. 07_DATA_LAYER_v5.2.md
**Unified data layer specification** covering:
- Vector databases
- Graph databases
- Embeddings lifecycle
- Hybrid retrieval
- Multi-modal embeddings
- Replication & HA
- Security & residency

---

## Annexes

### A1. A1_SLA_MATRIX_v5.2.md
Comprehensive SLA definitions for:
- HTTP APIs
- Databases
- Caching
- Event streaming
- AI/LLM operations
- Vector search
- Multi-region operations

### A2. A2_DISASTER_RECOVERY_v5.2.md
Complete DR/BCP strategy including:
- RTO/RPO requirements
- Failover procedures
- Backup policies
- Multi-region strategies
- Testing requirements

### A3. A3_EVENT_CATALOG_v5.2.md (NEW)
Canonical event topic map with:
- Topic taxonomy
- Producer/consumer contracts
- Payload schemas
- Ordering guarantees
- Versioning rules

---

## Key Changes from v5.1

### Consolidation
- Merged AI Incident Pipeline into core AI Orchestration
- Unified event specifications into single Event-Driven document
- Combined Vector/Graph/Embeddings into Data Layer

### New Features
- AI-assisted incident decision engine
- Cryptographic audit chain (ops.audit@v2)
- Enhanced PII sanitization pipeline
- Residency-aware command routing
- Canonical event catalog

### Breaking Changes
- RequestContext upgraded to v5.2 (adds `flags.ai_safe_mode`)
- Event envelope includes mandatory `residency` field
- Audit events require ed25519 signatures
- AI commands require incident_id linkage

---

## Compatibility Notes

- **Backward compatible** with v5.1 for most operations
- **Breaking changes** marked with ⚠️ throughout documents
- Migration guide available in each document
- All services must adopt v5.2 by Q3 2027

---

## Versioning Strategy

```
MAJOR.MINOR.PATCH
5.2.0 = Current stable release

MAJOR: Breaking changes to core contracts
MINOR: New features, backward-compatible additions
PATCH: Bug fixes, clarifications
```

---

## Compliance Requirements

A system is **Rhelma v5.2 Compliant** if:

✅ Implements RequestContext v5.2  
✅ Follows event envelope v5.2  
✅ Meets all SLA targets (A1)  
✅ Implements DR procedures (A2)  
✅ Uses canonical event topics (A3)  
✅ Enforces zero-trust security  
✅ Supports AI observability  
✅ Maintains audit chain integrity  

---

## Document Dependencies

```
01_ARCHITECTURE_CORE
    ├── 02_OBSERVABILITY
    ├── 03_AI_ORCHESTRATION
    │   └── 07_DATA_LAYER
    ├── 04_EVENT_DRIVEN
    │   └── A3_EVENT_CATALOG
    ├── 05_SECURITY
    └── 06_DISTRIBUTED_TRANSACTIONS

A1_SLA_MATRIX (referenced by all)
A2_DISASTER_RECOVERY (referenced by all)
```

---

## Quick Start

1. **New deployments**: Start with 01_ARCHITECTURE_CORE
2. **AI services**: Focus on 03_AI_ORCHESTRATION + 07_DATA_LAYER
3. **Event-driven systems**: Read 04_EVENT_DRIVEN + A3_EVENT_CATALOG
4. **Security audits**: Begin with 05_SECURITY
5. **Operations**: Study A1_SLA_MATRIX + A2_DISASTER_RECOVERY

---

## Support & Feedback

- **Issues**: Report via internal Rhelma governance board
- **Changes**: Follow RFC process for contract modifications
- **Questions**: Consult architecture review committee

---

**End of Index v5.2**