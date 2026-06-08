# rhelma.ir Site Structure

`rhelma.ir` is the official reference site for the Rhelma platform. It should explain the system, document the public core, and create a clear path from open-source adoption to commercial services.

## Primary Navigation

- Home
- Docs
- Open Source
- Asrnegar Demo
- Commercial
- Security
- Roadmap

## Page Map

```text
/
  Platform overview
  Public core summary
  Link to quick start
  Link to Asrnegar live demo

/docs
  Getting started
  Architecture
  Contracts
  API reference
  Operations
  Testing

/open-source
  What is included
  License
  Repository links
  Contribution guide
  Release checklist

/demo
  Asrnegar introduction
  Demo status
  Public API examples
  Known limits

/commercial
  Hosted operations
  Enterprise modules
  Private integrations
  Support model

/security
  Security model
  Vulnerability reporting
  Supported versions
  Disclosure process

/roadmap
  Public roadmap
  Experimental features
  Recently shipped changes
```

## Content Rules

- The first screen should state that Rhelma is the platform and Asrnegar is the operational social demo.
- Do not make the homepage a marketing-only page; include a direct route to docs and the demo.
- Keep public and commercial boundaries explicit.
- Use the same terminology as repository docs: core, services, contracts, realms, observability, and zero-trust.
- Never publish production secrets, private customer information, or real infrastructure identifiers.

## Suggested Homepage Copy

Rhelma is a Rust-based platform for building secure, observable, multi-service systems. The public core provides reusable crates, service contracts, development tooling, and a working social demo through Asrnegar. Commercial deployments add hosted operations, private integrations, and enterprise administration.
