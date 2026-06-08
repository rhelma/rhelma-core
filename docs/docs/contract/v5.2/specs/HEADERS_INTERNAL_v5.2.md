# Internal HTTP Headers (v5.2)

This document lists **non-public / internal** `x-rhelma-*` HTTP headers that appear in the Rhelma6 codebase.

These headers are **not part of the public tracing contract** (`x-rhelma-request-id`, `x-rhelma-correlation-id`, `traceparent`). They are used for:

- admin surfaces and privileged actions
- internal error envelopes
- edge ingress metadata
- routing / failover diagnostics

> Contract rule: If a header is intended for *external clients*, it must be documented in the main contract docs. The headers below are *internal* and should be stripped or ignored at public ingress unless explicitly permitted.

---

## Tracing headers (public)

These are the only **public** `x-rhelma-*` headers that must be accepted/propagated across services:

- `x-rhelma-request-id` — UUIDv7 request id (per-request)
- `x-rhelma-correlation-id` — UUIDv7 correlation id (multi-request)

W3C tracing:

- `traceparent`
- `tracestate` (optional)

---

## Admin surface headers (internal)

Used by admin APIs and development surfaces.

- `x-rhelma-admin-token` — bearer-like admin token for privileged routes
- `x-rhelma-admin-actor` — logical actor identifier (e.g. `admin-web`)
- `x-rhelma-actor` — non-admin actor hint (internal)

### Admin action attestation

For human-approved actions / governance attestations.

- `x-rhelma-admin-action-ts` — RFC3339 timestamp for the action
- `x-rhelma-admin-action-attestation` — single attestation payload
- `x-rhelma-admin-action-attestations` — multi-attestation payload (array)

---

## Error envelope headers (internal)

Used to annotate error responses for downstream mapping.

- `x-rhelma-error-type`
- `x-rhelma-error-envelope`

---

## Edge ingress metadata (internal)

Used at edge and ingress services for observability.

- `x-rhelma-ingress-service`
- `x-rhelma-ingress-env`
- `x-rhelma-ingress-version`
- `x-rhelma-surface` — indicates which surface served the request (`admin` vs `web`, etc.)

Optional client hint (mTLS/PKI):

- `x-rhelma-client-cert-sha256` — client certificate fingerprint

---

## Routing / failover diagnostics (internal)

Used for troubleshooting or multi-origin failover.

- `x-rhelma-upstream-attempt` — attempt counter during retry/failover
- `x-rhelma-upstream-region` — selected upstream region

---

## Reserved / under review

These headers appear in code but should be validated against the current contract:

- `x-rhelma-node-id` — node identity hint (should usually be derived from mTLS / registration)
- `x-rhelma-region` — prefer the canonical `x-region` header. This header is **deprecated** unless explicitly needed for a legacy surface.
- `x-rhelma-canary` — feature flag / canary hint (must not bypass authorization)

