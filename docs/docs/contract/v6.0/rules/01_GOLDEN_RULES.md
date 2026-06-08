# Golden rules (Contract v6.0)

These rules are **normative**.

## 1) Security and trust boundaries

- Services **MUST** treat all inbound data as untrusted.
- Authentication/authorization decisions **MUST** be explicit and test-covered.
- Secrets **MUST NOT** be logged. Logs/metrics **MUST** be scrubbed for PII.

## 2) Observability is not optional

- Every request **MUST** have a `request_id` and must propagate context across HTTP/event boundaries.
- Metrics endpoints **MUST** be scrapeable and stable.
- High-cardinality labels **MUST** be clamped or rejected.

## 3) Contracts-first

- Public interfaces (HTTP APIs, events, wire formats) **MUST** be defined in `docs/contract/<version>/specs/`.
- Changes that affect a contract **MUST** be versioned and documented.

## 4) Eventing discipline

- Wildcard/regex subscriptions **MUST NOT** be used in production consumers.
- Topics **MUST** be allow-listed.

## 5) Reliability and change control

- Any rollout gate **MUST** have a rollback plan.
- CI **MUST** enforce formatting, lint, tests, and required checks.

## 6) Documentation policy

- Developer docs **MUST NOT** be organized by “phases”. Use domain-based structure (getting-started/architecture/operations/reference).
- Historical phase notes **MAY** be kept under `docs/archive/`.
