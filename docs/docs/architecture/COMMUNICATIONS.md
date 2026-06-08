# Communication standards

This repo follows a **contract-driven interface** approach:

- **Normative rules/specs:** `docs/contract/v6.0/`
- **Practical developer docs:** `docs/`

## HTTP

MUST follow `docs/contract/v6.0/05_SECURITY_v6.0.md` and the service interface expectations in the contract.

Recommended defaults:

- **Health:** `GET /healthz`
- **Readiness:** `GET /readyz`
- **Metrics:** `GET /metrics` (Prometheus)
- **Tracing:** W3C Trace Context (`traceparent`) propagated across calls.
- **Correlation:** use a stable `request_id` (preferably a UUID) across service boundaries.
- **Errors:** sanitized, stable error codes; never leak secrets.

## Eventing (Kafka / NATS)

See `docs/contract/v6.0/04_EVENT_DRIVEN_v6.0.md` and `docs/contract/v6.0/specs/`.

Recommended defaults:

- Use explicit topic allow-lists (no regex subscriptions).
- Carry `request_id` / trace context in the event envelope.
- Version schemas; never break consumers without a contract bump.

## Compatibility

- If you change an interface (HTTP or event schema), update the relevant contract spec under `docs/contract/v6.0/specs/`.
- If a change is breaking, bump the contract version and keep the old version as compatibility docs.
