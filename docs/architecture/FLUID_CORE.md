# Rhelma6 — Architecture: Fluid Core + Distributed Swarm

**Status:** Draft (Roadmap)

## 1) Target architecture (high level)

Rhelma6 splits the “core intelligence” into three cooperating planes:

1) **Control Plane (Governance + Registry)**  
   - Node identity, admission, attestation verification  
   - Reputation/value accounting  
   - Policy distribution (allow-lists, model routing constraints)

2) **Data Plane (Work + Events)**  
   - Task routing, event streaming, content addressing  
   - Contract-aligned EventEnvelope + RequestContext propagation

3) **Execution Plane (Swarm compute)**  
   - Inference nodes (LLM, embeddings, rerank)  
   - Tool nodes (search, storage, sandbox, patch apply)  
   - Edge/near-user nodes for latency-sensitive tasks

Fluidity comes from **dynamic routing** across the execution plane, not from one fixed “central model”.

## 2) Start centralized, evolve decentralized (bootstrap strategy)

We start with a **Bootstrap Coordinator** (BC) that provides:

- Node registration and capability discovery
- A “routing directory” for the initial swarm
- A minimal policy distribution channel

Later, BC responsibilities migrate to a decentralized substrate:
- **Distributed registry** (DHT / CRDT-based)
- **Governance ledger** (append-only log with quorum signatures)
- **Federated routing** (multiple coordinators with consensus)

## 3) Routing model (practical)

Routing is multi-stage and policy-driven:

1. **Ingress** receives a request (API Gateway / local node)
2. **Router** chooses:
   - model family (cheap vs high quality)
   - node class (edge vs regional vs specialized)
   - safety mode (ai_safe_mode from contract v5.2)
3. **Scheduler** selects specific node(s) using:
   - health score + latency
   - trust/attestation level
   - reputation/value score
   - cost budgets / quotas
4. **Executor** runs on the chosen node and returns:
   - output + telemetry + cost report
   - signed receipts (optional, later)

## 4) Swarm roles (node types)

- **Gateway Node**: ingress, rate limiting, authentication, request fanout
- **Inference Node**: text generation, tool calling, multi-model routing
- **Retrieval Node**: vector search, hybrid ranking
- **Storage Node**: content-addressed storage, object store proxy
- **Audit Node**: verifies signatures, stores tamper-evident logs
- **Watchdog Node**: anomaly detection, incident signals
- **Builder Node (restricted)**: runs sandboxed evaluation jobs

A single physical machine may run multiple roles, but roles remain logically separated.

## 5) Contract alignment points (must keep)

- Use RequestContext v5.2 (headers, trace context, ai_safe_mode flag)
- Use EventEnvelope v5.2 for node events (with residency + optional signatures)
- Use audit signatures for privileged actions (ops.audit*)

## 6) Degraded mode requirements

When the Bootstrap Coordinator is unavailable:
- Nodes must still be able to **serve cached routing** for a limited TTL.
- Nodes must still be able to **peer-discover** (last-known peers, signed seed list).
- Writes to registry/value system may queue locally and reconcile later.
