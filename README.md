# Rhelma Platform

**Multi-tenant · Multi-region · AI-native**

Rhelma is a distributed platform built around zero-trust security, event-driven workflows (Kafka/NATS), observability-first primitives, and a self-improvement loop (proposal → evaluation → approval → apply).

This repository tracks **Contract v6.0** (v5.2 kept for compatibility).

---

## 📋 Table of Contents

- [Quick Start (TL;DR)](#-quick-start-tldr)
- [System Requirements](#-system-requirements)
- [First-time Setup](#️-first-time-setup)
- [Common Errors & Solutions](#-common-errors--solutions)
- [Quick Reference — Scripts](#-quick-reference--scripts)
- [Repository Layout](#-repository-layout)
- [Services Map](#️-services-map)
- [Hosted Access (`app.rhelma.app`)](#-hosted-access-apprhelmaapp)
- [Social Template (`rhelma.app`) — Commercial](#-social-template-rhelmaapp--commercial)
- [MNI — Rhelma Native Intelligence](#-mni--rhelma-native-intelligence)
- [Environment Variables](#-environment-variables)
- [Verification Gates](#-verification-gates)
- [Local Docker Stacks](#-local-docker-stacks)
- [Key Generation](#-key-generation)
- [Documentation Index](#-documentation-index)
- [Open-Source Model](#-open-source-model)
- [Changelog](#-changelog)

---

## 🚀 Quick Start (TL;DR)

```bash
# 1. Clone and enter the repo
git clone <repo-url> && cd rhelma-enterprise

# 2. Fix script permissions (Linux/macOS)
find scripts/ -name "*.sh" -exec chmod +x {} \;

# 3. Bootstrap (creates .env, generates Ed25519 JWT keys, checks tooling)
bash scripts/setup/bootstrap.sh

# 4. Open .env and paste the generated key values
#    (printed at the end of bootstrap, also in keys/jwt_private.b64 / jwt_public.b64)
nano .env   # or code .env

# 5. Start local infrastructure (Postgres, Redis, Qdrant, Meilisearch)
#    docker-compose.dev.yml uses standard ports (5432/6379/...) matching run-world.sh.
docker compose -f docker-compose.dev.yml up -d

# 6. Start all services
bash scripts/run-world.sh

# 7. Verify everything compiles and passes guards
bash scripts/verify_pre_frontend.sh
```

> ⚠️ If you get `Permission denied`, run step 2 first.
>
> **Windows:** replace `bash <script>.sh` with `.\scripts\<script>.ps1` throughout.

---

## 💻 System Requirements

| Component | Minimum | Recommended |
|---|---|---|
| CPU | 2 cores | 4+ cores |
| RAM | 4 GB | 8+ GB |
| Disk | 20 GB | 50+ GB |
| OS | Ubuntu 20.04+ / macOS 12+ | Ubuntu 22.04 LTS |
| Rust | 1.75+ | Latest stable |
| Docker | 24.0+ | Latest |
| Node.js | 20.x | 20.x LTS |

**Required tools:** `git`, `curl`, `openssl`, `pkg-config`, `build-essential`, `docker-compose`

---

## 🛠️ First-time Setup

### Step 1: Clone and enter the repo
```bash
git clone <repo-url> && cd rhelma-enterprise
```

### Step 2: Fix script permissions (Linux/macOS)
```bash
find scripts/ -name "*.sh" -exec chmod +x {} \;
```

### Step 3: Bootstrap
```bash
bash scripts/setup/bootstrap.sh
```
This script:
- ✅ Checks pre-requisites (preflight)
- ✅ Creates `.env` from `.env.example`
- ✅ Generates Ed25519 JWT keypair → `keys/`

Output:
```
keys/jwt_private.pem    ← keep secret, never commit
keys/jwt_public.pem
keys/jwt_private.b64    ← paste into RHELMA_AUTH_JWT_PRIVATE_KEY_B64
keys/jwt_public.b64     ← paste into RHELMA_AUTH_JWT_PUBLIC_KEY_B64
```

### Step 4: Verify pre-requisites
```bash
bash scripts/setup/preflight.sh
```
Expected output:
```
== Rhelma preflight ==
✅ git
✅ cargo
✅ rustfmt
✅ clippy
✅ docker
✅ openssl
✅ node
✅ npm
preflight: OK ✅
```
Common fixes if missing:
- Missing `clippy`: `rustup component add clippy`
- Missing `node`/`npm`: install Node.js 20+ from https://nodejs.org
- Missing `rg` (ripgrep): `sudo apt install ripgrep` (Linux) or `brew install ripgrep` (macOS)

### Step 5: Configure environment variables
```bash
nano .env   # or code .env
```
Minimum required variables:
```env
# Database
DATABASE_URL=postgres://rhelma_user:password@127.0.0.1:5432/rhelma_platform

# Redis
RHELMA_REDIS__URL=redis://127.0.0.1:6379/0

# JWT Keys (copy from keys/*.b64 after bootstrap)
RHELMA_AUTH_JWT_PRIVATE_KEY_B64=<paste from keys/jwt_private.b64>
RHELMA_AUTH_JWT_PUBLIC_KEY_B64=<paste from keys/jwt_public.b64>

# Optional: AI/MNI
RHELMA_AI_ORCH__OLLAMA_URL=http://localhost:11434
RHELMA_AI_ORCH__MNI_BASE_MODEL=qwen2.5:0.5b

# Fresh install only: create the first Super Admin after migrations
RHELMA_BOOTSTRAP_ADMIN_ENABLED=true
RHELMA_BOOTSTRAP_ADMIN_EMAIL=admin@example.com
RHELMA_BOOTSTRAP_ADMIN_PASSWORD=<set-a-strong-secret>
```
💡 Tip: `RHELMA_AUTH_JWT_PRIVATE_KEY_B64=$(cat keys/jwt_private.b64)`

> ⚠️ **Never commit `.env` or `keys/`** — both are in `.gitignore`.

The Super Admin bootstrap runs through `scripts/install.sh` after migrations. It
uses the existing auth password hasher, creates a private `platform-admin`
workspace identity by default, writes an audit record, and is safe to re-run
without duplicating or overwriting an existing Super Admin.

### Step 6: Start infrastructure containers
```bash
docker compose -f docker-compose.dev.yml up -d
```
Services started: Postgres, Redis, Qdrant, Meilisearch (standard ports, matching `run-world.sh`).

> ℹ️ `run-world.sh` itself does **not** require Docker — it only starts the lightweight
> realm stack (node-registry, gossip-discovery, realm-hub, ai-companion) plus the
> multi-frontend gateway. ai-companion treats Redis as optional and falls back to
> in-memory state if it isn't running. Start the infra above only when you also run
> the data-plane services (api-gateway, search-service, …) that require it.

### Step 7: Start all services
```bash
bash scripts/run-world.sh
```
Starts all Rust services via `cargo run`.

### Step 8: Start admin dashboard (optional)
```bash
bash scripts/run-admin-web-dev.sh
```

### Step 9: Verify everything
```bash
bash scripts/verify_pre_frontend.sh
```

---

## ❗ Common Errors & Solutions

| Error | Solution |
|---|---|
| `Permission denied: scripts/*.sh` | `find scripts/ -name "*.sh" -exec chmod +x {} \;` |
| `component 'clippy' is unavailable` | `rustup update && rustup component add clippy` |
| Docker permission denied | `sudo usermod -aG docker $USER && newgrp docker` |
| `node: command not found` | Install Node.js 20+ from https://nodejs.org |
| `DATABASE_URL` not set | Copy `.env.example` to `.env` and fill values |
| Port 3000 already in use | Change `RHELMA_API_GATEWAY__PORT` in `.env` |
| `cargo: command not found` | Install Rust: `curl --proto 'https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| `verify_all.sh` fails | Check permissions, run `bash scripts/verify.sh` first |
| `error: could not find system library 'ssl'` | `sudo apt install libssl-dev pkg-config` |
| `error: linker 'cc' not found` | `sudo apt install build-essential` |

<details>
<summary>Permission Denied — full fix</summary>

If you see:
```
Permission denied: scripts/check-structure.sh
```
Fix:
```bash
find scripts/ -name "*.sh" -exec chmod +x {} \;
```
</details>

<details>
<summary>Docker Permission — full fix</summary>

If you see:
```
Got permission denied while trying to connect to the Docker daemon socket
```
Fix:
```bash
sudo usermod -aG docker $USER
# Re-login or run:
newgrp docker
```
</details>

<details>
<summary>Rust Component — full fix</summary>

If you see:
```
⚠ Missing recommended command 'clippy'
```
Fix:
```bash
rustup component add clippy
# If error, update first:
rustup update && rustup component add clippy
```
</details>

---

## 📚 Quick Reference — Scripts

| Command | What it does |
|---------|-------------|
| `bash scripts/setup/bootstrap.sh` | First-time setup: preflight + `.env` + Ed25519 keys |
| `bash scripts/setup/generate-keys.sh` | (Re-)generate Ed25519 JWT keypair → `keys/` |
| `bash scripts/setup/preflight.sh` | Check required tools are installed |
| `bash scripts/run-world.sh` | Start all Rust services via `cargo run` |
| `bash scripts/run-isolated.sh` | Start only infra containers (Postgres/Redis/Qdrant/Meili) |
| `bash scripts/run-first-realm.sh` | Minimal stack: realm-hub + ai-companion only |
| `bash scripts/run-admin-web-dev.sh` | SvelteKit admin dashboard in dev mode |
| `bash scripts/verify_pre_frontend.sh` | Pre-frontend gate: fmt + clippy + tests + guards |
| `bash scripts/verify_all.sh` | Full verification suite |
| `bash scripts/verify.sh` | Rust-only: fmt + clippy + tests |
| `bash scripts/smoke_local.sh` | Smoke-test all services against `localhost` |
| `bash scripts/test_all.sh` | `cargo test --workspace` |
| `bash scripts/run-isolated.sh start\|stop\|status` | Full admin/observability stack (22 services) on the isolated port map |
| `./scripts/sync-nginx.sh [--check\|--diff]` | Sync the git nginx origin config → runtime + reload (validates first) |

### Single-service run

```bash
cargo run -p api-gateway
cargo run -p ai-orchestrator
cargo run -p search-service
cargo run -p realtime-service
cargo run -p node-registry
```

---

## 📁 Repository Layout

```
apps/            Runnable services (api-gateway, ai-orchestrator, …)
crates/          Reusable libraries (rhelma-core, rhelma-auth, …)
observability/   Tracing/metrics/logging wiring
extras/          Optional components (llm-node)
packages/sdk-go/   Go client SDK
packages/sdk-js/   JavaScript/ESM client SDK
packages/sdk-python/ Python async client SDK
docs/            Architecture, contracts, runbooks, getting-started guides
infra/           Kubernetes manifests, Helm charts, Prometheus, Terraform
deploy/          Docker Compose stacks for local / production
scripts/         Dev tooling, guards, smoke tests
  setup/         First-time setup (bootstrap, generate-keys, preflight)
  dev/           Day-to-day helpers (run-world, run-first-realm, …)
  guards/        Contract / env / event / metrics guard scripts
  rhelma6/       Phase-specific E2E, chaos, and load test scripts
```

---

## 🗺️ Services Map

| Service | Default port | Purpose |
|---------|-------------|---------|
| `api-gateway` | 3000 | Public HTTP entry point, rate limiting, routing |
| `ai-orchestrator` | 4000 | AI workflow engine, improvement loop |
| `search-service` | 8082 | Hybrid semantic + keyword search |
| `realtime-service` | 9000 | WebSocket rooms and pub/sub |
| `file-storage` | 3005 | Upload / serve files (local or S3) |
| `api-gateway /api/v1/media` | 3000 | Canonical public media entrypoint; proxies to file-storage-service |
| `social-service` | 8085 | Tenant-aware posts/comments/reactions/feed + moderation queue (RLS-isolated) |
| `license-service` | 8090 | Social-template billing/licensing (Stripe → tenant provisioning); commercial, see boundary |
| `control-service` | 8086 | Tenant control plane (provisioning) |
| `node-registry` | 8090 (k8s: varies) | Peer node registration + attestation |
| `security-governance` | 8091 | Quorum-signed policy bundles |
| `gossip-discovery` | 8092 | Peer-to-peer gossip sync |
| `agent-handoff` | 8093 | Signed agent delegation tokens |
| `bridge-adapter` | 8094 | Cross-chain value bridge |
| `digital-family-vault` | 8095 | Time-locked vault + recovery workflows |
| `mni-rag` | 8096 | Data-sovereignty RAG + signed lineage |
| `region-health-aggregator` | 8097 | Multi-region health + failover events |
| `value-ledger-federation` | 8098 | Replicated credit ledger |
| `realm-hub` | 9110 | First-Realm event hub |
| `ai-companion` | 9120 | Realm AI assistant |
| `multi-frontend` | 8080 | Svelte frontend + admin dashboard proxy |
| `observability-agent` | 9090 (opt-in) | Reflex/anomaly agent; `/healthz` + `/metrics` admin server (enable via `OBS_AGENT_ADMIN_ADDR`) |

---

## 🌐 Hosted Access (`app.rhelma.app`)

The platform is fronted by nginx (behind Cloudflare). `multi-frontend` (`:8080`)
is the single entry point — it serves the landing page, the `/admin` console, and
its own allowlisted `/api/*` proxies to the backend services.

| URL | Serves |
|-----|--------|
| `https://app.rhelma.app/` | Landing portal (static, from `apps/web/build/`) |
| `https://app.rhelma.app/admin` | Admin console (Overview · Status · Realm · AI · Metrics) |
| `https://app.rhelma.app/api/{realm,registry,ai,governance,observability,…}/…` | Allowlisted, app-authenticated backend proxies |

**Direct backend access (ops/debug)** is exposed under `/_svc/<name>/` and
protected by HTTP Basic Auth:

| Prefix | Backend | Example |
|--------|---------|---------|
| `/_svc/registry/` | node-registry (`:9010`) | `/_svc/registry/healthz` |
| `/_svc/gossip/` | gossip-discovery (`:9020`) | `/_svc/gossip/healthz` |
| `/_svc/realm/` | realm-hub (`:9110`) | `/_svc/realm/v1/realms/realm_first/manifest` |
| `/_svc/ai/` | ai-companion (`:9120`) | `/_svc/ai/healthz` |

### nginx / TLS

Two nginx configs live in `config/nginx/` — **they are different deployment targets, not copies**
(see `config/nginx/README.md`):

| Config | Target | Certs | Admin SPA served by |
|--------|--------|-------|---------------------|
| `app.rhelma.app.conf` (+ `rhelma.app.conf`, `api.rhelma.app.conf`) | Production reference (`sites-available` style) | Let's Encrypt | proxies `/admin/` → `multi-frontend` `:8080` |
| `nginx.local.conf` | **Runnable origin config for THIS host** (behind Cloudflare) | self-signed origin cert in `/opt/rhelma/nginx/certs` | serves the SvelteKit build **statically** from `apps/admin-web/build` |

**The live host runs `nginx.local.conf`** as a manually launched master (not `systemctl nginx`,
which is a separate/failed unit that can't bind :80/:443 while this one holds them):

```bash
nginx -p /opt/rhelma/nginx -c /opt/rhelma/nginx/nginx.local.conf   # (start; already running here)
```

**Editing nginx → one command.** `config/nginx/nginx.local.conf` (in git) is the source of truth;
the runtime copy at `/opt/rhelma/nginx/nginx.local.conf` must match it. After editing the repo file:

```bash
./scripts/sync-nginx.sh            # validate repo config → copy to runtime → reload (no downtime)
./scripts/sync-nginx.sh --check    # validate only (no copy, no reload)
./scripts/sync-nginx.sh --diff     # show repo-vs-runtime drift
```

The script validates **before** copying, so a broken config never lands on disk or reloads.

- Production references use: shared proxy headers `/etc/nginx/snippets/rhelma_proxy.conf`;
  basic-auth users `/etc/nginx/rhelma_admin.htpasswd` (rotate with
  `htpasswd -B /etc/nginx/rhelma_admin.htpasswd <user>` then `nginx -s reload`); Let's Encrypt
  origin TLS (`certbot`, auto-renewing) with Cloudflare SSL mode **Full (Strict)**.
- **Static admin SPA cache (in `nginx.local.conf`):** `Cache-Control: immutable` is scoped to the
  content-hashed `/_app/immutable/` subtree ONLY; `/_app/version.json` and `index.html` are
  `no-cache`. This matters — if `version.json` is cached immutable, a browser on an old build never
  learns it is stale, its lazy chunk imports 404 after a redeploy, and client-side nav silently
  breaks. On a live Cloudflare deploy, also **purge the CF cache for `/admin/app/*`** after a rebuild.
- Certs under `/opt/rhelma/nginx/certs` are host-local and **not** committed.
- **Admin app (`/admin/`)** — two modes:
  - *Production:* `multi-frontend` (`:8080`) serves the built SPA from
    `RHELMA_ADMIN_WEB_DIST_DIR` (default `apps/admin-web/build`); no extra nginx block needed.
  - *Dev:* point `/admin/` at the admin-web Vite server (`:3001`) so changes hot-reload:
    ```nginx
    location /admin/ {
        proxy_pass http://127.0.0.1:3001/admin/;
        proxy_set_header Host localhost:3001;   # Vite's host-check 403s app.rhelma.app
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;  # HMR websocket
        proxy_set_header Connection $rhelma_connection_upgrade;
    }
    ```
    Start the dev server with `bash scripts/run-admin-web-dev.sh`. The app lives under the
    `/admin/app` base path; data APIs (`/admin/api/*`) are only served by `:8080`, so add a
    matching `location /admin/api/ { proxy_pass http://127.0.0.1:8080; }` if you need live data.

Initial Super Admin: on fresh installs, set `RHELMA_BOOTSTRAP_ADMIN_ENABLED=true`,
`RHELMA_BOOTSTRAP_ADMIN_EMAIL`, and `RHELMA_BOOTSTRAP_ADMIN_PASSWORD` in `.env`
before running `bash scripts/install.sh`. Then sign in at `/admin/login` and use
the admin panel to create additional admins. The bootstrap script never logs the
password and does not overwrite an existing Super Admin.

> **Security notes:** restrict the origin firewall to Cloudflare IP ranges; to use
> IP allowlists on `/_svc/*` or `/admin`, first restore the real client IP
> (`set_real_ip_from <CF ranges>; real_ip_header CF-Connecting-IP;`). The most
> hardened option for the control-plane services (`node-registry`,
> `gossip-discovery`) is an SSH tunnel instead of public `/_svc/*` exposure.

### Reproducing this host / resuming work

Everything needed to bring the hosted admin stack back up is in git. From a clean checkout on the
same host layout:

```bash
# 1. Backend services — full 22-service admin/observability stack on the isolated port map.
#    (Starts multi-frontend :8080, mni-rag :8096 with an empty admin token, security-governance
#     tokenless, etc. — the open-dev posture the admin console expects.)
bash scripts/run-isolated.sh start

# 2. Admin SPA — build the static bundle nginx serves from apps/admin-web/build.
cd apps/admin-web && npm ci && npm run build && cd -

# 3. nginx origin config — copy git → runtime, validate, reload (see config/nginx/README.md).
./scripts/sync-nginx.sh
```

Verify green before/after changes:

```bash
cargo test -p multi-frontend                          # 22 pass (incl. MNI RBAC)
cd apps/admin-web && npm run check && npm run build    # 0 errors + build ok
```

**Not captured by a `git checkout`, re-apply manually on a fresh host:**
- **nginx runtime file** — `git` holds `config/nginx/nginx.local.conf`; `./scripts/sync-nginx.sh`
  installs it to `/opt/rhelma/nginx/nginx.local.conf`. The runtime path is outside the repo tree.
- **Host-local certs** — `/opt/rhelma/nginx/certs` (self-signed origin cert) are not committed; on
  a real deploy swap for a Cloudflare Origin CA cert at the same paths (SSL mode Full (Strict)).
- **Running processes** — the stack is a set of `run-isolated.sh`-managed processes (pidfiles in
  `/tmp/ent-logs/`), not a systemd service; re-run `run-isolated.sh start` after a reboot.
- **`.env`** — generated by `scripts/setup/bootstrap.sh` (secrets/JWT keys), gitignored.

> **Point-in-time resume:** the session's completed state is tagged **`checkpoint/2026-07-02-mni`**
> and summarized in the **CHECKPOINT block at the top of `PROGRESS.md`** (resume/re-verify commands,
> what's live, open follow-ups). `git checkout checkpoint/2026-07-02-mni` lands on exactly that point.

---

## 🛒 Social Template (`rhelma.app`) — Commercial

The **hosted social template** lets a customer buy a provisioned social instance.
The customer site `rhelma.app` (React/Vite, `/var/www/rhelma.app` — distinct from
the admin `app.rhelma.app`) provides pricing + the tenant dashboard:

| URL | Serves |
|-----|--------|
| `rhelma.app/social` | Plan pricing → `/checkout` → Stripe |
| `rhelma.app/panel/social` | **Social Instance** dashboard tab (Overview · Settings · Moderation · Analytics) inside the existing user panel |

Flow: a purchase calls `license-service` (via the gateway's `/billing/*` proxy),
which on Stripe `checkout.session.completed` creates a `licenses` row, derives a
tenant, and provisions it through `control-service`. The shared `social-service`
then serves that tenant by `x-tenant-id`, isolated by row-level security
(`010_social_rls.sql`). No per-customer containers.

- Billing/licensing implementation is **commercial** — excluded from public
  release per `COMMERCIAL_BOUNDARY.md` / `OPEN_SOURCE_MANIFEST.md`.
- Service details: `apps/license-service/README.md`. Site map: `docs/sites/rhelma-app.md`.
- Bringing it online on a live host (migrations 009–012, RLS, restarts) and the
  **routing + auth gaps** that still block the public path:
  `docs/runbooks/social_licensing_rollout.md`.

---

## 🧠 MNI — Rhelma Native Intelligence

MNI turns the platform's **Lexicon** (short-hand rules the system learns from its own
failures) into **custom models**. Phase 1 runs entirely on a small CPU box using
[Ollama](https://ollama.com) as the model backend — no GPU, no Python ML stack.

Pipeline: `Lexicon → SDG (orchestrator LLM) → Modelfile-bake → ollama create → registry → serve`.
It is **Modelfile-bake** (a Lexicon-distilled system prompt + SDG few-shot examples), not
weight fine-tuning; the trainer is a pluggable job so a real LoRA backend can drop in on a
bigger box. Lives in `ai-orchestrator` (`src/mni/`, `src/providers/ollama.rs`).

**Prerequisites:** Ollama running (`ollama serve`) and a small base model pulled
(`ollama pull qwen2.5:0.5b`).

**API** (on `ai-orchestrator`, reverse-proxied at `/api/orchestrator/v1/mni/*`):

| Method | Path | Purpose |
|--------|------|---------|
| `POST` | `/v1/mni/train` | Generate SDG data from the Lexicon and bake a new model version → `{job_id, model_name}` |
| `GET`  | `/v1/mni/train/{job_id}` | Poll job status (`queued → generating_data → building_model → ready`) |
| `GET`  | `/v1/mni/models` | List registered custom model versions |
| `POST` | `/v1/mni/models/activate` | Mark a version active |
| `POST` | `/v1/mni/query` | Inference against a model (custom or base) |
| `GET`  | `/v1/mni/lexicon` | Show the resolved Lexicon (source + digest + entries) |

```bash
# create the first custom model from the Lexicon
curl -sX POST localhost:9000/v1/mni/train -H 'content-type: application/json' \
  -d '{"name":"rhelma-mni","per_entry":1,"fewshot":6}'
curl -s localhost:9000/v1/mni/train/<job_id>        # poll until "ready"
curl -sX POST localhost:9000/v1/mni/query -H 'content-type: application/json' \
  -d '{"prompt":"What does Rhelma say about an unresolved import?"}'
```

**Admin UI:** **MNI Studio** (`/admin/app/mni`) — view the Lexicon, trigger training,
watch jobs, browse the model registry, chat-test a model, and compare base vs custom.
The Overview dashboard shows an MNI status widget with a one-click train action.

Lexicon staging/promotion (separate, llm-node-backed) lives at `/admin/app/dev/lexicon`.

---

## 🔧 Environment Variables

Start from `.env.example`. Minimum required for a working local stack:

```bash
DATABASE_URL=postgres://rhelma_user:password@127.0.0.1:5432/rhelma_platform
RHELMA_REDIS__URL=redis://127.0.0.1:6379/0
RHELMA_AUTH_JWT_PRIVATE_KEY_B64=<from keys/jwt_private.b64>
RHELMA_AUTH_JWT_PUBLIC_KEY_B64=<from keys/jwt_public.b64>
```

MNI (ai-orchestrator) — optional, defaults shown:

```bash
RHELMA_AI_ORCH__OLLAMA_URL=http://localhost:11434   # Ollama backend (provider "ollama")
RHELMA_AI_ORCH__MNI_BASE_MODEL=qwen2.5:0.5b         # FROM base for generated Modelfiles
RHELMA_AI_ORCH__MNI_DATA_DIR=data/mni               # local registry, datasets, Modelfiles
```

Full reference: `docs/reference/ENVIRONMENT_VARIABLES.md`

---

## ✅ Verification Gates

```bash
# Fastest feedback loop (recommended before every commit)
bash scripts/verify_pre_frontend.sh

# Full suite (run in CI or before a release)
bash scripts/verify_all.sh

# Individual guards
bash scripts/guards/contract_guard.sh
bash scripts/guards/uuidv7_guard.sh
bash scripts/guards/todo_guard.sh
```

---

## 🐳 Local Docker Stacks

```bash
# Dev infra (Postgres, Redis, Qdrant, Meilisearch on standard ports) — pairs with run-world.sh
docker compose -f docker-compose.dev.yml up -d

# Isolated infra on non-standard ports (15432/16379/16333/17700) — pairs with run-isolated.sh,
# for running alongside another live stack without port collisions.
docker compose -f docker-compose.isolated.yml up -d

# Full Rhelma6 stack (services built via cargo run)
docker compose -f deploy/rhelma6/docker/docker-compose.rhelma6.yml up
```

> ℹ️ Use `docker-compose.dev.yml` for everyday local dev (standard ports, pairs with
> `run-world.sh`). Use `docker-compose.isolated.yml` only when you need to run a second
> stack alongside a live one — it remaps every host port and uses distinct container
> names so the two never collide.

---

## 🔑 Key Generation

```bash
# Generate a fresh Ed25519 keypair (writes to ./keys/)
bash scripts/setup/generate-keys.sh

# Output:
#   keys/jwt_private.pem    ← keep secret, never commit
#   keys/jwt_public.pem
#   keys/jwt_private.b64    ← paste into RHELMA_AUTH_JWT_PRIVATE_KEY_B64
#   keys/jwt_public.b64     ← paste into RHELMA_AUTH_JWT_PUBLIC_KEY_B64
```

> ⚠️ **Never commit `.env` or `keys/`** — both are in `.gitignore`.
> If you accidentally committed them: `git filter-repo --path .env --invert-paths && git push --force-with-lease`

---

## 📖 Documentation Index

- **Getting started:** `docs/getting-started/QUICKSTART_MVP.md`
- **Architecture:** `docs/architecture/OVERVIEW_RHELMA6.md`
- **Workspace identity (User owns Workspace; Tenant = isolation only):** `docs/architecture/WORKSPACE_IDENTITY.md`
- **Contract v6.0:** `docs/contract/v6.0/00_INDEX_v6.0.md`
- **Environment vars:** `docs/reference/ENVIRONMENT_VARIABLES.md`
- **Runbooks:** `docs/runbooks/`
- **Open-source model:** `docs/open-source/README.md`
- **Release checklist:** `docs/open-source/RELEASE_CHECKLIST.md`
- **ROADMAP:** `ROADMAP.md`

---

## 🤝 Open-Source Model

Rhelma follows an open-core model:

- **Public core** — reusable Rust crates, service contracts, local dev tooling, SDKs, docs
- **Social product** — Asrnegar (operational social system) built on the public core
- **Commercial layer** — customer deployments, billing, hosted ops, enterprise support

Before a public release: `docs/open-source/RELEASE_CHECKLIST.md`.

---

## 📝 Changelog

- **v6.0** — Contract v6.0, MNI (Rhelma Native Intelligence), enhanced social template
- **v5.2** — Legacy version (kept for compatibility)

---

## 📄 License

See `LICENSE` file for details.

---

## 💬 Support

- **Documentation:** `docs/`
- **Issues:** GitHub Issues
- **Community:** Join our Discord

---

<p align="center">Built with ❤️ by the Rhelma Team</p>

## Monorepo structure note

The canonical service/package structure is documented in `docs/architecture/PROJECT_STRUCTURE.md`.
Key conventions:

- `apps/api-gateway` is the public API entrypoint.
- `apps/file-storage-service` is the canonical file storage path.
- SDKs live under `packages/sdk-js`, `packages/sdk-python`, and `packages/sdk-go`.
- Public API/event/RLS contracts live under `contracts/`.
- Root frontend workspace commands are available through `pnpm`.

Run `bash scripts/check-structure.sh` and `bash scripts/guards/env_example_sync_guard.sh .` after structural changes.


## Codex-assisted testing loop

After structural changes, run:

```bash
bash scripts/dev/codex-test-loop.sh .
```

Use the generated `.codex-test-logs/` file as the focused context for Codex. New browser/API clients should target api-gateway versioned routes (`/api/v1/social`, `/api/v1/search`, `/api/v1/media`) while legacy unversioned routes remain available during migration.
