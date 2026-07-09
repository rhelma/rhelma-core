# Rhelma6 — Security, Attestation & Trust

**Status:** Draft (Roadmap)

Trust in Rhelma6 is earned and verifiable. We assume hostile networks.

## 1) Threat model (minimum)

- Sybil nodes joining at scale
- Nodes lying about capability
- Nodes returning poisoned outputs
- Traffic interception / replay
- Coordinator compromise (bootstrap phase)
- Supply chain attacks (modified binaries)

## 2) Controls (phase-based)

### Phase 1 (bootstrap)
- Node keypairs (Ed25519)
- Signed registration + heartbeat
- TLS + mutual auth where possible (mTLS behind gateway)
- Basic anti-sybil (rate limits, allowlist, invite codes)

### Phase 2–3 (attestation)
- Software attestation: signed build provenance (SLSA-like)
- Optional hardware attestation (TPM/TEE) where available
- Remote measurement: binary hash + config hash

### Phase 4+ (distributed trust)
- Quorum-based policy signing
- Peer validation + challenge protocols
- Reputation-weighted scheduling (with caps)

## 3) Attestation levels

- L0: none (local/dev only)
- L1: binary signature verified (release key)
- L2: runtime measurement (hash + config) verified
- L3: hardware-backed attestation verified (TPM/TEE)

Scheduling policy can require minimum attestation for certain roles (e.g., Builder Node).

## 4) Anti-sybil strategy

Combine:
- admission throttling
- reputation maturation (slow reward)
- proof-of-resource (optional later)
- operator verification (optional, depending on goals)

## 5) Safety boundaries

Untrusted nodes MUST NOT:
- apply patches
- manage secrets
- access tenant data beyond authorized scope
- execute privileged tools without containment

## Live defense roles (Rhelma Realm Security System)

Rhelma6 security uses a **three-layer live defense** model:
- **Everyday Guardians:** all citizens can report threats (low-friction reporting, low-evidence signals).
- **Honorary Police:** trained volunteers with higher trust and better tools (triage, guidance, incident support).
- **Digital Champions:** rotating top defenders (proposal-only powers), selected by verified impact and citizen voting.

This model is specified in: `docs/09_SECURITY_SYSTEM_LIVE_DEFENSE.md`.

### Evidence tiers and proportional response (mandatory)

Security actions MUST be proportional to evidence strength:
- Tier 0 (weak signal): observation only
- Tier 1: monitoring + limited mitigations (rate limits)
- Tier 2: temporary suspension pending review
- Tier 3: confirmed exploitation → enforcement + incident response

### Anti-abuse + appeals (mandatory)

- False or malicious reports reduce reputation and may claw back rewards.
- Every high-impact action MUST have an appeal path.
- Rate limits on reporting and escalation scale with trust and reputation.

