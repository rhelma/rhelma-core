# Services quickstart
This page lists runnable services under `apps/` and how to start them locally.
- Full env list: `.env.example`
- Env conventions: `docs/reference/ENVIRONMENT_VARIABLES.md`
- Recommended gate: `./scripts/verify_pre_frontend.sh`
## Services
| Service | Type | Package | Run | Path |
|---|---|---|---|---|
| `agent-handoff` | rust | `agent-handoff` | `cargo run -p agent-handoff` | `apps/agent-handoff` |
| `ai-orchestrator` | rust | `ai-orchestrator` | `cargo run -p ai-orchestrator` | `apps/ai-orchestrator` |
| `api-gateway` | rust | `api-gateway` | `cargo run -p api-gateway` | `apps/api-gateway` |
| `bridge-adapter` | rust | `bridge-adapter` | `cargo run -p bridge-adapter` | `apps/bridge-adapter` |
| `control-service` | rust | `control-service` | `cargo run -p control-service` | `apps/control-service` |
| `digital-family-vault` | rust | `digital-family-vault` | `cargo run -p digital-family-vault` | `apps/digital-family-vault` |
| `edge-worker` | rust | `edge-worker` | `cargo run -p edge-worker` | `apps/edge-worker` |
| `file-storage` | rust | `file-storage-service` | `cargo run -p file-storage-service` | `apps/file-storage-service` |
| `gossip-discovery` | rust | `gossip-discovery` | `cargo run -p gossip-discovery` | `apps/gossip-discovery` |
| `guardian-agent` | rust | `guardian-agent` | `cargo run -p guardian-agent` | `apps/guardian-agent` |
| `rhelma-bridge-drivers` | rust-lib | `rhelma-bridge-drivers` | `cargo test -p rhelma-bridge-drivers` | `apps/rhelma-bridge-drivers` |
| `rhelma-node` | rust | `rhelma-node` | `cargo run -p rhelma-node` | `apps/rhelma-node` |
| `mni-rag` | rust | `mni-rag` | `cargo run -p mni-rag` | `apps/mni-rag` |
| `node-registry` | rust | `node-registry` | `cargo run -p node-registry` | `apps/node-registry` |
| `patch-applier` | rust | `patch-applier` | `cargo run -p patch-applier` | `apps/patch-applier` |
| `realtime-service` | rust | `realtime-service` | `cargo run -p realtime-service` | `apps/realtime-service` |
| `sandbox-runner` | rust | `sandbox-runner` | `cargo run -p sandbox-runner` | `apps/sandbox-runner` |
| `search-service` | rust | `search-service` | `cargo run -p search-service` | `apps/search-service` |
| `social-service` | rust | `social-service` | `cargo run -p social-service` | `apps/social-service` |
| `security-governance` | rust | `security-governance` | `cargo run -p security-governance` | `apps/security-governance` |
| `value-ledger` | rust | `value-ledger` | `cargo run -p value-ledger` | `apps/value-ledger` |
| `value-ledger-federation` | rust | `value-ledger-federation` | `cargo run -p value-ledger-federation` | `apps/value-ledger-federation` |
| `web` | node | `@rhelma/web` | `npm install && npm run dev` | `apps/web` |
