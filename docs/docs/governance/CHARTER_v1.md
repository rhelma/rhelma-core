# Rhelma Realm Constitutional Charter (v1.0)

This Charter is **normative**.
It defines *constitutional governance* (power distribution, policy signing, dispute resolution) while keeping Rhelma Realm **decentralized-by-default**.

Technical protocols and interfaces remain versioned under `docs/contract/v6.0/`.

---

## 1) Mission

Rhelma Realm exists to build a **free, open, resilient, decentralized network** for intelligence + services:

- No single operator, organization, or service may be a required point of control.
- Governance must be **verifiable** (signed, versioned, auditable).
- Safety and privacy boundaries must be enforced without turning into centralized censorship.

---

## 2) Non‑negotiable principles

1. **Decentralization First**: the Network MUST survive the loss/compromise of any single entity.
2. **Verifiability Over Trust**: governance actions MUST be signed and recorded in an append-only log.
3. **Least Privilege**: privileged powers MUST be minimal, scoped, time-limited, and reviewable.
4. **Due Process**: severe actions MUST include evidence, an appeal path, and recorded reasoning.
5. **Consent & Data Rights**: training/knowledge workflows MUST respect data tiers, consent, and provenance.
6. **Right to Exit**: participants MAY leave and MAY fork open components.

---

## 3) Governance objects (no ambiguity)

### 3.1 Governance Log

The Network MUST maintain a **Governance Log** that is:

- append-only,
- content-addressed (hash-linked entries),
- and replicated (at least 3 independent operators).

All items below MUST be recorded in the Governance Log.

### 3.2 Policy Bundle

The Network is governed operationally via **Policy Bundles**:

- versioned,
- hash-linked (each bundle references a previous bundle),
- signed by the required quorum.

Canonical schema is defined in `docs/governance/POLICY_BUNDLES_v1.md`.

---

## 4) Governance bodies and quorum rules

### 4.1 Policy Council (operational governance)

**Purpose:** routine governance (admission policy, routing caps, rate limits, safe defaults, feature flags, protocol rollouts).

**Fixed size (v1.0):** **5 seats**.

**Quorum thresholds:**

- **Standard policy**: **3-of-5** signatures.
- **High-impact policy**: **4-of-5** signatures.

**High-impact policy** is any policy that:

1) changes governance thresholds or signer sets,
2) enables/disables federation or value settlement,
3) authorizes slashing/punitive deductions,
4) performs network-wide suspension of nodes/classes,
5) introduces a breaking change rollout plan.

**Term:** 180 days per seat.

**Rotation:** every 180 days, at least 2 seats MUST be eligible for rotation (anti-capture).

**Seat changes:** adding/removing seats or changing thresholds requires a **Constitutional Amendment** (Section 9).

### 4.2 Security Council (emergency response)

**Purpose:** fast, time-boxed mitigations.

**Fixed size (v1.0):** **3 seats**, selected from the Policy Council.

**Threshold:** **2-of-3** signatures.

**Term:** 90 days.

Security Council actions MUST be time-limited (Section 6).

---

## 5) Creator Stewardship (rights without central control)

The Creator is a **Steward**, not a daily ruler.

### 5.1 Permanent rights (always protected)

The Creator (and Successor, when active) ALWAYS has:

1) **Attribution Right**: recognized as Creator in official provenance and documentation.
2) **Advisory Right**: may publish official proposals (RFCs), critiques, and risk notices.
3) **Arbitration Trigger Right**: may trigger binding arbitration (Section 7).
4) **Succession Right**: may appoint a Successor via the Succession Record (Section 8).
5) **Emergency Vote Request**: may require the Security Council to vote on an emergency mitigation within 24 hours.

### 5.2 Creator prohibitions (hard limits)

Even the Creator/Succesor MUST NOT:

- unilaterally sign or impose standard policies (Policy Council quorum is required),
- permanently seize user assets/credits,
- permanently ban/suspend without due process + appeal,
- introduce breaking changes outside the amendment + rollout process.

---

## 6) Emergency powers (strictly limited and time-locked)

### 6.1 Emergency definition (v1.0)

“Emergency” is strictly limited to:

1) active critical exploit or intrusion,
2) key compromise or mass credential theft risk,
3) catastrophic network instability (widespread outage risk),
4) credible governance capture attempt.

### 6.2 Constitutional Safeguard Action (CSA)

The Creator/Succesor MAY issue a **CSA** only when all are true:

- the situation matches Section 6.1,
- the action is minimum necessary,
- the action is fully logged and signed,
- and it is time-limited.

**Default limits (v1.0):**

- CSA duration: **max 48 hours**.
- Security Council review: **within 24 hours**.

**Outcomes:**

1) **Ratify** (Security Council 2-of-3) → continues until the 48h limit.
2) **Modify/Rollback** (Security Council 2-of-3) → must be applied immediately.
3) **No decision within 24h** → CSA auto-expires at 48h and MUST be rolled back.

**Extension:**

- One extension MAY be issued by Security Council 2-of-3 for **up to 72 hours**.
- Any time beyond that requires Policy Council **4-of-5**.

Every CSA MUST include:

- scope (what is disabled/limited),
- justification,
- expected exit condition,
- and a postmortem deadline (Section 10).

---

## 7) Dispute resolution (binding, staged, auditable)

### 7.1 Scope

Disputes include:

- governance deadlocks,
- claims of abuse by councils,
- constitutional interpretation conflicts,
- disputes about CSA legitimacy.

### 7.2 Stages and deadlines (v1.0)

1) **Mediation** (max **7 days**) — handled by Policy Council.
2) **Jury Arbitration** (max **14 days**) — binding decision.
3) **Single Appeal** (optional, max **14 days**) — only with new evidence.

### 7.3 Jury composition and voting (v1.0)

**Jury Panel:** **7 members**

- 3 randomly sampled from a public “high-trust operator” list,
- 2 appointed by Policy Council (3-of-5 vote),
- 1 community-elected seat (election process is a Policy Bundle),
- 1 Creator-appointed seat.

**Decision threshold:** **5-of-7**.

**Appeal panel:** **9 members**, threshold **7-of-9**.

All decisions MUST be written, signed, and recorded in the Governance Log.

---

## 8) Succession (Creator → Successor)

Succession exists to preserve rights and continuity without centralizing control.

### 8.1 Succession Record

The Creator MAY publish a **Succession Record** (schema in `GENESIS_AND_SUCCESSION_v1.md`).

### 8.2 Activation (no ambiguity)

The Successor becomes **Active** for Charter powers only if one of the following occurs:

1) **Creator activation**: a signed `gov.succession.activate` entry by the Creator, countersigned by Policy Council **3-of-5**.
2) **Unreachability activation**: if **90 days** pass without any Governance Log entry signed by the Creator identity key, Policy Council **4-of-5** MAY activate the primary Successor.

Active status is recorded in the Governance Log.

---

## 9) Constitutional amendments (changing this Charter)

Any change to Sections **2–8** is a **Constitutional Amendment**.

Approval requirements (v1.0):

1) Proposal published with motivation + security analysis + rollout plan.
2) Public review window: **14 days**.
3) Policy Council approval: **4-of-5**.
4) If the Creator files a formal objection during review, arbitration MUST be triggered; the amendment proceeds only if arbitration upholds it.

---

## 10) Transparency and postmortems

1) All Policy Bundles MUST have a human-readable summary.
2) Severe actions (suspensions, slashing, CSA) MUST publish a redacted explanation.
3) Security incidents MUST publish a redacted postmortem within **30 days**, unless doing so increases risk.

---

## 11) Repository mapping (to avoid ambiguity)

- Charter (this file): `docs/governance/CHARTER_v1.md`
- Succession & genesis keys: `docs/governance/GENESIS_AND_SUCCESSION_v1.md`
- Policy Bundle schema: `docs/governance/POLICY_BUNDLES_v1.md`
- Technical contract/specs: `docs/contract/v6.0/`
