# Workspace Identity Model

_Finalized in Stage 13.5. Normative requirements live in the contract
(`docs/contract/v6.0/00_INDEX_v6.0.md`); this document is developer-facing._

## The one rule

```
User != Workspace != Tenant
```

These are three **distinct** identities, backed by three distinct Rust newtypes
in `crates/rhelma-core/src/types/ids.rs`:

| Type          | Backing     | Meaning                                             |
| ------------- | ----------- | --------------------------------------------------- |
| `UserId`      | `Uuid`      | The human principal (login identity).               |
| `WorkspaceId` | `Uuid`      | The **product** identity — the user's environment.  |
| `TenantId`    | `String`    | The **technical isolation boundary** only.          |

## Ownership hierarchy

```
User
 └── owns Workspace
        ├── Services   (social, profile, wallet, commerce, agent, …)
        ├── Agent      (workspace-scoped execution context)
        ├── Members    (owner / admin / member / viewer)
        └── Policies   (visibility, access, ai_policy, agent_policy, license)
```

- **User owns Workspace.** `workspaces.owner_user_id` is `NOT NULL` with an FK to
  `users`; a workspace cannot exist without an owner.
- **Workspace owns Services.** `workspace_services(workspace_id, service_name,
  enabled, config)` records which services a workspace has turned on. Services
  are neither user-owned nor tenant-owned.
- **Tenant provides isolation only.** `workspaces.tenant_id` is `UNIQUE` — a
  workspace maps to exactly one tenant. Service data (`social_*`,
  `user_profiles`, `wallets`, `agent_*`) is `tenant_id`-scoped under FORCE
  row-level security. The tenant is **never** exposed as the product identity.

## Resolution order (workspace first, tenant derived)

`crates/rhelma-db/src/workspace.rs::WorkspaceResolver` resolves in this order:

1. workspace **slug** (`x-rhelma-workspace-slug`)
2. verified **custom domain**
3. workspace slug from the **request path** (`/workspace/{slug}/…`)
4. request **host**

It then enforces visibility (a `private` workspace requires the owner or an
`active` member) and only **afterwards** derives `tenant_id` from the resolved
workspace. Callers never pass a tenant id as the product selector.

`ResolvedWorkspaceContext` carries: `workspace_id, owner_user_id, tenant_id,
visibility, access_policy, ai_policy, agent_policy, license_id,
enabled_services`.

## Service ownership

| Service   | Owned by      | Notes                                                     |
| --------- | ------------- | -------------------------------------------------------- |
| Agent     | **Workspace** | `AgentExecutionContext` binds user + workspace + tenant + roles + permissions + entitlements + `enabled_services`. Never operates on `user_id + tenant_id` alone. |
| Social    | **Workspace** | Workspace owns the social space; a user creates content inside it. Rows are tenant-scoped today (1 tenant = 1 workspace). |
| Profile   | **Workspace** | Separate concerns: a **User Profile** (identity) vs. a **Workspace Public Profile** (branding). Do not mix. |
| Wallet    | **Workspace** (by design) | Modeled workspace-owned, not user-owned. Execution wiring is Stage 14; not implemented here. |

## Edge routing & tenant compatibility

The public edge (`apps/multi-frontend`) currently addresses workspaces as
`/workspace/{tenant}/…` and pins the path segment authoritatively as
`x-tenant-id`. Because tenant↔workspace is 1:1 today, the tenant slug doubles as
the workspace selector. The backend still resolves **workspace-first**, so this
is a compatibility surface, not a domain-model compromise. A future change can
move the edge to `/workspace/{workspace_slug}/…` and derive the tenant purely
server-side; the resolver already supports slug-based resolution.

## Invariants (enforced by tests)

- Every resolved workspace has an owner (`rhelma-db`:
  `every_resolved_workspace_has_an_owner`).
- A workspace maps to exactly one tenant (`map_workspace_to_tenant`,
  `UNIQUE(tenant_id)`).
- A tenant id cannot be used as a workspace selector
  (`tenant_id_cannot_be_used_as_a_workspace_selector`).
- `WorkspaceId` is distinct from `TenantId`
  (`workspace_id_is_distinct_from_tenant_id`).
- A user cannot access another user's private workspace
  (`reject_private_workspace_for_non_member`); only `active` members resolve a
  role (`member_role_is_resolved_for_active_members_only`).
- The agent always carries workspace scope + enabled services
  (`agent-service`: `agent_context_carries_workspace_scope_not_just_user_and_tenant`,
  `service_ownership_is_workspace_scoped_for_all_services`).
