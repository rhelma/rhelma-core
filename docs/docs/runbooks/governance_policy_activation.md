# Governance Policy Activation (Rhelma6)

This runbook covers proposing, signing, and activating governance policy bundles, including quorum and time-lock.

## Concepts

- **Bundle ID**: unique identifier for a policy bundle.
- **Impact class**: indicates how strict quorum and activation rules should be.
- **Quorum mode**: determines how many signatures are required.
- **Time-lock**: an optional “not before” timestamp; the bundle cannot be activated before it.

## Endpoints (security-governance)

- `POST /v1/governance/propose`
- `POST /v1/governance/sign/:bundle_id`
- `POST /v1/governance/activate/:bundle_id`
- `GET  /v1/governance/pending`
- `GET  /v1/governance/pending/:bundle_id`
- `GET  /v1/governance/pending/:bundle_id/status`
- `GET  /v1/governance/active`
- `GET  /v1/governance/history`
- `GET  /v1/governance/council/status`

## Configuration references

### Quorum mode

`RHELMA_GOVERNANCE_QUORUM_MODE` controls the threshold behavior:

- `fixed` (default): uses the bundle’s explicit quorum parameters.
- `majority`: requires more than half of configured council keys.
- `supermajority`: requires a stronger threshold (for high-impact rollouts).

### Time-lock

- Bundles may include `activate_not_before` (RFC3339).
- `RHELMA_GOVERNANCE_HIGH_IMPACT_MIN_DELAY_SECONDS` can enforce a minimum delay for high-impact bundles.
- `RHELMA_GOVERNANCE_CRITICAL_MIN_DELAY_SECONDS` enforces a minimum delay for `class=critical` (default 24h when unset).

### Policy chain continuity

By default, `security-governance` enforces **chain continuity** on activation:

- If there is an active bundle, the pending bundle’s `prev_bundle_hash` must match the active bundle hash.
- If there is no active bundle (genesis), `prev_bundle_hash` must be `null`.

This prevents activating a bundle that was proposed against an old state.

You can disable this (not recommended) by setting:

- `RHELMA_SECURITY_GOVERNANCE_ENFORCE_POLICY_CHAIN=0`

On mismatch, activation will return **409 Conflict** with a message showing the expected and received `prev_bundle_hash`.

## Step-by-step procedure

### Step 1 — Propose a bundle

Example (high impact with a time-lock):

```bash
curl -s -X POST http://security-governance:8090/v1/governance/propose \
  -H 'content-type: application/json' \
  -d '{
    "summary": "Enable stricter admin RBAC",
    "class": "high_impact",
    "activate_not_before": "2026-01-06T10:00:00Z",
    "policy": {
      "rules": [
        {"id": "admin.readonly", "action": "allow", "resource": "admin:read"}
      ]
    }
  }' | jq .
```

Record:

- `bundle_id`
- `created_at` and `activate_not_before`
- the returned bundle hash (if present)

List pending bundles:

```bash
curl -s http://security-governance:8090/v1/governance/pending | jq .
```

### Step 2 — Collect signatures

Signatures are submitted by `key_fpr` (key fingerprint). Supported schemes:

- `hs256:<kid>` — server-side signing (legacy bootstrap)
- `ed25519:<kid>` — client supplies an Ed25519 signature (offline private key)

> **Critical bundles** require signatures from **BOTH** Policy and Security councils.
> A signature is counted for the council that contains its `key_fpr`.

#### HS256 signing (server-side)

```bash
curl -s -X POST http://security-governance:8090/v1/governance/sign/<BUNDLE_ID> \
  -H 'content-type: application/json' \
  -d '{
    "key_fpr": "hs256:policy-1"
  }' | jq .
```

#### Ed25519 signing (client-side)

1) Fetch the bundle hash to sign:

```bash
curl -s http://security-governance:8090/v1/governance/pending/<BUNDLE_ID> | jq -r .bundle_hash
```

2) Sign the decoded `bundle_hash` bytes with your Ed25519 private key, then submit the base64url signature:

```bash
curl -s -X POST http://security-governance:8090/v1/governance/sign/<BUNDLE_ID> \
  -H 'content-type: application/json' \
  -d '{
    "key_fpr": "ed25519:policy-2",
    "signature_b64url": "<BASE64URL_SIGNATURE_OVER_BUNDLE_HASH_BYTES>"
  }' | jq .
```

Check council key inventory:

```bash
curl -s http://security-governance:8090/v1/governance/council/status | jq .
```

### Step 3 — Confirm quorum is reached

Use the status endpoint to see quorum progress and timelock state:

```bash
curl -s http://security-governance:8090/v1/governance/pending/<BUNDLE_ID>/status | jq .
```

If quorum is not reached:

- verify the signer is in the correct council keyset
- verify the key id matches the configured key inventory
- verify signature encoding is base64url (not base64 with `+` and `/`)

### Step 4 — Activate

```bash
curl -s -X POST http://security-governance:8090/v1/governance/activate/<BUNDLE_ID>   -H 'content-type: application/json' | jq .
```

Activation is **serialized** to prevent races. If multiple activations are attempted at once, later requests may fail with **409 Conflict** due to chain continuity **or** an **active-head CAS conflict** (another instance advanced the active policy on disk). In either case, fetch the latest active bundle and re-propose/re-activate if needed.

Common activation failures:

- **Activation delay not met**: current time is before `activate_not_before`.
- **Insufficient quorum**: not enough valid signatures.
- **Invalid signature**: the submitted signature does not verify against the configured key.

### Step 5 — Validate the rollout

Validate that downstream services have picked up the new policy:

- check service logs for “policy loaded / policy updated”
- verify behavior with a small canary request set
- ensure error rate does not spike

## Rollback procedure

If a bundle causes unintended impact:

1) Propose a rollback bundle that restores the previous ruleset.
2) Collect signatures using the same quorum policy.
3) Activate the rollback bundle.
4) Verify behavior returns to expected state.

If the situation is severe:

- enable Safe Mode (if your deployment uses it) to block privileged actions temporarily
- restrict ingress to admin endpoints until the incident is contained

## Troubleshooting checklist

- Clock skew: compare node times; time-lock depends on UTC timestamps.
- Wrong council: verify the signature was submitted under the correct council type.
- Key mismatch: verify env vars that provide the HS256 secrets and Ed25519 public keys.
- Encoding: ensure base64url for Ed25519 signatures.

## Audit log (JSONL)

When `RHELMA_SECURITY_GOV__AUDIT_ENABLED=1`, every **mutating governance action** is appended to an
operational audit trail (one JSON object per line):

- default path: `<DATA_DIR>/governance_audit.jsonl`
- configurable with: `RHELMA_SECURITY_GOV__AUDIT_PATH`

Each event includes:
- `ts` (UTC), `action` (`propose|sign|activate`)
- `bundle_id` and (when available) `bundle_hash`
- best-effort request context (`request_id`, `correlation_id`, `traceparent`, `residency`)
- `result` (e.g. `ok`, `conflict`)

Quick queries:

```bash
# show latest entries
sudo tail -n 50 data/security-governance/governance_audit.jsonl | jq .

# list only activations
grep '"action":"activate"' data/security-governance/governance_audit.jsonl | tail -n 20 | jq .
```

Strict mode:
- if `RHELMA_SECURITY_GOV__AUDIT_STRICT=1`, failures to append to the audit log will fail the request
  (treated as part of the governance transaction).
