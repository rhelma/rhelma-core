# v5.2 Preview Migration Notes (Internal)

**Status:** Preview / internal only (as of rhelma-core v5.1.x)

This document describes how to evaluate and incrementally adopt the **v5.2 preview APIs**
(e.g., `RequestContextV52`, `ErrorEnvelopeV52`) without disrupting stable **v5.1** production
services.

---

## When to use v5.2 preview

Adopt v5.2 preview when you need **stricter boundary validation** than the v5.1 contract provides, for example:

- Gateways / edges that must fail-closed on malformed or missing identity and tracing headers.
- Internal services that want to standardize on canonical `x-rhelma-*` header names.
- Controlled environments where you can roll out changes gradually and monitor validation failures.

## When NOT to use v5.2 preview

Avoid v5.2 preview when:

- You are building public-facing services that must interoperate with diverse clients.
- You cannot coordinate a coordinated header rollout across all callers.
- You are not ready to treat validation failures as hard errors at the edge.

In these cases, keep using v5.1 `RequestContext` and adopt improvements incrementally.

---

## Compatibility summary

- **v5.1**: stable production contract.
- **v5.2 preview**: additional types + stricter validation helpers. Nothing changes until you opt in.

A practical way to think about it:

- v5.1 = “accept and normalize”
- v5.2 preview = “accept only canonical + strict”

---

## Recommended rollout plan

### Step 1 — Observe (no breaking change)

At the gateway, parse context as usual, but also run v5.2 preview validation in “observe mode”:

- Log validation failures (count them).
- Do **not** fail requests yet.
- Use this to detect client populations that would break under strict rules.

### Step 2 — Dual-propagate (internal only)

For internal traffic, propagate both:

- legacy headers (`x-request-id`, `x-correlation-id`, etc.), and
- canonical headers (`x-rhelma-request-id`, `x-rhelma-correlation-id`, etc.)

This keeps compatibility while you update downstream services.

### Step 3 — Enforce at the edge

Once metrics show low/no failures, switch external entrypoints to:

- enforce `RequestContextV52::validate_external(...)`,
- require canonical headers,
- return canonical Rhelma errors on validation failures.

### Step 4 — Consolidate

After adoption is complete:

- stop emitting legacy aliases,
- keep only canonical headers,
- keep v5.2 preview validators enabled for external entrypoints.

---

## Testing checklist

Before enforcement:

- Unit tests cover “missing header”, “invalid UUID”, “invalid traceparent”, and “residency mismatch”.
- Integration tests cover real HTTP requests through the gateway.
- Dashboards exist for validation failure rates.

After enforcement:

- Any spike in validation failures triggers rollback or temporary “observe mode”.

