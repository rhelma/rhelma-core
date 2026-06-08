# rhelma-tracing

Rhelma v6.0-compliant distributed tracing layer (RequestContext-ready).

## Contract

This component MUST comply with **Rhelma Contract v6.0**. See `docs/contract/v6.0/00_INDEX_v6.0.md`.

## Usage

Add as a dependency and follow the public API.

```toml
# In Cargo.toml
# [dependencies]
```

## Configuration

This crate reads the following environment variables (directly or via configs):

| Variable | Purpose |
|---|---|
| `HOSTNAME` |  |
| `TRACING_SAMPLING_RATE` |  |

## Security & Compliance

Normative requirements are in `docs/contract/v6.0/00_INDEX_v6.0.md`.

## Kafka Trace Propagation (feature: `kafka`)

If a service uses `rdkafka` directly (outside `rhelma-event-kafka`) and wants end-to-end
OpenTelemetry trace correlation across the event bus, enable the `kafka` feature and use
`rhelma_tracing::kafka_propagation::{inject_trace_context, extract_trace_context}` to inject/extract
W3C trace context headers (`traceparent`/`tracestate`) plus sanitized `baggage`.
