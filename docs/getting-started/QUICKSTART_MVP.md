# Quickstart (MVP)

This guide gets you from **zero → running** with the smallest set of steps.

## 1) Bootstrap

Linux/macOS/WSL:

```bash
bash scripts/bootstrap.sh
```

Windows (PowerShell):

```powershell
.\scripts\bootstrap.ps1
```

Bootstrap will:

- Check basic tooling (best-effort)
- Create `.env` from `.env.example` (if missing)
- Generate dev RSA keys into `./keys` (if missing)

## 2) Run the local "world" stack

This starts the **First Realm** stack + the **multi-frontend gateway**:

```bash
bash scripts/run-world.sh
```

Endpoints:

- Web: `http://localhost:8080/`
- Admin (Rust): `http://localhost:8080/admin`
- Admin Web (Svelte): `http://localhost:8080/admin/app`

> Tip: Start the admin-web dev server (hot reload) with:
>
> ```bash
> bash scripts/run-admin-web-dev.sh
> ```

## 3) Smoke checks

Linux/macOS/WSL:

```bash
bash scripts/smoke_local.sh
```

Optional: exercise a minimal **auth flow** (requires Postgres + migrations):

```bash
RHELMA_SMOKE_AUTH_FLOW=1 RHELMA_SMOKE_TENANT_ID=local bash scripts/smoke_local.sh
```

Windows:

```powershell
.\scripts\smoke_local.ps1
```

## 4) Verification gates

Recommended (fast + deterministic):

```bash
bash scripts/verify_all.sh
```

If you don't have Node installed yet (skip frontend checks):

```bash
bash scripts/verify_pre_frontend.sh
```

## Troubleshooting

- If `openssl` is missing, install it and re-run `scripts/bootstrap.*`.
- If ports are busy, update `.env` (or stop the conflicting process) and restart.
