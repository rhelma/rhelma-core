# Rhelma6 — Rhelma Native Intelligence (MNI): Realm-Native Collective AI

**Status:** Draft (Operational + Governed)  
**Goal:** Build a **Realm-native intelligence layer** that grows from Rhelma’s own culture, contracts, lexicon, and citizen contributions—without becoming a privacy nightmare or a poisoning vector.

> This is NOT “one big LLM in one place.”  
> MNI is a **versioned, audited, governed artifact** that can be composed from many nodes and many experts.

---

## 1) Core principles

1. **Consent & Ownership First**  
   Data belongs to citizens. Training is opt-in by default.
2. **Lineage & Auditability**  
   Every training sample and derived artifact has provenance (source, license, sensitivity, approvals).
3. **Poisoning Resistance**  
   Training inputs are trust-weighted and must pass safety gates.
4. **Rollback & Repairability**  
   All training runs are versioned, signed, and reversible (model rollback, dataset rollback).
5. **Human Oversight for High Impact**  
   Irreversible policy or enforcement is never solely AI-driven.

---

## 2) Data Constitution (mandatory)

### 2.1 Data tiers
All content ingested into MNI MUST be classified into one of these tiers:

- **Tier A — Public Commons**  
  Publicly shareable, explicitly marked for training. Safe for open RAG and distillation.
- **Tier B — Consent-Based**  
  Created by citizens but explicitly opted-in for training usage under a license.
- **Tier C — Private (No Train)**  
  Allowed for local inference, not used for training or global RAG. May be used for private personal agents.
- **Tier D — Sensitive / Jury-Gated**  
  Security reports, private governance deliberations, personally identifying info, secrets.  
  Only accessible to specific roles with strict access logs; never globally trained without explicit, multi-party approval.

### 2.2 Rights
- **Opt-out / revoke consent** at any time (must remove from future dataset builds).
- **Right-to-forget (best-effort)**  
  We can delete from datasets/RAG stores; model unlearning is hard—so policy must be honest about limitations and use distillation from approved corpora.

### 2.3 Provenance requirements
Every record must include:
- `source_id`, `author_id` (pseudonymous allowed), `created_at`
- `tier`, `license`, `consent_receipt`
- cryptographic `content_hash`
- moderation decisions (if any)

---

## 3) Build strategy (safe → scalable)

### Step 1: Lexicon-first RAG (fast & controllable)
- Treat the **Lexicon** as the canonical dataset for early MNI.
- Provide retrieval with strict tier filtering and policy-residency constraints.
- Output is immediately useful without training risk.

### Step 2: Distillation (small, safe, iterative)
- Train a small “Realm Assistant” model from **Tier A/B only**.
- Use approved transcripts and curated corpora.
- Maintain a signed model registry with rollback.

### Step 3: Expert Composition (MoE-style, permissioned)
- Add experts by domain: `code`, `security`, `governance`, `creative`.
- Experts can be hosted on nodes, but **updates are gated**:
  - attestation threshold
  - trust-weighting
  - anomaly/poison checks
  - jury gating for Tier D content

### Step 4: Federated learning (only when mature)
- Allowed only under **permissioned federation** (high-attestation trainers).
- Gradually relax if/when verifiable training is implemented.

---

## 4) Safety gates (non-negotiable)

1. **Trust-weighted ingestion** (reputation, attestation, history).
2. **Poisoning detection** (distribution shifts, trigger phrases, abnormal gradients in trainers).
3. **Red-teaming** for each release (prompt attacks, backdoors).
4. **Policy testing** (contract adherence, safety).
5. **Rollback**: instant revert to last safe model/dataset.

---

## 5) How this maps to Rhelma6 phases

- **Phase 3–4**:  
  - Lexicon schema + ingestion events + audit trail  
  - RAG over Tier A/B + policy filters  
- **Phase 5**:  
  - Federation enables distributed lexicon replication  
  - Introduce “trainer eligibility” as an attestation tier  
- **Phase 6+**:  
  - Expert composition + agent mobility + controlled training  
  - Begin permissioned federated learning pilots

---

## 6) Minimal specs to implement next

- Lexicon record schema (content_hash + tier + license + provenance)
- Ingestion API (`POST /v1/lexicon/ingest`) with signature + moderation flags
- Retrieval API with tier constraints
- Model registry metadata: version, dataset digest, signatures, rollback pointer

