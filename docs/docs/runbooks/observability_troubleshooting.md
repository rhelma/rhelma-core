# Observability Troubleshooting (Rhelma6)

This guide helps correlate logs, metrics, and traces across Rhelma6 services.

## Correlation identifiers

- `request_id` and `correlation_id` are Rhelma headers and should appear in logs.
- `traceparent` is the W3C trace context header used for distributed tracing.

## Find a failing request and follow it

1) Capture a failing request_id from gateway logs.
2) Search downstream service logs for the same correlation id.

Examples:

```bash
kubectl logs -n rhelma6 -l app=api-gateway --tail=500 | grep -i 'correlation'
kubectl logs -n rhelma6 -l app=node-registry --tail=500 | grep -i '<CORRELATION_ID>'
```

## Kafka event correlation

When Kafka publishing is enabled, events include:

- Rhelma envelope fields (request_id, correlation_id)
- Kafka headers (traceparent) when the publisher has an active trace context

If an event consumer is missing trace context:

- verify the producer is injecting headers
- verify the consumer extracts headers and attaches them to the current span
- check for intermediate components that drop Kafka headers

## Metrics checks

- confirm targets are being scraped (service discovery)
- validate label cardinality for newly added labels
- validate histogram buckets for latency metrics

## Log hygiene

- redact secrets
- avoid logging raw tokens and private keys
- prefer structured fields over embedding values into free-form strings
