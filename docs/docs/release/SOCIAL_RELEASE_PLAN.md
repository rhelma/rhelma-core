# Social Release Plan

## Target Product

Asrnegar social product on `https://asrnegar.ir`.

## Include

- `apps/social-service`
- `apps/api-gateway`
- `apps/search-service`
- `apps/realtime-service`
- `apps/file-storage`
- `web/asrnegar.ir`
- Social API docs
- Synthetic seed data
- Moderation and reporting docs

## Public Website

Current static website source:

```text
D:\project\web\asrnegar.ir
```

## Launch Phases

1. Static public site.
2. Read-only social feed backed by synthetic data.
3. Registration with rate limits.
4. Posting, comments, follows, and search.
5. Realtime updates.
6. Moderation operations.
7. Public status and changelog.

## Work Items

- Connect static UI to gateway routes.
- Add social seed data.
- Add smoke tests for feed, profile, post, comment, follow, search, and report.
- Add moderation role checks.
- Add production deploy configuration in a private operations repository.
