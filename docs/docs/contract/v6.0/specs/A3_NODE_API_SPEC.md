# Rhelma6 — Annex A3: Node Registry + Node HTTP APIs (Phase 1–5)

**Status:** Draft (operational, executable)

This annex defines the minimal HTTP interfaces required for the Rhelma6 bootstrap, routing, trust gating, and registry federation.

---

## 1) Node Registry API (Coordinator / Registry)

### 1.1 Register node

`POST /v1/nodes/register`

**Body (JSON)**
- `manifest`: `NodeManifestV1`
- `signature_hex`: hex-encoded Ed25519 signature over the canonical JSON of `manifest` (UTF-8 bytes)

`NodeManifestV1` (minimum fields)
- `node_id`: lower-hex Ed25519 public key (32 bytes)
- `region`: string
- `capabilities`: array of strings (e.g. `"llm.chat"`, `"search.batch"`)
- `allowed_residencies`: array of strings (e.g. `"EU"`, `"US"`, `"LOCAL"`)
- `data_url`: string (base URL for execution APIs)
- `control_url`: string (base URL for node control endpoints; can be same as data_url)
- `version`: string (node software version)
- `created_at`: RFC3339 timestamp

**Response (JSON)**
- `status`: `"active" | "pending" | "suspended"`
- `registry_time`: RFC3339
- `policy`: optional registry policy snapshot (min reputation, require attested)

### 1.2 Heartbeat

`POST /v1/nodes/heartbeat`

**Body (JSON)**
- `node_id`: string
- `at`: RFC3339
- `load`: optional object (cpu, mem, gpu, queue depth)
- `signature_hex`: optional signature over canonical JSON of the heartbeat body (recommended)

**Response (JSON)**
- `ok`: boolean
- `next_heartbeat_seconds`: integer

### 1.3 Attestation

`POST /v1/nodes/attest`

**Body (JSON)**
- `node_id`
- `attestation_kind`: string (`"software" | "hardware" | "custom"`)
- `attestation`: opaque JSON object
- `at`: RFC3339
- `signature_hex`: signature over canonical JSON of the attestation body

**Response**
- `ok`: boolean
- `attested`: boolean

### 1.4 Discover nodes (routing)

`GET /v1/nodes/discover?capability=llm.chat&region=eu&residency=EU&limit=20&require_attested=true&min_reputation=10`

**Response (JSON)**
- `nodes`: array of node summaries:
  - `node_id`
  - `data_url`
  - `control_url`
  - `region`
  - `capabilities`
  - `allowed_residencies`
  - `reputation`
  - `attested`
  - `status`
  - `last_seen_at`

### 1.5 Internal outcome reporting (reputation updates)

`POST /v1/internal/nodes/report`

**Auth**
- `x-registry-admin-token: <token>`

**Body (JSON)**
- `node_id`
- `outcome`: `"ok" | "fail" | "timeout" | "bad_result"`
- `at`: RFC3339
- `details`: optional object (latency_ms, task_kind, trace_id)

**Response**
- `ok`: boolean
- `reputation`: integer
- `status`: string

---

## 2) Federation API (Phase 5)

Federation enables multiple registries to replicate state without a single required coordinator.

### 2.1 Snapshot export

`GET /v1/federation/snapshot`

**Response (JSON)**
- `payload`: object
  - `registry_id`: string
  - `generated_at`: RFC3339
  - `nodes`: array of full node records (including trust fields)
  - `version`: string
- `signing_pubkey_hex`: lower-hex public key
- `signature_hex`: signature over canonical JSON of `payload`

### 2.2 Merge snapshot

`POST /v1/federation/push`

**Body**
- snapshot envelope returned by `/v1/federation/snapshot`

**Auth (recommended)**
- optional shared token (env-controlled), plus required signature verification

**Response**
- `ok`: boolean
- `merged_nodes`: integer

---

## 3) Node Execution API (Phase 3+)

The node exposes one of the following execution interfaces (minimum for Phase 3 is chat completions):

### 3.1 Chat completions

`POST /v1/chat/completions`

**Body**
- `request_context`: object (contract-aligned subset)
- `messages`: array
- `model`: optional
- `agent_lease_id`: optional (for sticky routing)

**Response**
- model output
- optional telemetry summary

### 3.2 Generic task execution (optional)

`POST /v1/tasks/execute`

**Body**
- `request_context`
- `task_kind`
- `payload`

**Response**
- `result`

---

## 4) Node Ops endpoints

- `GET /healthz` / `GET /readyz`
- `GET /metrics`

