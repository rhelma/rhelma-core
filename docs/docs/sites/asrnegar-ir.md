# asrnegar.ir Demo Structure

`asrnegar.ir` is the operational public demo for the Rhelma platform. It should behave like a real social product while making it clear that it is powered by the public Rhelma core.

## Product Scope

The first public version should focus on a small, stable social workflow:

- User registration and login
- Public profiles
- Posts
- Comments
- Follows
- Search
- Basic realtime updates
- Moderation and reporting
- Public API examples

## Site Map

```text
/
  Social feed
  Sign in / register
  Public platform note linking to rhelma.ir

/u/:handle
  Public user profile
  User posts
  Follow controls

/post/:id
  Post detail
  Comments
  Shareable public URL

/search
  People and post search

/settings
  Account settings
  Privacy settings
  Session controls

/moderation
  Reports queue
  Basic admin controls

/status
  Demo uptime
  Known limits
  Current version

/api
  Public API examples
  Link to OpenAPI documentation
```

## Backend Mapping

- `apps/social-service`: social domain API.
- `apps/api-gateway`: public gateway and auth boundary.
- `apps/search-service`: people and post search.
- `apps/realtime-service`: live feed and notification events.
- `apps/file-storage`: avatars and post attachments.
- `crates/rhelma-auth`: authentication and authorization primitives.
- `crates/rhelma-core`: shared identifiers, tenant/realm types, and contracts.
- `crates/rhelma-metrics` and `crates/rhelma-tracing`: observability.

## Demo Safety Rules

- Use synthetic seed data for public screenshots and examples.
- Rate-limit registration, posting, commenting, search, and file upload.
- Make moderation and abuse reporting part of the first release, not a later add-on.
- Keep admin-only routes protected and excluded from public docs unless intentionally exposed.
- Display demo status without exposing internal infrastructure details.

## Launch Checklist

- Confirm the demo runs using public modules only.
- Confirm all environment values come from safe examples.
- Confirm seed data has no real private user information.
- Confirm public OpenAPI docs match deployed routes.
- Confirm links back to `rhelma.ir` explain the platform relationship.
