# MNI Node Readiness

Status: **architecture / not implemented**. This document records what the docs
require for node-aware intelligence, the gaps in the Phase-1 MNI implementation,
and the minimal, backward-compatible changes to make it node-ready. It does not
change runtime behavior.

## TL;DR — premise correction

The docs do **not** ask for each node to invent its **own private language/model**.
Rhelma intelligence is **realm-native** (singular, governed, federated), not
node-native (plural, independent). "Node-aware MNI" in doc terms means:

1. **Specialization by expert/role** (MoE-lite: `code`, `security`, `governance`,
   `creative`, `ops`…), not per-node-instance languages.
2. **One shared, governed Lexicon** (realm-scoped, tiered), **replicated** across
   nodes — not independent per-node lexicons.
3. **Node↔node exchange via signed, content-hash-anchored artifacts and
   deterministic expert routing** — not learned embeddings or a novel protocol.

So "prepare for nodes to have their own internal language" resolves to: scope MNI
models by **(realm, expert)**, keep the Lexicon **shared**, and bridge to the
**already-specified** expert-routing + federated-delta protocols.

## 1. What the docs require

| Question | Doc answer | Source |
|---|---|---|
| Do individual nodes create their own language/model? | **No.** Intelligence is realm-native and composed from many nodes/experts; "NOT one big LLM in one place," but a single versioned/governed artifact. Nodes serve and *contribute*; they don't each train a private language. | `RHELMA_NATIVE_INTELLIGENCE.md:4-7,58-60`; `MNI_SERVING_MOE_LITE_v1.md` (expert = named capability class) |
| Per-node or global Lexicon? | **Global / realm-scoped, federated replication.** Data tiers (Public Commons / Consent / Sensitive-jury / Private-no-train) are realm-wide contracts. Phase 5 = "distributed lexicon replication." | `RHELMA_NATIVE_INTELLIGENCE.md:93-97,110`; `MNI_DATA_CONSTITUTION_V1.md:6-9` |
| How do nodes communicate this "language"? | **Structured signed events + content hashes**, routed by a **deterministic expert router** (tags/reputation/region/latency). Federated updates = `UpdateProposalV1` (delta_artifact_sha256) → `AcceptanceCertificateV1`. No embeddings/learned protocol on the wire. | `MNI_FEDERATED_FINETUNE_PROTOCOL_v1.md:9-24`; `MNI_EXPERT_ROUTING_POLICY_v1.md`; `COMMUNICATIONS.md:10-29` |

Roadmap ordering (all "node intelligence" sits on this): **Lexicon → Distillation →
Expert composition (MoE-lite) → Federated fine-tune (permissioned only)**. Expert
composition and federation are **Phase 6+ / deferred**.

## 2. Current state & gaps

**Substrate that already exists (real, wired):**
- Node identity & discovery: `node-registry` (`node_id`, `region`, `capabilities`,
  `tags`, `reputation`, `attestation`, `endpoints`) + `gossip-discovery` (signed
  heartbeats, weighted fanout, snapshot replication).
- Deterministic expert routing: `mni-rag/src/expert_router` (`ExpertProfile`,
  `Candidate`, `RoutingDecision` with reason codes).
- Node-serving helpers in the orchestrator: `mni/types.rs::RegistryNodeSummary`,
  `mni/mni_provider.rs::{pick_node,fallback_order}`, `OrchestratorRoutingTrace`.
- Federated contribution types (stub): `mni-rag/src/federated_finetuning`.

**Gaps in Phase-1 MNI (`ai-orchestrator/src/mni/`):**
- **Entirely global / singleton.** No `node_id`, `realm`, `expert`, or `scope`
  on `LexEntry`, `DatasetRecord`, `ModelVersionRecord`, `JobRecord`, `Store`,
  `MniService`, `TrainParams`, or any `/v1/mni/*` request.
- Model tags are a flat family (`rhelma-mni:vN`); two scopes would version-collide.
- **The Phase-38 routing helpers are disconnected** — `RegistryNodeSummary` /
  `MniServingProvider` are compiled but never called by the registry/trainer/query.
- Lexicon source is a single global URL; no realm/expert selection.

Net: the *node substrate* is rich and real; the *MNI layer* ignores all of it.

## 3. Target scoping model

Introduce one optional value object, defaulting to global:

```
Scope {
  realm:  Option<String>,   // realm-scoped Lexicon tier / governance boundary
  expert: Option<String>,   // MoE-lite capability class: code|security|gov|…
}
```

- **Lexicon stays shared.** `Scope.realm` only selects which realm-tier slice of the
  *one* Lexicon is used (Public Commons always; Consent/Sensitive per policy). No
  per-node lexicon store.
- **Models are scoped by (realm, expert)**, not by node instance. Model tag becomes
  `rhelma-mni-<expert>[-<realm>]:vN`. A node *serves* a scoped model; it does not own
  a private one.
- **Serving picks a node** via the existing expert router: `Scope.expert → intent_tags
  → RoutingDecision → RegistryNodeSummary.data_url`. This is where `mni_provider`
  finally gets wired.
- **Cross-node contribution** reuses `federated_finetuning::UpdateProposalV1`
  (delta + hashes + signature), gated by data tiers — unchanged on the wire.

## 4. Minimal, backward-compatible changes (when needed)

All additive (`Option`, `#[serde(default)]`) — no breaking changes, defaults = global.

| # | File | Change |
|---|------|--------|
| 1 | `mni/registry.rs` | Add `scope: Option<Scope>` to `DatasetRecord`, `ModelVersionRecord`, `JobRecord`; add `Scope` struct. |
| 2 | `mni/registry.rs` | `Store::next_version` keys on `(base_name, scope)`; model tag includes expert/realm. |
| 3 | `mni/mod.rs` | `TrainParams.scope`; `load_lexicon` filters realm tiers by `scope.realm`. |
| 4 | `mni/routes.rs` | Optional `scope` on `TrainRequest` / `QueryRequest`; filter `GET /models` by scope. |
| 5 | `mni/routes.rs` (query) | When `scope.expert` set and a remote node is chosen, route via `node_registry::chat_on_node` instead of local ollama (wires `mni_provider` + `expert_router`). |
| 6 | `mni-rag` | Promote `federated_finetuning` stub: accept `UpdateProposalV1` for a scoped track. (Phase 6+, deferred.) |

Steps **1–4** make the registry/training/query node-ready (data model + API) with
zero behavior change. Step **5** turns on node-to-node serving. Step **6** is the
deferred federation piece and should stay gated behind permissioned attestation per
`RHELMA_NATIVE_INTELLIGENCE.md:75`.

## 5. What stays out of scope (per docs)

- Per-node **private** lexicons or languages — contradicts realm-native design.
- Embedding/learned wire protocols between nodes — docs specify events + hashes.
- Unpermissioned federated training — gated until "verifiable training" exists.

## 6. Step 4 — node-to-node routing (prepared, disabled by default)

**Status: implemented behind a flag, off by default.** The query path is wired to
route to a node when enabled; with the flag off, behavior is byte-identical to
local Ollama serving.

Pieces (all committed):
- Config flag `mni_node_routing_enabled` (default `false`), env
  **`RHELMA_MNI_NODE_ROUTING_ENABLED`** — `apps/ai-orchestrator/src/config/mod.rs`.
- `AiOrchestrator::node_registry()` accessor — `src/orchestrator/mod.rs`.
- Guarded branch + `try_node_route()` / `node_to_summary()` in
  `src/mni/routes.rs` `query` handler. Wires the formerly-dead
  `mni_provider::{fallback_order, pick_node}` + `types::RegistryNodeSummary` +
  `node_registry::{discover, chat_on_node}`.

Behavior when enabled, per request:
1. Only engages if the request carries `scope.expert` (else local, unchanged).
2. `discover(expert)` against node-registry → candidate `NodeSummaryV1`s.
3. Map → `RegistryNodeSummary`; `fallback_order()` ranks; `pick_node()` selects.
4. `chat_on_node(chosen, req)` executes on the node's `data_url`.
5. **Graceful fallback to local Ollama** if no node advertises the expert, or the
   node call errors (logged at WARN). So enabling the flag alone changes nothing
   until a node actually advertises the expert.

### To enable ("go")
1. Ensure node-registry is on and reachable:
   `RHELMA_AI_ORCH__NODE_REGISTRY__ENABLED=true`,
   `RHELMA_AI_ORCH__NODE_REGISTRY__URL=http://<node-registry>:9010`.
2. Register ≥1 node whose manifest `capabilities` include the expert id
   (e.g. `code`, `security`) and a reachable `endpoints.data_url` serving
   `POST /v1/chat/completions`.
3. Flip the flag: `RHELMA_MNI_NODE_ROUTING_ENABLED=true`; restart ai-orchestrator.
4. Query with a scope: `POST /v1/mni/query {"prompt":"…","scope":{"expert":"code"}}`
   → routed to the node; without `scope.expert` it stays local.

### Known limitation (enrich later, non-blocking)
The orchestrator's discover client returns a trimmed `NodeSummaryV1` (no
reputation/attestation/routing_weight), so `node_to_summary()` defaults those and
ranking falls back to deterministic `node_id` order. When the full node summary
(or the mni-rag `expert_router` `RoutingDecision`) is wired, replace
`node_to_summary` + the `fallback_order` call with the richer signal — the call
sites are isolated in `try_node_route()`.
