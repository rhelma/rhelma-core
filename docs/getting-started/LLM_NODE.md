# Local LLM Node (Phase 3)

The **LLM Node** is a small HTTP service you can run on the same machine as
`ai-orchestrator`. It keeps the prompt and response flow inside your network and
allows swapping in a true in-memory model runner later.

## What it does today

- Exposes `POST /v1/chat/completions`
- Accepts the same JSON schema as `ai-orchestrator`'s `ChatCompletionRequest`
- Returns a `ChatCompletionResponse`
- Default engine is a lightweight **stub** (useful for plumbing, tests, and
  offline dev)

## Run the node

From the repo root:

```bash
cargo run --manifest-path extras/llm-node/Cargo.toml
```

Environment variables:

- `RHELMA_LLM_NODE_LISTEN_ADDR` (default `127.0.0.1:8088`)
  - Back-compat: `RHELMA_LLM_NODE_ADDR` is also accepted.
- `RHELMA_LLM_NODE_LEXICON_PATH` (optional; if set, persists lessons/lexicon to a JSON file)
- `RHELMA_LLM_NODE_LEXICON_MAX_ENTRIES` (default: `2000`)
- `RHELMA_LLM_NODE_LEXICON_AUDIT_PATH` (optional; JSONL audit log)
- `RHELMA_LLM_NODE_LEXICON_BEARER_TOKEN` (optional; protects lexicon endpoints)
- `RHELMA_LLM_NODE_LEXICON_ATTESTATION_ENABLED` (default: `false`; signs lexicon meta using `RHELMA_AI_ATTESTATION__*` keys)


Health check:

```bash
curl -s http://127.0.0.1:8088/health
```

## Lessons -> "language" (Phase 49)

The node can ingest "lessons" (e.g. rollback-derived patterns) and use them as a small
**lexicon** to rewrite/normalize prompts before passing them to a real model.

Endpoints:

- `GET /v1/lexicon`
- `POST /v1/lexicon/entries`
- `POST /v1/lexicon/lessons`

Example ingest:

```bash
curl -s -X POST http://127.0.0.1:8088/v1/lexicon/lessons \
  -H 'content-type: application/json' \
  -d '{
    "proposal_id": "p-123",
    "lessons": [
      {"error_pattern": "RHELMA_LLM_NODE_ADDR", "optimized_fix": "RHELMA_LLM_NODE_LISTEN_ADDR", "tags": ["env"]}
    ]
  }'
```



## Bridge rollback lessons from ai-orchestrator (Phase 50)

If you want rollback-derived lessons to automatically flow into the node's lexicon,
enable the bridge in `ai-orchestrator`:

- `RHELMA_AI_ORCH__LEXICON_BRIDGE__ENABLED=true`
- `RHELMA_AI_ORCH__LEXICON_BRIDGE__BASE_URL=http://127.0.0.1:8088`
- `RHELMA_AI_ORCH__LEXICON_BRIDGE__TIMEOUT_MS=1500` (optional)
- `RHELMA_AI_ORCH__LEXICON_BRIDGE__MAX_RETRIES=2` (optional)
- `RHELMA_AI_ORCH__LEXICON_BRIDGE__BEARER_TOKEN=...` (optional)

## Wire it into ai-orchestrator

Set these variables for `ai-orchestrator`:

- `RHELMA_AI_ORCH__LEXICON_CONTEXT__ATTESTATION_REQUIRED=true` (optional; verify lexicon meta signatures)

- `RHELMA_AI_ORCH__LOCAL_NODE_URL=http://127.0.0.1:8088`
- `RHELMA_AI_ORCH__LOCAL_NODE_TIMEOUT_MS=1500`

And choose the provider:

- `RHELMA_AI_ORCH__PROVIDERS__PRIMARY=local-inmemory`
- `RHELMA_AI_ORCH__PROVIDERS__FAILOVER=mock` (optional)

## Next step (real in-memory inference)

The stub engine is intentionally tiny. The next phase can replace it with a real
model runner (e.g. Candle / GGUF / llama.cpp bindings) while keeping the same HTTP
contract.
