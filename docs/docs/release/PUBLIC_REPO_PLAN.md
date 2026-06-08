# Public Repository Plan

## Target Repository

`rhelma-open`

## Include

- Public crates listed in `OPEN_SOURCE_MANIFEST.md`
- Public services listed in `OPEN_SOURCE_MANIFEST.md`
- Public SDKs
- Public docs
- Local development and smoke-test scripts
- Example infrastructure only

## Exclude

- Commercial paths listed in `COMMERCIAL_BOUNDARY.md`
- Production deployment configuration
- Secrets and private environment files
- Customer-specific docs, scripts, and integrations

## First Publish Shape

```text
rhelma-open/
  apps/
    api-gateway/
    social-service/
    search-service/
    realtime-service/
    file-storage/
    node-registry/
  crates/
  docs/
  rhelma-sdk-js/
  rhelma-sdk-python/
  rhelma-sdk-go/
  scripts/
  docker-compose.dev.yml
  .env.example
  .env.public.example
  Cargo.toml
  README.md
  ROADMAP.md
```

## Work Items

- Create a clean public branch or export directory.
- Reduce workspace members to public paths only.
- Run build, tests, clippy, docs, and secret scan.
- Publish with `MIT OR Apache-2.0` license files.
- Add repository topics and public description.

## Helper Script

Dry run:

```powershell
.\scripts\prepare_public_release.ps1
```

Export after review:

```powershell
.\scripts\prepare_public_release.ps1 -Copy -OutputDir D:\project\release\rhelma-open
```
