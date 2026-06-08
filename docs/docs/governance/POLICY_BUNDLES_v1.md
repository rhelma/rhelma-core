# Policy Bundles (schema + signing rules) (v1.0)

This document is **normative**.
It defines the canonical structure of a Policy Bundle and the exact signing/validation rules.

---

## 1) Goals

A Policy Bundle exists to:

- enforce operational rules (admission, routing caps, feature flags, security allow-lists),
- provide a verifiable governance trail (hash-linked, signed, auditable),
- avoid hidden “central admin” changes.

---

## 2) Canonical fields (v1.0)

Every Policy Bundle MUST include the following fields.

### 2.1 Identifiers

- `bundle_id` (UUID)
- `version` (string, e.g., `1.0`)
- `created_at` (RFC3339)
- `prev_bundle_hash` (base64url(sha256), or `null` for the first bundle)

### 2.2 Scope and class

- `class` (enum): `standard` | `high_impact` | `critical` | `emergency`
- `summary` (human-readable string, <= 4000 chars)

### 2.2.1 Optional activation time-lock

- `activate_not_before` (optional RFC3339): if present, the bundle MUST NOT be activated before this timestamp.

High-impact bundles MAY be configured (via env; see Section 3.4) to require a minimum activation delay.

### 2.3 Policy content

- `policy` (object): the operational policy payload.

The policy payload MUST be deterministic under canonical serialization (Section 4).

### 2.4 Signatures

- `signatures` (array) of:
  - `key_fpr` (fingerprint)
    - v1 supports `hs256:<kid>` (HMAC-SHA256 shared-secret councils)
    - v1 also supports `ed25519:<kid>` (Ed25519 public-key councils)
  - `sig` (base64url(signature_bytes))


### 2.4.1 Key configuration (informative)

Governance key sets are typically configured via environment variables.

**HS256 (shared secrets):**
- `RHELMA_GOVERNANCE__POLICY_COUNCIL_HMAC_KEYS`
- `RHELMA_GOVERNANCE__SECURITY_COUNCIL_HMAC_KEYS`
- `RHELMA_GOVERNANCE__HMAC_KEYS` (fallback)

**Ed25519 (public keys):**
- `RHELMA_GOVERNANCE__POLICY_COUNCIL_ED25519_PUBKEYS`
- `RHELMA_GOVERNANCE__SECURITY_COUNCIL_ED25519_PUBKEYS`
- `RHELMA_GOVERNANCE__ED25519_PUBKEYS` (fallback)

Ed25519 public key format: `kid:<base64url(pubkey32)>[,kid2:<base64url(pubkey32)>,...]`.

---

## 3) Quorum requirements (v1.0)

### 3.1 Standard

`class=standard` MUST have Policy Council quorum **3-of-5**.

### 3.2 High-impact

`class=high_impact` MUST have Policy Council quorum **4-of-5**.

`class=critical` MUST have **both**:

- Policy Council quorum **4** (default; can be increased via quorum mode/overrides)
- Security Council quorum **3** (default; can be increased via quorum mode/overrides)

Additionally, `class=critical` MUST include `activate_not_before` and it MUST be at least
`created_at + RHELMA_GOVERNANCE_CRITICAL_MIN_DELAY_SECONDS` (default: 86400 seconds / 24h).


High-impact classification is REQUIRED if the policy includes any of:

1) governance signer or threshold changes,
2) federation/value-settlement enable/disable,
3) slashing/punitive deductions authorization,
4) network-wide suspension,
5) breaking-change rollout instructions.

### 3.3 Emergency

`class=emergency` MUST have Security Council quorum **2-of-3**.

Additionally, `class=emergency` MUST include:

- `expires_at` (RFC3339, max 72 hours after `created_at`)
- `rollback_plan` (human-readable string)

Emergency bundles auto-expire and MUST be replaced by a standard/high-impact bundle to persist.

### 3.4 Optional dynamic quorum and timelock enforcement

Deployments MAY enable stricter governance constraints via environment variables:

- `RHELMA_GOVERNANCE_QUORUM_MODE`:
  - `fixed` (default): keep fixed class quorums (3/4/2)
  - `majority`: require a majority of the configured council size (but never less than fixed defaults)
  - `supermajority`: require 2/3 of the configured council size (but never less than fixed defaults)

- `RHELMA_GOVERNANCE_HIGH_IMPACT_MIN_DELAY_SECONDS`:
  - if set to a positive number, `class=high_impact` bundles MUST include `activate_not_before` and it must be at least `created_at + delay`.

These controls are designed to be **fail-open by default** and only tighten constraints when explicitly configured.

---

## 4) Canonical hashing and signing

### 4.1 Canonical serialization

Bundles MUST be serialized canonically before hashing/signing.

In v1.0, canonical serialization is defined as:

- UTF-8 JSON
- sorted object keys (lexicographic)
- no insignificant whitespace
- arrays preserved in order

### 4.2 Bundle hash

`bundle_hash = base64url( sha256( canonical_json(bundle_without_signatures) ) )`

The signature MUST be computed over `bundle_hash` bytes.

### 4.3 Validation rules

Nodes MUST reject a bundle if:

- it fails canonical hash verification,
- `prev_bundle_hash` does not match the locally accepted head (unless the node is explicitly syncing),
- quorum signatures are insufficient,
- signer fingerprints are not in the current council key set,
- `class=emergency` is past `expires_at`.

---

## 5) Distribution rules

1) A valid bundle MUST be recorded in the Governance Log.
2) Nodes SHOULD fetch bundles from multiple sources (at least 3 independent operators).
3) Nodes MUST support degraded-mode operation using the last valid bundle for up to **72 hours**.
4) After 72 hours without a new valid bundle, nodes MUST enter **Safe Mode** (reduced capabilities) until policy sync resumes.

---

## 6) Minimal Safe Mode (v1.0)

Safe Mode MUST:

- deny any high-privilege actions (patch apply, value settlement, governance writes),
- allow read-only and local-safe operations,
- keep audit logging enabled.

---

## 7) Examples

### 7.1 Minimal standard bundle (illustrative)

```json
{
  "bundle_id": "5a91f6d2-1a71-4d77-9f56-9dd0c1b3a9bb",
  "version": "1.0",
  "created_at": "2025-12-30T00:00:00Z",
  "prev_bundle_hash": "base64urlsha256...",
  "class": "standard",
  "summary": "Tighten anonymous admission throttles; cap single-operator routing share.",
  "policy": {
    "admission": {"pow_required": true, "max_registrations_per_hour": 50},
    "routing": {"max_operator_share": 0.25}
  },
  "signatures": [
    {"key_fpr": "hs256:c1", "sig": "..."}
  ]
}
```

This example omits additional signatures for brevity; production bundles MUST meet quorum.
