# Rhelma — Multi-Frontend Architecture

**Status:** Draft  
**Owner:** Platform / Runtime  
**Last Updated:** 2026-01-03

## Summary

Rhelma uses a **multi-frontend** pattern to keep sensitive operator/admin surfaces in **Rust** (tight
control, minimal dependencies) while serving the public/low-risk UI from the web frontend.
The `multi-frontend` service is the entrypoint that multiplexes these surfaces under one origin.

## Purpose

- Reduce blast radius for admin/governance surfaces.
- Keep critical UX paths available even if the web toolchain is broken.
- Provide a single origin for browsers while enforcing stricter caching and security rules for
  `/admin/*` routes.

## Scope

This document covers:

- Routing and surface separation (`/admin` vs `/`)
- Dev/prod wiring expectations
- Security and caching defaults for admin pages

It does **not** define UI product requirements; those live with each app/team.

## Definitions

- **Admin Surface:** Operator-only pages (policy, governance, keys, incident response).
- **Web Surface:** General dashboards, non-sensitive tools, and product UX.
- **Entry Service:** A single HTTP service that serves/proxies both surfaces.

## Requirements

1. **Admin surfaces MUST be safe-by-default**
   - Require auth (token and/or mTLS) in production.
   - Prefer read-only defaults; write actions should be explicit and auditable.

2. **Admin pages MUST set private caching**
   - Avoid shared caches.
   - Allow short-lived caching to prevent refetch storms.

3. **Web surface MUST be replaceable**
   - Dev: redirect to the web dev server.
   - Prod: serve built static output.

4. **Proxy paths MUST be allowlisted**
   - No open proxy.

## Implementation Notes

### Service: `apps/multi-frontend`

At runtime:

- `/admin/*` is served from a Rust UI (via the `frontpage` crate).
- `/` serves the web build directory if present; otherwise redirects to `RHELMA_WEB_DEV_URL`.
- `/api/*` routes may be reverse proxied to upstreams with explicit allowlists.

The repo includes a launcher script:

- `scripts/run-multi-frontend.sh`

### Recommended local flow

1. Start the web dev server (`apps/web`).
2. Run `multi-frontend` from repo root.

### Production flow

1. Build the web app to a static output directory.
2. Run `multi-frontend` and serve the build directly.

## Decisions (ADR-style)

- **Decision:** Keep `/admin` served by Rust (minimal dependencies).
  - **Alternatives:** Serve everything from the web app; use a reverse proxy split.
  - **Rationale:** reduces supply-chain/runtime risk for privileged surfaces; keeps an operator UI
    available during web/toolchain outages.
  - **Consequences:** duplication of some UI components; requires clear boundary rules.

## Operational Impact

- Admin routes should emit stricter audit events.
- Observability: tag requests with `surface=admin|web` (or equivalent) to keep dashboards clear.
- Failure modes:
  - web build missing → redirect to dev server (dev only)
  - upstream proxy unavailable → admin status pages degrade gracefully

## Acceptance Criteria

- `/admin` works without the web build.
- `/` serves a web build if present.
- Proxy routes are allowlisted.
- Admin routes set private Cache-Control.

## Risks / Open Questions

- Where to enforce auth: edge proxy vs app middleware vs both?
- How to standardize cross-surface navigation without coupling UI stacks too tightly?

## References

- `apps/multi-frontend/README.md`
- `scripts/dev/run-multi-frontend.sh`
- `docs/architecture/COMMUNICATIONS.md`
