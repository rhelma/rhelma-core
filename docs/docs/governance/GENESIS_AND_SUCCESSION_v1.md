# Genesis, Identity Keys, and Succession (v1.0)

This document is **normative**.
It defines the Creator identity key, governance signer keys, and the exact mechanics for succession and emergency key handling.

---

## 1) Governance identities

### 1.1 Creator Identity Key (CIK)

The Creator is represented by a single **Creator Identity Key (CIK)**.

**Algorithm (v1.0):** Ed25519.

**Fingerprint format (v1.0):** `ed25519:<base64url(sha256(pubkey_bytes))>`.

The **CIK public key fingerprint MUST be recorded** in the Governance Log at genesis under `gov.genesis.record`.

### 1.2 Policy Council key set

The Policy Council is represented by **five** Ed25519 public keys.

Each council key MUST be recorded in the Governance Log under `gov.council.keys`.

**Quorum (v1.0):**

- Standard policy: 3-of-5
- High-impact policy: 4-of-5

### 1.3 Security Council key set

The Security Council is represented by **three** Ed25519 public keys.

In v1.0, Security Council keys MUST be a subset of the current Policy Council keys.

**Quorum (v1.0):** 2-of-3.

---

## 2) Governance Log genesis requirements

The genesis record (`gov.genesis.record`) MUST include:

1) Charter version and digest (SHA-256 of `docs/governance/CHARTER_v1.md`)
2) Contract version pointer (e.g., `docs/contract/v6.0/`)
3) CIK public key fingerprint
4) Policy Council key fingerprints (5)
5) Security Council key fingerprints (3)
6) A monotonic genesis timestamp

---

## 3) Key rotation (no ambiguity)

### 3.1 Policy Council key rotation

Council key changes MUST be recorded as `gov.council.rotate` entries.

**Approval (v1.0):** 4-of-5 Policy Council signatures.

**Constraints:**

- At least **3** of the 5 seats MUST change at most once per 180 days.
- A rotation MUST include a migration window (minimum 14 days) where both key sets are accepted.

### 3.2 Security Council key rotation

Security Council rotation MUST be recorded as `gov.security.rotate`.

**Approval (v1.0):** 3-of-5 Policy Council signatures.

### 3.3 Creator Identity Key rotation

Creator key rotation is rare and MUST be recorded as `gov.creator.rotate`.

**Normal rotation approval (v1.0):**

- Signed by the current CIK, AND
- countersigned by Policy Council 3-of-5.

---

## 4) Key compromise procedure

### 4.1 Emergency suspension of a compromised key

If there is credible evidence a governance key is compromised, the Security Council MAY issue a time-boxed suspension:

- Entry: `gov.key.suspend`
- Approval: Security Council 2-of-3
- Maximum duration: 72 hours

During suspension, policies signed by the suspended key MUST NOT count toward quorum.

### 4.2 Resolution

Within 72 hours, Policy Council MUST either:

1) **Confirm compromise and rotate** the affected key(s) (Policy Council 4-of-5), or
2) **Lift suspension** (Policy Council 3-of-5) with recorded justification.

If the suspended key was the CIK, arbitration MUST be triggered (see `CHARTER_v1.md` Section 7) within the same 72-hour window.

---

## 5) Succession

Succession exists to preserve the Creator’s rights and continuity **without** centralizing day-to-day governance.

### 5.1 Successor roles

There are two scopes of successor authority:

1) **Personal Stewardship Rights** (attribution, advisory representation)
2) **Charter Powers** (arbitration trigger, emergency CSA ability as defined in the Charter)

### 5.2 Succession Record

A Succession Record MUST be recorded in the Governance Log under `gov.succession.record`.

**Required fields (v1.0):**

- `record_id` (UUID)
- `created_at` (RFC3339)
- `primary_successor_pubkey_fpr` (fingerprint)
- `additional_successor_pubkey_fprs` (0..N)
- `scope` (`personal_only` or `charter_powers`)
- `notes` (human-readable, optional)

**Signatures:**

- For `personal_only`: CIK signature is sufficient.
- For `charter_powers`: CIK signature AND Policy Council 3-of-5 countersignatures are required.

### 5.3 Activation of a Successor

Activation MUST follow the Charter rules (Section 8 of `CHARTER_v1.md`).

Activation is recorded under `gov.succession.activate` or `gov.succession.activate_emergency`.

---

## 6) Deactivation / replacement of Successor

The Creator MAY replace a Successor at any time by publishing a new Succession Record.

If a Successor is credibly compromised or acting maliciously, Policy Council MAY deactivate the Successor:

- Entry: `gov.succession.deactivate`
- Approval: Policy Council 4-of-5
- Arbitration MUST be available upon request.

---

## 7) Implementation note

This file defines governance records as **log entries**. The concrete serialization (JSON, CBOR, etc.) may evolve, but the *fields and approvals above* MUST remain consistent until amended constitutionally.