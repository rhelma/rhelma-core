# rhelma-observability-agent (v5.2)

An **enterprise-grade** observability agent for Rhelma services.

## What it does

- Heartbeats (`obs.heartbeat`)
- Reflex anomaly detection (`obs.signal` → insights/alerts/incidents)
- AI command execution (`ai.command.execute` → `ai.command.result`)
- AI incident decision application (`ai.incident.decision`)
- Internal counters (Prometheus text export)

## Reliability & safety guarantees

- **No Kafka single-point failure**: Kafka consumers run with **reconnect + exponential backoff**.
- **Backpressure for signals**: bounded in-memory queue for reflex signals, with drop-on-full **plus metrics**.
- **Serialized detector execution**: one worker processes signals sequentially to avoid detector state interleaving.
- **Cooperative shutdown**:
  - Runtime listens for **Ctrl+C** and cancels a shutdown token.
  - Command/decision loops **also** observe the same shutdown token (best-effort stop).
- **No wildcard topics**: subscription is allow-list only; `*` / regex prefixes are rejected.

## Admin endpoints (optional)

The agent can expose a small admin HTTP server for platform probes and scraping.

Enable by setting:

- `OBS_AGENT_ADMIN_ADDR=127.0.0.1:9090` (or any valid `ip:port`)
- Set `OBS_AGENT_ADMIN_ADDR=none` or unset it to disable (default: **disabled**)

Endpoints:

- `GET /healthz` (also `/readyz`, `/livez`)
- `GET /metrics` (Prometheus text format)

## Configuration

### Required identity (Rhelma contract)

- `RHELMA_SERVICE_NAME` (**required**)
- `RHELMA_ENVIRONMENT` (default: from `CentralEnv`)
- `RHELMA_REGION` (default: from `CentralEnv`)
- `RHELMA_SERVICE_VERSION` (default: from `CentralEnv`)

### Pipeline toggles

- `RHELMA_AGENT_ENABLE_COMMAND` (default: `true`)
- `RHELMA_AGENT_ENABLE_DECISION` (default: `true`)

### Signal (reflex) source

- `OBS_AGENT_SIGNAL_SOURCE` = `kafka|none` (default: `kafka`)
- `OBS_AGENT_SIGNAL_TOPICS` = `obs.signal,obs.signal.eu` (comma-separated allow-list)
- `OBS_AGENT_SIGNAL_GROUP` (default: `rhelma-agent-signals`)
- `OBS_AGENT_SIGNAL_QUEUE` (default: `1024`, max: `65536`)

### Command source

- `OBS_AGENT_COMMAND_SOURCE` = `kafka|nats|none` (default: `kafka` if enabled)
- `OBS_AGENT_COMMAND_TOPICS` (CSV allow-list) OR `OBS_AGENT_COMMAND_TOPIC` (single)
- `OBS_AGENT_COMMAND_GROUP` (default: `rhelma-agent-commands`)

NATS mode:

- `NATS_URL` (default: `nats://127.0.0.1:4222`)
- `OBS_AGENT_COMMAND_SUBJECT` (default: `ai.command.execute`)

### Decision source

- `OBS_AGENT_DECISION_TOPICS` (CSV allow-list) OR `OBS_AGENT_DECISION_TOPIC` (single)
- `OBS_AGENT_DECISION_GROUP` (default: `rhelma-agent-decisions`)

### Kafka shared

- `KAFKA_BOOTSTRAP_SERVERS` (default: `localhost:9092`)
- `OBS_AGENT_TOPIC_PREFIX` (optional topic prefix, must match the orchestrator)

## Monitoring

### Key metrics (from `/metrics`)

- `rhelma_obs_signal_received_total`
- `rhelma_obs_signal_dropped_total`
- `rhelma_obs_kafka_retry_total`

If `signal_dropped_total` is increasing:
- increase `OBS_AGENT_SIGNAL_QUEUE` (bounded) **or**
- reduce upstream signal rate **or**
- improve detector processing time (CPU/IO)

## Developer ergonomics

- Prefer `SignalPayload::new(...)` over struct literals in downstream code/tests.
  This keeps call sites stable when optional fields (like `incident_id`, `trace_id`, `span_id`) are added.

## Notes

- The agent initializes logger/tracing/metrics via `rhelma-observability-core` to avoid
  double-initialization and keep behavior consistent across services.
