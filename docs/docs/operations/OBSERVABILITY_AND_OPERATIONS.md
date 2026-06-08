# Rhelma6 — Operations & Observability

**Status:** Draft (Roadmap)

Rhelma6 inherits the observability-first stance from v5.2, and extends it to nodes.

## 1) What we measure

- Node health: heartbeat freshness, CPU/GPU load, memory pressure
- Quality: task success, error categories, retry rates
- Latency: p50/p95/p99 per region and per role
- Cost: tokens, compute seconds, storage egress
- Security: failed attestations, policy violations, suspicious patterns

## 2) Logging & tracing

- Nodes SHOULD emit structured logs (JSON) compatible with Rhelma observability.
- Requests MUST propagate trace context when crossing nodes.

**Kafka correlation:** Rhelma events published to Kafka carry W3C `traceparent` headers.
If a service is built with `rhelma-tracing` feature `otel` and installs the
`tracing-opentelemetry` layer, Rhelma will mirror the active OTEL span IDs into its
local context so the Kafka `traceparent` matches what is exported to OTLP.

**Kafka consumer parentage (OTLP end-to-end):** If a consumer service enables
`rhelma-event-kafka` feature `otel`, its per-message `kafka.event` span will set the
upstream `traceparent` as the OTEL parent (via `tracing-opentelemetry`). This is
what makes traces appear as a single connected graph in Jaeger/Tempo/etc.

## 3) Auditing

- Privileged actions produce signed audit events:
  - node suspensions
  - governance policy publish
  - key rotations
  - builder executions

## 4) Incident response in a swarm

- Detect: watchdog signals + anomaly detection
- Triage: coordinator (or quorum) creates an incident id
- Contain: isolate nodes / reduce privileges
- Recover: re-route work, rotate keys
- Postmortem: publish sanitized incident summary

## 5) Degraded ops

During partitions:
- local routing continues with cached policy
- write operations queue and reconcile
- safety defaults to “deny / reduce privilege”

---

## Business spans (optional)

Some workflows benefit from *low-cardinality* business fields (e.g. tenant/user/subject ids, value amounts).
`rhelma-tracing` provides helpers:

- `rhelma_tracing::business::business_span("service.op", "operation_name")`
- `BusinessSpanExt` (`record_tenant_id`, `record_user_id`, `record_subject_id`, `record_value_amount`)

Standard field names:
- `rhelma_operation`
- `rhelma_tenant_id`
- `rhelma_user_id`
- `rhelma_subject_id`
- `rhelma_value_amount`

These are meant for dashboards and incident investigations; avoid high-cardinality values.

## W3C tracestate

When available, `tracestate` is propagated alongside `traceparent` for Kafka events and generic transport headers.
This improves interoperability with vendor tracers and preserves vendor routing hints.


## W3C baggage

`baggage` is an optional W3C header used for propagating small key/value pairs end-to-end.
In Rhelma, baggage propagation is **bounded and allowlisted** to reduce risk:

- Max header size: 2048 bytes
- Max items: 16
- Only these keys are forwarded:
  - `rhelma.operation`
  - `rhelma.tenant`
  - `rhelma.subject`
  - `rhelma.value.amount`

`rhelma-tracing::business` helpers automatically set these baggage keys for the current request/span.
Baggage is propagated through:

- HTTP/generic transport headers
- Kafka event headers (via `rhelma-event-kafka`)

If `rhelma-event-kafka` is built with the `otel` feature, producers will also read `baggage` from the current OpenTelemetry context using the W3C baggage propagator, then apply Rhelma sanitization (bounds + allowlist) before putting it on Kafka headers.

Consumers also apply the same Rhelma sanitization step when extracting OTEL baggage from incoming headers, so OTEL backends only see allowlisted, bounded baggage values.
