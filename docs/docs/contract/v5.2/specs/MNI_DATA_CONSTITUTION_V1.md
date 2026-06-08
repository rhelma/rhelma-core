# Rhelma Native Intelligence — Data Constitution v1 (MVP)

This constitution defines how Realm data may be used to build Rhelma-native intelligence.

## Data Tiers
1. **Public Commons** — freely usable for RAG; training allowed in later phases by governance.
2. **Consent-based** — usable only when explicit consent is recorded.
3. **Sensitive / Jury-gated** — usable only with explicit incident/jury approval; defaults to *no training*.
4. **Private / No-train** — never used for training; access is policy-gated.

## Non-negotiables
- Default is **privacy**; inclusion is explicit.
- Every dataset item must have **lineage** (source, tier, consent, timestamp).
- Poisoning defense: trust-weighted ingestion, anomaly checks, and appeal path.

## Phase mapping
- Phase 31: RAG only (no training)
- Phase 32: signed lineage + storage + checkpoint roots
- Phase 33+: distillation, expert routing, then permissioned federated learning
