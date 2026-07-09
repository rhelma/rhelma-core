# Rhelma6 — Incentives & Value System

**Status:** Draft (Roadmap)

Goal: attract honest compute and grow horizontally, while resisting sybil and abuse.

## 1) Design goals

- **Simple first**: start with reputation + credits, not a complex on-chain token.
- **Anti-sybil aware**: avoid “free farming” by fake nodes.
- **Useful incentives**: reward uptime, correctness, and real work.
- **Governable**: allow policy changes without breaking the network.
- **Upgradeable**: enable an external bridge later without forcing it now.

## 2) Two-layer value model (recommended)

1) **Reputation (non-transferable)**  
   - earned via good performance over time  
   - decays with inactivity  
   - used for scheduling priority and admission to higher-privilege roles

2) **Credits (transferable inside the system)**  
   - earned by executing tasks (work proofs / receipts later)  
   - spent by users/services to request compute  
   - can be bridged to external value later (optional)

This avoids immediate regulatory/operational burden of public crypto, while still enabling “economic gravity”.

## 3) Reward signals (inputs)

Credits/reputation can be computed from:

- Uptime minutes (capped)
- Task success rate (weighted higher)
- Latency vs region baseline
- Resource contribution (GPU availability)
- Verified attestation level
- Positive peer reviews (carefully rate-limited)

## 4) Penalties (negative signals)

- Missed heartbeats / flaky connections
- High error rate / corrupt outputs
- Policy violations
- Detected sybil patterns
- Attempted privilege escalation

Penalties should be **fast**, rewards should be **slow** (security principle).

## 5) Implementation stages

- Phase 1–2: reputation only + scheduler preference
- Phase 3: credits ledger on coordinator/registry (append-only log + signatures)
- Phase 4: federated ledger (multiple coordinators, quorum)
- Phase 5+: optional external settlement / bridging (only if you choose)

## 6) External bridging track (Phase 5+)

If you want a path like **$Rhelma** “into the real world”, define it as a *track* that can remain disabled by default:

- **Bridge interface**: a narrow module that converts internal credits/receipts to external units.
- **Proof-of-work receipts**: signed receipts for completed jobs (verifiable, replay-safe).
- **Governance gates**: N-of-M approvals to enable or change bridge parameters.
- **Compliance hooks**: jurisdiction-aware enablement, risk tiers, KYC/AML only if required by your chosen design.

### Digital family / inheritance compatibility

To align with your constitutional concepts (inheritance, digital family), model ownership as:

- **policy-controlled accounts** (not raw “keys = funds”)
- **escrow + recovery flows** governed by quorum, audits, and time-locks
- **revocable grants** for family roles (limited scopes, rotating keys)

This keeps the system “alive under any condition” while preserving zero-trust and auditability.

## 7) Transparency

Operators should see:
- their score and why it changed
- how to improve
- what policies exist and how they are updated
