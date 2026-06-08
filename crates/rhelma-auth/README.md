# rhelma-auth

Rhelma Auth (Enterprise): JWT(EdDSA) + Redis Sessions + RBAC + Tower Middleware + OIDC traits + Eventing

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
| `RHELMA_AUTH_ISSUER` | JWT issuer (defaults to `<service>-auth`) |
| `RHELMA_AUTH_AUDIENCE` | JWT audience (defaults to `<service>`) |
| `RHELMA_AUTH_JWT_PRIVATE_KEY_B64` | Ed25519 private key (base64 DER) |
| `RHELMA_AUTH_JWT_PUBLIC_KEY_B64` | Ed25519 public key (base64 DER) |
| `RHELMA_AUTH_REDIS_URL` | Redis connection URL for distributed sessions |
| `RHELMA_AUTH_REDIS_PREFIX` | Redis key namespace prefix |
| `RHELMA_AUTH_ACCESS_TTL_SECS` | Access token TTL in seconds (<= 900) |
| `RHELMA_AUTH_REFRESH_TTL_SECS` | Refresh token TTL in seconds (<= 604800) |
| `RHELMA_AUTH_SESSION_IDLE_SECS` | Session idle timeout (<= 1800) |
| `RHELMA_AUTH_SESSION_TOUCH_SECS` | Throttle interval for session "touch" writes (<= 300) |
| `RHELMA_AUTH_SESSION_ABS_SECS` | Session absolute timeout (<= 28800) |
| `RHELMA_AUTH_COOKIE_SECURE` | Cookie secure flag (default: true outside development) |
| `RHELMA_AUTH_COOKIE_SAME_SITE` | Cookie SameSite policy (default: Lax) |
| `RHELMA_AUTH_PASSWORD_HASH_COST` | Argon2 cost factor (default: 12) |
| `RHELMA_AUTH_RATE_LIMIT_REQUESTS` | Baseline requests per window (default: 60) |
| `RHELMA_AUTH_RATE_LIMIT_WINDOW_SECS` | Rate limit window seconds (default: 60) |

## Security & Compliance

Normative requirements are in `docs/contract/v6.0/00_INDEX_v6.0.md`.
